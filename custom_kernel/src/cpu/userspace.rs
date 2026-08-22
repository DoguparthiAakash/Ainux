use crate::cpu::gdt::{USER_CODE, USER_DATA};

// Enter Userspace (Ring 3)
// Arguments: entry_point in RDI, stack_ptr in RSI (System V ABI)
// Must be naked to avoid compiler prologue corrupting the iretq frame.
#[unsafe(naked)]
pub unsafe extern "C" fn enter_userspace(_entry_point: u64, _stack_ptr: u64) {
    // RDI = entry_point, RSI = stack_ptr
    // We build an iretq frame: [SS, RSP, RFLAGS, CS, RIP]
    // USER_DATA = 0x18 | 3 = 0x1B
    // USER_CODE = 0x20 | 3 = 0x23
    // RFLAGS = 0x202 (IF + reserved bit 1)
    core::arch::naked_asm!(
        "mov ax, 0x1B",
        "mov ds, ax",
        "mov es, ax",
        "push 0x1B",        // SS = USER_DATA
        "push rsi",         // RSP = stack_ptr
        "push 0x202",       // RFLAGS
        "push 0x23",        // CS = USER_CODE
        "push rdi",         // RIP = entry_point
        "iretq",
    );
}
