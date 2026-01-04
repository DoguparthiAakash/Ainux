[bits 64]

global _start
extern main
extern sys_exit

section .text

_start:
    ; Provide dummy argc/argv for now
    mov rdi, 0 ; argc
    mov rsi, 0 ; argv
    
    ; Call main
    call main
    
    ; Exit with return value
    mov rdi, rax
    call sys_exit
    
    ; Should never reach here
    hlt
