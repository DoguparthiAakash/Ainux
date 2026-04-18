// =============================================================================
// Ainux Local APIC + I/O APIC Driver
// Replaces legacy 8259 PIC for multi-core interrupt routing
// =============================================================================

use core::arch::asm;
use core::fmt::Write;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

// ---- Local APIC Register Offsets (MMIO) ----

const LAPIC_ID: u32         = 0x020;
const LAPIC_VERSION: u32    = 0x030;
const LAPIC_TPR: u32        = 0x080;  // Task Priority
const LAPIC_EOI: u32        = 0x0B0;  // End of Interrupt
const LAPIC_SVR: u32        = 0x0F0;  // Spurious Interrupt Vector
const LAPIC_ICR_LO: u32     = 0x300;  // Interrupt Command (low)
const LAPIC_ICR_HI: u32     = 0x310;  // Interrupt Command (high)
const LAPIC_TIMER_LVT: u32  = 0x320;  // Timer LVT
const LAPIC_TIMER_INIT: u32 = 0x380;  // Timer Initial Count
const LAPIC_TIMER_CUR: u32  = 0x390;  // Timer Current Count
const LAPIC_TIMER_DIV: u32  = 0x3E0;  // Timer Divide Config

// SVR flags
const SVR_ENABLE: u32 = 0x100;
const SVR_VECTOR: u32 = 0xFF; // Spurious vector = 0xFF

// ICR delivery modes
const ICR_INIT: u32     = 0x00000500;
const ICR_STARTUP: u32  = 0x00000600;
const ICR_LEVEL_ASSERT: u32 = 0x00004000;
const ICR_LEVEL_DEASSERT: u32 = 0x00000000;

// Timer modes
const TIMER_PERIODIC: u32 = 0x20000;
const TIMER_VECTOR: u32 = 32; // IRQ 0 → vector 32

// ---- I/O APIC ----

const IOAPIC_REGSEL: u32 = 0x00;
const IOAPIC_WIN: u32    = 0x10;
const IOAPIC_ID: u32     = 0x00;
const IOAPIC_VER: u32    = 0x01;
const IOAPIC_REDTBL: u32 = 0x10;

// ---- State ----

pub static LAPIC_BASE: AtomicU64 = AtomicU64::new(0xFEE00000);
pub static IOAPIC_BASE: AtomicU64 = AtomicU64::new(0);
pub static APIC_INITIALIZED: AtomicBool = AtomicBool::new(false);

// ---- MMIO Read/Write ----

#[inline(always)]
unsafe fn lapic_read(reg: u32) -> u32 {
    let base = LAPIC_BASE.load(Ordering::Relaxed) as *const u32;
    let ptr = (base as usize + reg as usize) as *const u32;
    core::ptr::read_volatile(ptr)
}

#[inline(always)]
unsafe fn lapic_write(reg: u32, val: u32) {
    let base = LAPIC_BASE.load(Ordering::Relaxed) as *mut u32;
    let ptr = (base as usize + reg as usize) as *mut u32;
    core::ptr::write_volatile(ptr, val);
}

#[inline(always)]
unsafe fn ioapic_read(reg: u32) -> u32 {
    let base = IOAPIC_BASE.load(Ordering::Relaxed) as *mut u32;
    core::ptr::write_volatile(base, reg);
    core::ptr::read_volatile((base as usize + IOAPIC_WIN as usize) as *const u32)
}

#[inline(always)]
unsafe fn ioapic_write(reg: u32, val: u32) {
    let base = IOAPIC_BASE.load(Ordering::Relaxed) as *mut u32;
    core::ptr::write_volatile(base, reg);
    core::ptr::write_volatile((base as usize + IOAPIC_WIN as usize) as *mut u32, val);
}

// ---- Public API ----

/// Initialize the BSP's Local APIC
pub fn init_lapic() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);

    let acpi_state = crate::cpu::acpi::ACPI.lock();
    let lapic_addr = acpi_state.local_apic_addr;
    let io_apic_addr = acpi_state.io_apic_addr;
    drop(acpi_state);

    LAPIC_BASE.store(lapic_addr, Ordering::Relaxed);
    if io_apic_addr != 0 {
        IOAPIC_BASE.store(io_apic_addr, Ordering::Relaxed);
    }

    let _ = write!(serial, "APIC: Initializing LAPIC at {:#x}\n", lapic_addr);

    unsafe {
        // Enable the Local APIC via SVR
        let svr = lapic_read(LAPIC_SVR);
        lapic_write(LAPIC_SVR, svr | SVR_ENABLE | SVR_VECTOR);

        // Set Task Priority to 0 (accept all interrupts)
        lapic_write(LAPIC_TPR, 0);

        let id = lapic_read(LAPIC_ID) >> 24;
        let ver = lapic_read(LAPIC_VERSION);
        let _ = write!(serial, "APIC: LAPIC ID={} Version={:#x}\n", id, ver);
    }

    APIC_INITIALIZED.store(true, Ordering::Relaxed);
    let _ = write!(serial, "APIC: Local APIC enabled\n");
}

/// Initialize the I/O APIC and route legacy IRQs
pub fn init_ioapic() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let base = IOAPIC_BASE.load(Ordering::Relaxed);

    if base == 0 {
        let _ = write!(serial, "APIC: No I/O APIC found, using legacy PIC\n");
        return;
    }

    let _ = write!(serial, "APIC: Initializing I/O APIC at {:#x}\n", base);

    unsafe {
        let ver = ioapic_read(IOAPIC_VER);
        let max_entries = ((ver >> 16) & 0xFF) + 1;
        let _ = write!(serial, "APIC: I/O APIC max redirection entries: {}\n", max_entries);

        // Route keyboard (IRQ 1) to vector 33, BSP LAPIC ID 0
        ioapic_route_irq(1, 33, 0);
        // Route timer (IRQ 0) to vector 32
        ioapic_route_irq(0, 32, 0);
        // Route mouse (IRQ 12) to vector 44
        ioapic_route_irq(12, 44, 0);
        // Route ATA (IRQ 14) to vector 46
        ioapic_route_irq(14, 46, 0);

        let _ = write!(serial, "APIC: I/O APIC IRQ routing configured\n");
    }
}

/// Route an IRQ pin to a specific vector on a specific LAPIC
unsafe fn ioapic_route_irq(irq: u8, vector: u8, lapic_id: u8) {
    let reg_lo = IOAPIC_REDTBL + (irq as u32) * 2;
    let reg_hi = reg_lo + 1;

    // Low 32 bits: vector, delivery mode fixed (0), active low (0), level (0)
    let lo: u32 = vector as u32; // Fixed delivery, edge-triggered, active-high
    // High 32 bits: destination APIC ID
    let hi: u32 = (lapic_id as u32) << 24;

    ioapic_write(reg_hi, hi);
    ioapic_write(reg_lo, lo);
}

/// Send End-of-Interrupt to Local APIC
#[inline(always)]
pub unsafe fn eoi() {
    if APIC_INITIALIZED.load(Ordering::Relaxed) {
        lapic_write(LAPIC_EOI, 0);
    }
}

/// Get the current CPU's LAPIC ID
pub fn current_cpu_id() -> u32 {
    if !APIC_INITIALIZED.load(Ordering::Relaxed) {
        return 0;
    }
    unsafe { lapic_read(LAPIC_ID) >> 24 }
}

/// Calibrated LAPIC timer ticks per millisecond (set by init_timer)
pub static LAPIC_TICKS_PER_MS: AtomicU64 = AtomicU64::new(0);

/// Configure the LAPIC timer for periodic interrupts, calibrated via PIT
pub fn init_timer(frequency_hz: u32) {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "APIC: Calibrating LAPIC timer via PIT...\n");

    unsafe {
        // ---- PIT-based calibration ----
        // Use PIT Channel 2 in one-shot mode as a reference clock.
        // PIT base frequency = 1,193,182 Hz
        // We measure how many LAPIC ticks elapse in ~10ms.

        const PIT_FREQ: u32 = 1_193_182;
        const CALIBRATE_MS: u32 = 10;
        let pit_count: u16 = (PIT_FREQ / (1000 / CALIBRATE_MS)) as u16;

        // Set LAPIC timer divider to 16
        lapic_write(LAPIC_TIMER_DIV, 0x03);

        // Start LAPIC timer with max initial count (one-shot, masked)
        lapic_write(LAPIC_TIMER_LVT, 0x10000); // Masked one-shot
        lapic_write(LAPIC_TIMER_INIT, 0xFFFFFFFF);

        // Program PIT Channel 2 for one-shot (mode 0), lo/hi byte
        // Port 0x61: bits [0] = gate, [1] = speaker
        let gate: u8;
        asm!("in al, 0x61", out("al") gate, options(nomem, nostack, preserves_flags));
        // Enable gate (bit 0), disable speaker (bit 1)
        let gate_val = (gate & 0xFC) | 0x01;
        asm!("out 0x61, al", in("al") gate_val, options(nomem, nostack, preserves_flags));

        // PIT Channel 2, mode 0 (one-shot), lobyte/hibyte
        asm!("out dx, al", in("dx") 0x43u16, in("al") 0xB0u8, options(nomem, nostack, preserves_flags));

        // Write count (lo then hi)
        let lo = (pit_count & 0xFF) as u8;
        let hi = (pit_count >> 8) as u8;
        asm!("out dx, al", in("dx") 0x42u16, in("al") lo, options(nomem, nostack, preserves_flags));
        asm!("out dx, al", in("dx") 0x42u16, in("al") hi, options(nomem, nostack, preserves_flags));

        // Wait for PIT Channel 2 output to go high (bit 5 of port 0x61)
        loop {
            let status: u8;
            asm!("in al, 0x61", out("al") status, options(nomem, nostack, preserves_flags));
            if status & 0x20 != 0 {
                break;
            }
        }

        // Read how many LAPIC ticks elapsed
        let remaining = lapic_read(LAPIC_TIMER_CUR);
        let elapsed = 0xFFFFFFFF - remaining;
        let ticks_per_ms = elapsed as u64 / CALIBRATE_MS as u64;

        LAPIC_TICKS_PER_MS.store(ticks_per_ms, Ordering::Relaxed);

        let _ = write!(serial, "APIC: LAPIC ticks/ms = {} (elapsed {} in {}ms)\n",
            ticks_per_ms, elapsed, CALIBRATE_MS);

        // Now configure the real periodic timer
        let initial_count = (ticks_per_ms * 1000 / frequency_hz as u64) as u32;
        lapic_write(LAPIC_TIMER_LVT, TIMER_VECTOR | TIMER_PERIODIC);
        lapic_write(LAPIC_TIMER_INIT, initial_count);

        let _ = write!(serial, "APIC: Timer configured at {} Hz (initial_count={})\n",
            frequency_hz, initial_count);
    }
}

/// Send INIT IPI to a target APIC ID
pub unsafe fn send_init(target_apic_id: u8) {
    lapic_write(LAPIC_ICR_HI, (target_apic_id as u32) << 24);
    lapic_write(LAPIC_ICR_LO, ICR_INIT | ICR_LEVEL_ASSERT);

    // Wait for delivery
    ipi_wait();

    // De-assert
    lapic_write(LAPIC_ICR_HI, (target_apic_id as u32) << 24);
    lapic_write(LAPIC_ICR_LO, ICR_INIT | ICR_LEVEL_DEASSERT);

    ipi_wait();
}

/// Send STARTUP IPI to a target APIC ID with trampoline page number
pub unsafe fn send_sipi(target_apic_id: u8, trampoline_page: u8) {
    lapic_write(LAPIC_ICR_HI, (target_apic_id as u32) << 24);
    lapic_write(LAPIC_ICR_LO, ICR_STARTUP | trampoline_page as u32);
    ipi_wait();
}

/// Wait for IPI delivery to complete
unsafe fn ipi_wait() {
    for _ in 0..100000 {
        if lapic_read(LAPIC_ICR_LO) & (1 << 12) == 0 {
            return;
        }
        for _ in 0..100 {
            asm!("pause", options(nomem, nostack));
        }
    }
}

/// Disable the legacy 8259 PIC (mask all IRQs)
pub fn disable_pic() {
    unsafe {
        // Mask all IRQs on both PICs
        asm!("out dx, al", in("dx") 0x21u16, in("al") 0xFFu8, options(nomem, nostack));
        asm!("out dx, al", in("dx") 0xA1u16, in("al") 0xFFu8, options(nomem, nostack));
    }
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "APIC: Legacy 8259 PIC disabled\n");
}
