use core::arch::naked_asm;
use crate::process::task::Context;

#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn __switch(current: *mut Context, next: *const Context) {
    naked_asm!(
        // Function arguments: rdi = current, rsi = next
        
        // 1. Save current context to [rdi]
        // Struct: rsp(0), r15(8), r14(16), r13(24), r12(32), rbx(40), rbp(48), rip(56)

        "mov [rdi + 56], rax", // Helper for RIP... wait.
        // The return address is on the stack [rsp].
        // We pop it later? No, `ret` pops it.
        // We want to save the IP where we RESUME.
        // Resume point is after "call __switch".
        // The "ret" instruction will pop RIP.
        // So we just need to switch stacks.
        
        "mov [rdi + 8], r15",
        "mov [rdi + 16], r14",
        "mov [rdi + 24], r13",
        "mov [rdi + 32], r12",
        "mov [rdi + 40], rbx",
        "mov [rdi + 48], rbp",
        
        // Save current RSP (which points to return address)
        "mov [rdi + 0], rsp",

        // 2. Load next context from [rsi]
        "mov r15, [rsi + 8]",
        "mov r14, [rsi + 16]",
        "mov r13, [rsi + 24]",
        "mov r12, [rsi + 32]",
        "mov rbx, [rsi + 40]",
        "mov rbp, [rsi + 48]",
        
        // Load next RSP
        "mov rsp, [rsi + 0]",
        
        // RIP is handled by ret, which pops from the NEW stack.
        // If the new task was created manually, we must put its entry point on its stack.
        
        "ret"
    );
}
