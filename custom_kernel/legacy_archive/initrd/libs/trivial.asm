; Trivial Syscall Test
; Just call SYS_READ (0) with dummy args
MOV RAX, 0
MOV RDI, 1
MOV RSI, 0
MOV RDX, 0
INT 128
RET
