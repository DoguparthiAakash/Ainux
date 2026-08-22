global asm_magic_op
global fast_memcpy
global fast_memset

section .text
asm_magic_op:
    mov rax, 42
    ret

; void fast_memcpy(void* dest, const void* src, size_t count)
; RDI = dest, RSI = src, RDX = count
fast_memcpy:
    mov rcx, rdx
    rep movsb
    ret

; void fast_memset(void* dest, u8 val, size_t count)
; RDI = dest, RSI = val (in SIL), RDX = count
fast_memset:
    mov rcx, rdx
    mov rax, rsi
    rep stosb
    ret
