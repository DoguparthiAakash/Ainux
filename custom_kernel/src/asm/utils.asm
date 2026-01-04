global asm_magic_op

section .text
asm_magic_op:
    ; Just a dummy function
    ; Return 42 in RAX
    mov rax, 42
    ret
