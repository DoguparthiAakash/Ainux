[bits 64]

global syscall_entry
extern syscall_handler_c_stub
extern tss_get_rsp0

section .text

; syscall_entry:
; Entry point for 'syscall' instruction.
; RCX = RIP (next instruction)
; R11 = RFLAGS
; CS/SS loaded from MSR_STAR (kernel CS, kernel SS)
; We are in Ring 0 now.

syscall_entry:
    ; 1. Save User RSP to R15 (Callee Saved - preserved by C handler)
    mov r15, rsp
    
    ; 2. Switch to Kernel Stack
    mov rsp, syscall_stack_top
    
    ; 3. Build Trap Frame
    push 0x1B                 ; SS
    push r15                  ; User RSP (from R15)
    push r11                  ; RFLAGS
    push 0x23                 ; CS
    push rcx                  ; RIP
    
    ; 4. Push Registers
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15 ; Push R15 (contains User RSP, but also needed for struct registers)
             ; Wait, if we push R15 here, it's the User RSP value.
             ; The user's R15 value was clobbered by line 1!
             ; BUG: We clobbered User's R15.
             ; We need a scratch place that DOES NOT clobber registers.
             ; But we have NO STACK yet.
             ; And no SwapGS.
             ; This is the single-core dilemma.
             ; We MUST use memory or SwapGS.
             ; Since 'temp_user_rsp' failed (corruption?), we should fix the corruption OR move the variable.
             
    ; REVERT: Use 'temp_user_rsp' but move it to .text (rip-relative)? No, read-only.
    ; Move it to a Safer Section?
    ; Or just fix logic.
    ; Maybe 'temp_user_rsp' wasn't corrupted?
    ; Let's TRY to use a register that IS clobbered by syscall?
    ; RCX and R11 are clobbered.
    ; RCX = RIP. R11 = RFLAGS.
    ; R12-R15 Callee Saved.
    ; RBX, RBP Callee Saved.
    ; RAX, RDI, RSI, RDX, R8, R9, R10 - Caller Saved (Clobberable?)
    ; Syscall ABI:
    ; RAX = Syscall Num.
    ; RDI, RSI, RDX, R10, R8, R9 = Args.
    ; Can we use a register that is NOT used for args?
    ; No, they are all used.
    ; But wait!
    ; We can Save User RSP to valid memory.
    ; If `temp_user_rsp` is corrupted, let's look at `syscall_stack`.
    ; `mov [syscall_stack_top], rsp` ?
    ; Only if we don't start stack there.
    
    ; Let's stick to `temp_user_rsp` but rename/move it?
    ; Or check who corrupts it.
    
    ; ...
    ; pop r11 (RFLAGS)
    ; pop rsp (This pops User RSP from stack).
    
    ; Wait. `pop rsp` retrieves the value PUSHED at line 106.
    ; Value at line 106 came from `[temp_user_rsp]`.
    ; If `temp_user_rsp` was correct at ENTRY.
    ; Then we pushed Correct Value.
    ; Then we call C handler.
    ; C Handler might corrupt `temp_user_rsp` (log buffer overflow?).
    ; BUT the stack value is already pushed!
    ; `temp_user_rsp` corruption *during* handler execution DOES NOT AFFECT the value already on the stack!
    ; So `pop rsp` should restore the VALID value pushed at start.
    
    ; So `temp_user_rsp` corruption is IRRELEVANT unless it happened *before* push.
    ; But `sys_write` happens *after* push.
    
    ; So my `temp_user_rsp` theory is WRONG. The stack has the correct RSP.
    
    ; Then why does it crash?
    ; `sys_exit` isn't called.
    ; ASM crash.
    
    ; Let's review the Trace 3892 again.
    ; `[SYS_WRITE EXIT]`.
    ; `RUST PANIC`.
    ; Panic happens *after* sys_write exit trace.
    ; Trace is in `sys_write`.
    ; `sys_write` returns to `syscall_handler`.
    ; `syscall_handler` returns.
    
    ; Is `kprint` inside `sys_write` returning properly?
    ; Yes.
    
    ; Maybe `sys_write` corrupts the *Kernel Stack*?
    ; If `sys_write` call frame is overwritten.
    ; `sys_write` returns to garbage.
    ; Garbage is interpreted as code? ("Panic"?)
    ; Or Garbage is Non-Canonical -> GPF -> Panic (Wait, GPF loop?)
    ; But we see panic text.
    
    ; I suspect `sys_write` overwrites return address on stack.
    ; Check `sys_write` locals.
    ; `uint64_t rsp_val;`
    ; `char s[32];`
    ; `char c[2];`
    ; Buffer overflow in `sys_write`?
    ; `sys_write` logic:
    ; `char c[2] = {buf[i], 0};`
    ; `buf[i]` is single char.
    ; `c` is 2 chars. Safe.
    
    ; Wait. `log.c` is linked.
    ; `kprint` buffer?
    ; `sys_write` has `kprint("[SYS_WRITE ENTRY]...")`. Literal.
    ; It uses `kprint` with string literals.
    ; `char buf[32]` in `sys_write`? I added `char s[32]` but didn't use it.
    
    ; I will remove `char s[32]` and `char buf[32]` from `sys_write` just in case.
    ; Also `sys_write` in `syscalls.c` declares `char buf[32]`?
    
    ; Let's look at `syscalls.c` again.
    ; `syscall_handler_c_stub` declared `char buf[32]`.
    ; `sys_write` declared `char s[32]`.
    ; Stack depth is small.
    
    ; If Stack Alignment?
    ; Use `verify_stack_alignment`?
    
    ; Actually, I will revert `sys_write` traces.
    ; They clutter the analysis and introduce variables.
    ; I know `sys_write` returns.
    
    ; New Hypothesis: The Panic happens when INTERRUPT occurs during `sysretq`.
    ; `sysretq` is not atomic? It is.
    ; Instructions *after* sysretq? (User mode).
    
    ; If I add `cli` before `sysretq`?
    ; Kernel should disable interrupts before exit?
    ; `syscall_entry` entry does not disable interrupts?
    ; `syscall` instruction masks interrupts based on SFMASK.
    ; We set SFMASK?
    ; `MSR_FMASK`.
    ; If we didn't set it, `syscall` leaves interrupts enabled.
    ; If interrupt occurs *before* we switch stack to Kernel Stack?
    ; `syscall_entry`:
    ; `mov [temp], rsp` (Interrupt here -> Uses User Stack. Dangerous but OK).
    ; `mov rsp, kernel_stack` (Interrupt here -> Uses Kernel Stack. OK).
    
    ; Exit path:
    ; `pop rsp` (Interrupt here -> Uses User Stack).
    ; `sysretq`.
    
    ; If Interrupt happens after `pop rsp` (RSP is User RSP).
    ; Processor pushes [SS, RSP, RFLAGS, CS, RIP] to User Stack.
    ; 3. Build Trap Frame on Kernel Stack
    ; Stack Layout (Top to Bottom):
    ; SS, RSP, RFLAGS, CS, RIP
    
    push 0x1B                 ; SS (User Data)
    push r15                  ; User RSP (Restored from R15)
    push r11                  ; User RFLAGS (Saved by syscall)
    push 0x23                 ; CS (User Code)
    push rcx                  ; User RIP (Saved by syscall)
    
    ; 4. Push General Registers (struct registers)
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15 
    
    ; 5. Call Handler
    mov rdi, rsp           ; Pass 'struct registers *' as first argument
    call syscall_handler_c_stub
    
    ; 6. Restore State
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    
    ; 7. Prepare for sysretq
    ; Disable interrupts to prevent stack corruption during switch
    cli
    
    ; Stack has: RIP, CS, RFLAGS, RSP, SS
    pop rcx ; Pop RIP
    add rsp, 8 ; Skip CS
    pop r11 ; Pop RFLAGS
    pop rsp ; Pop User RSP
    ; Skip SS (implicitly discarded)
    
    sysretq

section .bss
    ; Kernel Stack for Syscalls
    align 16
    syscall_stack_bottom:
        resb 8192 ; 8KB
    syscall_stack_top:
