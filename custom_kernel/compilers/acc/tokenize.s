.section .text
.global fast_tokenize

# int fast_tokenize(const char* source, char* out_tokens)
# RDI = source, RSI = out_tokens
# Returns token count in RAX
fast_tokenize:
    push %rbp
    mov %rsp, %rbp
    
    # Very basic tokenizer stub
    # Loop over source, skip spaces, write token types to out_tokens
    xor %rcx, %rcx          # rcx = out index / token count
    
.Lloop:
    movzbq (%rdi), %rdx     # read character from source
    test %rdx, %rdx         # if null, break
    jz .Ldone
    
    # Check if space
    cmp $32, %rdx           # ' '
    je .Lskip
    cmp $10, %rdx           # '\n'
    je .Lskip
    cmp $9, %rdx            # '\t'
    je .Lskip
    
    # Otherwise, just count as a basic token and write '1' to out_tokens
    movb $49, (%rsi, %rcx, 1) # write '1'
    inc %rcx
    
.Lskip:
    inc %rdi
    jmp .Lloop
    
.Ldone:
    movb $0, (%rsi, %rcx, 1)  # null terminate
    mov %rcx, %rax            # return token count
    
    pop %rbp
    ret
