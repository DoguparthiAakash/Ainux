[BITS 64]
global _start
extern kmain

section .text
_start:
    ; Limine enters in 64-bit mode.
    ; Interrupts disabled, Paging enabled.
    ; We need to set up stack? Limine provides one if requested.
    ; But we have a stack in BSS. Let's use it.
    
    ; Stack setup
    mov rsp, stack_top
    
    ; Clear RBP
    xor rbp, rbp
    
    ; Call kmain(0) -> NULL boot_info signals Limine check
    xor rdi, rdi
    call kmain
    
    ; Halt if return
    cli
    hlt
    jmp $

section .bss
align 16
stack_bottom:
    resb 16384 ; 16KB Stack
stack_top:
