pub mod gdt;
pub mod idt;
pub mod pic;
pub mod syscall;
pub mod userspace;
pub mod control;
pub mod acpi;
pub mod apic;
pub mod smp;
pub mod cpuid;
pub mod percpu;

pub fn interrupts_enabled() -> bool {
    let rflags: u64;
    unsafe {
        core::arch::asm!("pushfq", "pop {0}", out(reg) rflags, options(nomem, nostack));
    }
    (rflags & (1 << 9)) != 0
}

/// Executes a closure with interrupts disabled and restores previous state.
pub fn without_interrupts<F, R>(f: F) -> R 
where F: FnOnce() -> R 
{
    let enabled = interrupts_enabled();
    if enabled { unsafe { core::arch::asm!("cli", options(nomem, nostack)); } }
    let ret = f();
    if enabled { unsafe { core::arch::asm!("sti", options(nomem, nostack)); } }
    ret
}
