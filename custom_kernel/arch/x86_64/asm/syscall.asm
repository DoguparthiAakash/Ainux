global syscall_entry
extern syscall_handler
extern TSS

section .data
align 8
; Scratch space for saving user RSP during syscall entry (single-CPU safe)
global user_rsp_scratch
user_rsp_scratch: dq 0

section .text
bits 64

; ======================================================================
; syscall_entry — Ring 3 → Ring 0 via SYSCALL instruction
;
; On entry (hardware sets these):
;   RCX = user RIP (return address)
;   R11 = user RFLAGS
;   CS/SS switched per STAR MSR
;   RSP = UNCHANGED (still user RSP!)
;
; Linux syscall convention:
;   RAX = syscall number
;   RDI = arg1, RSI = arg2, RDX = arg3, R10 = arg4, R8 = arg5, R9 = arg6
; ======================================================================

syscall_entry:
    ; 1. Save user RSP to scratch (we can't push yet — user stack!)
    mov [rel user_rsp_scratch], rsp

    ; 2. Load kernel stack from TSS.rsp0 (offset 4 in the packed TSS struct)
    mov rsp, [rel TSS + 4]

    ; 3. Now on kernel stack — save user context
    push qword [rel user_rsp_scratch]   ; user RSP
    push rcx                             ; user RIP
    push r11                             ; user RFLAGS

    ; 4. Save callee-saved registers and all syscall argument registers!
    ; Linux ABI expects syscalls to preserve everything except RAX, RCX, R11.
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    push rdi
    push rsi
    push rdx
    push r10
    push r8
    push r9

    ; 5. Set up arguments for Rust syscall_handler(sysno, a1, a2, a3, a4, a5, a6)
    ;    System V AMD64 ABI: rdi, rsi, rdx, rcx, r8, r9, [rsp+8]
    ;    SYSCALL ABI:        rax=sysno, rdi, rsi, rdx, r10, r8, r9

    ; Push r9 (arg6) as 7th argument on stack
    push r9

    ; Remap registers
    mov r9, r8              ; arg6 → r9 (Rust arg6)
    mov r8, r10             ; arg5 (syscall R10=arg4) → r8 (Rust arg5)
    mov rcx, rdx            ; arg3 → rcx (Rust arg4)
    mov rdx, rsi            ; arg2 → rdx (Rust arg3)
    mov rsi, rdi            ; arg1 → rsi (Rust arg2)
    mov rdi, rax            ; sysno → rdi (Rust arg1)

    ; 6. Call the Rust syscall dispatcher
    call syscall_handler

    ; 7. Clean up 7th argument from stack
    add rsp, 8

    ; 8. Restore all syscall argument registers and callee-saved registers
    pop r9
    pop r8
    pop r10
    pop rdx
    pop rsi
    pop rdi
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx

    ; 9. Restore user context
    pop r11                 ; user RFLAGS
    pop rcx                 ; user RIP
    pop rsp                 ; user RSP (direct pop into RSP!)

    ; 10. Return to userspace
    ; RAX already contains the return value from syscall_handler
    o64 sysret
