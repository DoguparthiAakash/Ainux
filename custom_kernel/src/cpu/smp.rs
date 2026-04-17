// =============================================================================
// Ainux SMP — Symmetric Multi-Processing
// Bootstraps Application Processors (APs) using INIT-SIPI-SIPI sequence.
// Uses NASM-assembled trampoline (ap_trampoline.bin) for correct 16→32→64-bit
// mode transitions.
// =============================================================================

use core::arch::asm;
use core::fmt::Write;
use core::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use spin::Mutex;

// ---- Per-CPU State ----

#[derive(Debug)]
pub struct PerCpuState {
    pub cpu_id: u32,
    pub apic_id: u8,
    pub online: bool,
    pub idle_count: u64,
}

impl PerCpuState {
    pub const fn new() -> Self {
        PerCpuState {
            cpu_id: 0,
            apic_id: 0,
            online: false,
            idle_count: 0,
        }
    }
}

// Support up to 64 CPUs
pub const MAX_CPUS: usize = 64;

pub static CPU_COUNT: AtomicU32 = AtomicU32::new(1); // BSP = 1
pub static AP_READY: AtomicBool = AtomicBool::new(false);
static AP_BOOTED: AtomicU32 = AtomicU32::new(0);

pub static PER_CPU: Mutex<[PerCpuState; MAX_CPUS]> = Mutex::new(
    [const { PerCpuState::new() }; MAX_CPUS]
);

// Trampoline code address (must be below 1MB, page-aligned)
const TRAMPOLINE_ADDR: usize = 0x8000;
// Offsets within the trampoline binary for patched data
const TRAMPOLINE_CR3_OFFSET: usize = 0xFF0;     // 8 bytes: BSP's CR3
const TRAMPOLINE_ENTRY_OFFSET: usize = 0xFF8;   // 8 bytes: ap_entry address

/// NASM-assembled trampoline binary (4096 bytes)
static TRAMPOLINE_BIN: &[u8] = include_bytes!("../asm/ap_trampoline.bin");

// ---- Install Trampoline ----

fn install_trampoline() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "SMP: Installing AP trampoline at {:#x} ({} bytes)\n",
        TRAMPOLINE_ADDR, TRAMPOLINE_BIN.len());

    // Get the BSP's CR3 (PML4 physical address)
    let cr3: u64;
    unsafe { asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack)); }

    // Get the address of ap_entry function
    let ap_entry_addr = ap_entry as *const () as u64;

    let _ = write!(serial, "SMP: CR3={:#x}, ap_entry={:#x}\n", cr3, ap_entry_addr);

    unsafe {
        let dest = TRAMPOLINE_ADDR as *mut u8;

        // Copy the NASM-assembled trampoline binary to 0x8000
        core::ptr::copy_nonoverlapping(TRAMPOLINE_BIN.as_ptr(), dest, TRAMPOLINE_BIN.len());

        // Patch the data area with runtime values
        let cr3_ptr = (TRAMPOLINE_ADDR + TRAMPOLINE_CR3_OFFSET) as *mut u64;
        let entry_ptr = (TRAMPOLINE_ADDR + TRAMPOLINE_ENTRY_OFFSET) as *mut u64;

        core::ptr::write_volatile(cr3_ptr, cr3);
        core::ptr::write_volatile(entry_ptr, ap_entry_addr);

        // Verify the patches
        let check_cr3 = core::ptr::read_volatile(cr3_ptr);
        let check_entry = core::ptr::read_volatile(entry_ptr);
        let _ = write!(serial, "SMP: Patched CR3={:#x}, entry={:#x}\n", check_cr3, check_entry);
    }
}

// ---- AP Entry Point (called by trampoline in 64-bit mode) ----

#[no_mangle]
pub extern "C" fn ap_entry() -> ! {
    // We're now in 64-bit mode on an AP core!
    let cpu_num = AP_BOOTED.fetch_add(1, Ordering::SeqCst) + 1;
    CPU_COUNT.fetch_add(1, Ordering::SeqCst);

    // Enable the local APIC on this core
    unsafe {
        let lapic_base = crate::cpu::apic::LAPIC_BASE.load(Ordering::Relaxed);

        // Read SVR and enable APIC
        let svr = core::ptr::read_volatile((lapic_base + 0xF0) as *const u32);
        core::ptr::write_volatile(
            (lapic_base + 0xF0) as *mut u32,
            svr | 0x100 | 0xFF
        );

        // Set TPR to 0 (accept all interrupts)
        core::ptr::write_volatile(
            (lapic_base + 0x80) as *mut u32,
            0
        );
    }

    // Get this CPU's APIC ID
    let apic_id = crate::cpu::apic::current_cpu_id();

    // Register in per-CPU state
    {
        let mut per_cpu = PER_CPU.lock();
        if (cpu_num as usize) < MAX_CPUS {
            per_cpu[cpu_num as usize] = PerCpuState {
                cpu_id: cpu_num,
                apic_id: apic_id as u8,
                online: true,
                idle_count: 0,
            };
        }
    }

    // Signal that this AP is online
    AP_READY.store(true, Ordering::SeqCst);

    // AP idle loop — wait for scheduler to assign work
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
}

// ---- SMP Init (BSP orchestrator) ----

/// Boot all Application Processors detected by ACPI
pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "\n========== SMP INITIALIZATION ==========\n");

    let acpi_state = crate::cpu::acpi::ACPI.lock();
    let cpu_count = acpi_state.cpu_count;

    if cpu_count <= 1 {
        let _ = write!(serial, "SMP: Single CPU detected. SMP disabled.\n");
        // Still set BSP info
        let mut per_cpu = PER_CPU.lock();
        per_cpu[0] = PerCpuState {
            cpu_id: 0,
            apic_id: if cpu_count > 0 { acpi_state.cpus[0].apic_id } else { 0 },
            online: true,
            idle_count: 0,
        };
        return;
    }

    let bsp_apic_id = acpi_state.cpus[0].apic_id;
    let _ = write!(serial, "SMP: BSP APIC ID: {}, Total CPUs: {}\n", bsp_apic_id, cpu_count);

    // Collect AP APIC IDs
    let mut ap_ids: [u8; MAX_CPUS] = [0; MAX_CPUS];
    let mut ap_count = 0;
    for i in 0..cpu_count {
        if acpi_state.cpus[i].apic_id != bsp_apic_id && acpi_state.cpus[i].enabled {
            ap_ids[ap_count] = acpi_state.cpus[i].apic_id;
            ap_count += 1;
        }
    }
    drop(acpi_state);

    // Set BSP per-CPU state
    {
        let mut per_cpu = PER_CPU.lock();
        per_cpu[0] = PerCpuState {
            cpu_id: 0,
            apic_id: bsp_apic_id,
            online: true,
            idle_count: 0,
        };
    }

    if ap_count == 0 {
        let _ = write!(serial, "SMP: No APs to boot\n");
        return;
    }

    // Install trampoline code
    install_trampoline();

    // Boot each AP
    for i in 0..ap_count {
        let target_id = ap_ids[i];
        let _ = write!(serial, "SMP: Booting AP (APIC ID {})...\n", target_id);

        AP_READY.store(false, Ordering::SeqCst);

        unsafe {
            // INIT IPI
            crate::cpu::apic::send_init(target_id);

            // 10ms delay
            delay_us(10_000);

            // SIPI #1 — trampoline page = TRAMPOLINE_ADDR >> 12
            let trampoline_page = (TRAMPOLINE_ADDR >> 12) as u8;
            crate::cpu::apic::send_sipi(target_id, trampoline_page);

            // 200µs delay
            delay_us(200);

            // SIPI #2 (some CPUs need a second SIPI)
            crate::cpu::apic::send_sipi(target_id, trampoline_page);
        }

        // Wait for AP to come online (timeout ~500ms)
        let mut booted = false;
        for _ in 0..50_000_000 {
            if AP_READY.load(Ordering::SeqCst) {
                booted = true;
                break;
            }
            unsafe { asm!("pause", options(nomem, nostack)); }
        }

        if booted {
            let _ = write!(serial, "SMP: AP {} online ✓\n", target_id);
        } else {
            let _ = write!(serial, "SMP: AP {} failed to boot ✗\n", target_id);
        }
    }

    let total = CPU_COUNT.load(Ordering::SeqCst);
    let _ = write!(serial, "SMP: {} CPUs online\n", total);
    let _ = write!(serial, "========================================\n\n");
}

/// Rough microsecond delay using PAUSE instruction
#[inline(always)]
unsafe fn delay_us(us: u64) {
    // Each PAUSE is ~10-140 cycles depending on CPU
    // At ~1GHz, ~100 pauses per microsecond is a rough estimate
    let iterations = us * 100;
    for _ in 0..iterations {
        asm!("pause", options(nomem, nostack));
    }
}
