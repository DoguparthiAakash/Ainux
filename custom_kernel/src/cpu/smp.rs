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

#[repr(C, align(64))]
#[derive(Debug)]
pub struct PerCpuState {
    pub lapic_id: u32,
    pub online: bool,
    pub idle_count: u64,
    pub current_task_id: usize,
    pub tss_ptr: *mut crate::cpu::gdt::Tss,
}

impl PerCpuState {
    pub const fn new() -> Self {
        PerCpuState {
            lapic_id: 0,
            online: false,
            idle_count: 0,
            current_task_id: 0,
            tss_ptr: core::ptr::null_mut(),
        }
    }
}

// Wrapper to allow Send/Sync for raw pointers in Mutex
#[derive(Copy, Clone)]
pub struct SendPtr(pub *mut PerCpuState);
unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

pub const MAX_CPUS: usize = 64;

pub static CPU_COUNT: AtomicU32 = AtomicU32::new(1); // BSP = 1
pub static AP_READY: AtomicBool = AtomicBool::new(false);
static AP_BOOTED: AtomicU32 = AtomicU32::new(0);

pub static PER_CPU: Mutex<[Option<SendPtr>; MAX_CPUS]> = Mutex::new([None; MAX_CPUS]);

// Trampoline code address (must be below 1MB, page-aligned)
const TRAMPOLINE_ADDR: usize = 0x8000;
// Offsets within the trampoline binary for patched data
const TRAMPOLINE_CR3_OFFSET: usize = 0xFF0;     // 8 bytes: BSP's CR3
const TRAMPOLINE_ENTRY_OFFSET: usize = 0xFF8;   // 8 bytes: ap_entry address

/// NASM-assembled trampoline binary (4096 bytes)
static TRAMPOLINE_BIN: &[u8] = include_bytes!("../asm/ap_trampoline.bin");

// ---- Trampoline Installation ----

/// Copy the NASM trampoline binary to physical address 0x8000.
/// Called once before the AP boot loop.
fn install_trampoline() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "SMP: Installing AP trampoline at {:#x} ({} bytes)\n",
        TRAMPOLINE_ADDR, TRAMPOLINE_BIN.len());

    unsafe {
        let dest = TRAMPOLINE_ADDR as *mut u8;
        core::ptr::copy_nonoverlapping(TRAMPOLINE_BIN.as_ptr(), dest, TRAMPOLINE_BIN.len());
    }
}

/// Patch the trampoline data area with CR3 and entry point.
/// MUST be called before each SIPI because the AP's stack may overwrite
/// the data area at 0x8FF0 during execution.
fn patch_trampoline_data() {
    let cr3: u64;
    unsafe { asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack)); }
    let ap_entry_addr = ap_entry as *const () as u64;

    unsafe {
        let cr3_ptr = (TRAMPOLINE_ADDR + TRAMPOLINE_CR3_OFFSET) as *mut u64;
        let entry_ptr = (TRAMPOLINE_ADDR + TRAMPOLINE_ENTRY_OFFSET) as *mut u64;
        core::ptr::write_volatile(cr3_ptr, cr3);
        core::ptr::write_volatile(entry_ptr, ap_entry_addr);
    }
}

// ---- AP Entry Point (called by trampoline in 64-bit mode) ----

#[no_mangle]
pub extern "C" fn ap_entry() -> ! {
    // We're now in 64-bit mode on an AP core.
    // Stack is at 0x7C00 (from trampoline), growing down into safe memory.
    // Interrupts are disabled (cli in trampoline).
    let cpu_num = AP_BOOTED.fetch_add(1, Ordering::SeqCst) + 1;
    CPU_COUNT.fetch_add(1, Ordering::SeqCst);

    // Enable this core's Local APIC via direct MMIO (no locks, no ACPI).
    // The LAPIC base address was already discovered by the BSP.
    unsafe {
        let lapic_base = crate::cpu::apic::LAPIC_BASE.load(Ordering::Relaxed);

        // Read SVR and enable APIC + set spurious vector
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

    // Load the BSP's GDT (without touching TSS/LTR to avoid GP fault).
    // The trampoline's GDT is in the 0x8000 page which may be overwritten
    // when the next AP boots, so we must switch to the kernel's permanent GDT.
    crate::cpu::gdt::init_ap();

    // Load the shared IDT so fault handlers work on this core.
    crate::cpu::idt::init();

    // Signal that this AP is online.
    // The BSP is polling AP_READY and will not send the next SIPI
    // until it sees this flag, giving us a clean handoff.
    AP_READY.store(true, Ordering::SeqCst);

    // AP idle loop — interrupts disabled, just park.
    // When the scheduler supports multi-core dispatch, it will
    // send an IPI to wake this core and enable interrupts.
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
    // Drop the ACPI lock before doing anything else — APs must never contend on it.
    drop(acpi_state);

    if ap_count == 0 {
        let _ = write!(serial, "SMP: No APs to boot\n");
        return;
    }

    // Install trampoline binary once
    install_trampoline();

    let cr3: u64;
    unsafe { asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack)); }
    let ap_entry_addr = ap_entry as *const () as u64;
    let _ = write!(serial, "SMP: CR3={:#x}, ap_entry={:#x}\n", cr3, ap_entry_addr);

    // Boot each AP sequentially (wait for each before starting the next)
    for i in 0..ap_count {
        let target_id = ap_ids[i];
        let _ = write!(serial, "SMP: Booting AP (APIC ID {})...\n", target_id);

        AP_READY.store(false, Ordering::SeqCst);

        // Re-patch trampoline data before EVERY SIPI.
        // The previous AP's stack may have overwritten the data area.
        patch_trampoline_data();

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

            // SIPI #2 (some CPUs need a second SIPI per Intel MP spec)
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

pub fn get_current_task_id() -> usize {
    let id: usize;
    unsafe {
        asm!(
            "mov {0:r}, gs:[16]", // current_task_id is at offset 16 for repr(C)
            out(reg) id,
            options(nostack, nomem, preserves_flags)
        );
    }
    id
}

pub fn set_current_task_id(id: usize) {
    unsafe {
        asm!(
            "mov gs:[16], {0:r}",
            in(reg) id,
            options(nostack, nomem, preserves_flags)
        );
    }
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
