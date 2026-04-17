use core::arch::asm;
use crate::cpu::gdt::{USER_CODE, USER_DATA};

// Enter Userspace (Ring 3)
// Arguments: function pointer (RIP), stack pointer (RSP)
pub unsafe fn enter_userspace(entry_point: u64, stack_ptr: u64) {
    // We must fake an interrupt stack frame to return "back" to Ring 3.
    // Frame: [SS, RSP, RFLAGS, CS, RIP]
    
    // Selectors must be ORed with 3 (RPL=3).
    // USER_DATA = 0x18 | 3 = 0x1B
    // USER_CODE = 0x20 | 3 = 0x23
    
    // RFLAGS: 0x202 (Interrupts Enabled, Reserved Bit 1 set)
    
    asm!(
        "push {ss}",
        "push {rsp}",
        "push {rflags}",
        "push {cs}",
        "push {rip}",
        "iretq",
        ss = in(reg) USER_DATA as u64,
        rsp = in(reg) stack_ptr,
        rflags = const 0x202,
        cs = in(reg) USER_CODE as u64,
        rip = in(reg) entry_point,
        options(noreturn)
    );
}
