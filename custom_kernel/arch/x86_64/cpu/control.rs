use core::arch::asm;

/// Puts the CPU into a low-power state until the next interrupt.
/// Corresponds to the `hlt` instruction.
pub fn cpu_idle() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Halts the CPU indefinitely (loops `hlt`).
/// Used for fatal errors or system shutdown.
pub fn cpu_halt() -> ! {
    loop {
        cpu_idle();
    }
}

/// Hints to the CPU that we are in a spin-wait loop.
/// Corresponds to the `pause` instruction.
pub fn cpu_relax() {
    unsafe {
        asm!("pause", options(nomem, nostack, preserves_flags));
    }
}

/// Saves the current CPU flags (RFLAGS).
pub fn save_cpu_flags() -> u64 {
    let rflags: u64;
    unsafe {
        asm!("pushfq; pop {}", out(reg) rflags, options(nomem, preserves_flags));
    }
    rflags
}

/// Restores CPU flags from a saved value.
/// # Safety
/// This is unsafe because restoring flags can change interrupt state or other critical CPU modes.
pub unsafe fn restore_cpu_flags(flags: u64) {
    asm!("push {}; popfq", in(reg) flags, options(nomem, preserves_flags));
}

/// Checks if interrupts are currently enabled.
pub fn are_interrupts_enabled() -> bool {
    (save_cpu_flags() & 0x200) != 0
}

/// Ensure CPU state is consistent (e.g., CR0, CR4 flags).
pub fn check_cpu_state() {
    let cr0: u64;
    unsafe { asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags)); }
    
    // Check Protection Enable (PE) bit 0
    if (cr0 & 1) == 0 {
        panic!("CPU Invariant Violation: Protection Mode Disabled!");
    }
}

/// Placeholder for syncing CPU state in SMP (Symmetric Multiprocessing) environment.
pub fn sync_cpu_state() {
    // No-op for UP (Uniprocessor) kernel
}

/// Writes a 64-bit value to a Model Specific Register (MSR).
/// # Safety
/// Writing to invalid MSRs or setting invalid bits can cause a General Protection Fault.
pub unsafe fn wrmsr(msr: u32, val: u64) {
    let low = val as u32;
    let high = (val >> 32) as u32;
    asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nostack, preserves_flags));
}

/// Reads a 64-bit value from a Model Specific Register (MSR).
/// # Safety
/// Reading from invalid MSRs can cause a General Protection Fault.
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    asm!("rdmsr", out("eax") low, out("edx") high, in("ecx") msr, options(nostack, preserves_flags));
    ((high as u64) << 32) | (low as u64)
}
