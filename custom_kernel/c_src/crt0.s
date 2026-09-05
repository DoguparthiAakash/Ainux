// Mithl OS C Runtime Startup (crt0)
// This is the entry point for ELF executables. The kernel ELF loader will jump to _start.

.section .text
.global _start
.extern main
.extern exit

_start:
    // The kernel is expected to pass argc in rdi, argv in rsi, envp in rdx
    // Standard System V AMD64 ABI sets up arguments on the stack, but for simplicity
    // in our initial loader, we'll pass them in registers.
    
    // Call main(argc, argv, envp)
    call main

    // Exit the process with the return code from main
    mov rdi, rax
    call exit

    // If exit() somehow returns, force a hardware fault or loop
.hlt_loop:
    hlt
    jmp .hlt_loop
