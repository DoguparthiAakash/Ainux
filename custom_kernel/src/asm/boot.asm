; src/asm/boot.asm
; Ainux Bootloader Shim — Multiboot to Long Mode (64-bit)

extern _start
; extern rust_main ; Renamed to _start in main.rs

section .multiboot
align 4
    dd 0x1BADB002               ; Magic
    dd 0x00                     ; Flags
    dd -(0x1BADB002 + 0x00)     ; Checksum

; =============================================================================
; 32-bit Entry Point
; =============================================================================
bits 32
global _start_multiboot
_start_multiboot:
    cli
    cld

    ; Save Multiboot magic and info pointer
    mov [MULTIBOOT_MAGIC_VAL], eax
    mov [MULTIBOOT_INFO_PTR], ebx

    ; Set up a temporary boot stack (low memory)
    mov esp, boot_stack_top

    ; ---- Paging Setup (Simplified & Robust) ----
    ; Clear page tables (6 pages: PML4, PDPT, 4 PDs)
    mov edi, boot_pml4
    xor eax, eax
    mov ecx, 1024 * 6
    rep stosd

    ; 1. Link PML4 entries
    mov eax, boot_pdpt
    or eax, 0b11 ; Present + Writable
    mov [boot_pml4], eax               ; Identity Map (0..512GB)
    mov [boot_pml4 + 256 * 8], eax     ; Direct Map (0xFFFF800000000000)
    mov [boot_pml4 + 511 * 8], eax     ; Higher Half (0xFFFFFFFF80000000)

    ; 2. Link PDPT entries (Map 4GB)
    ; Identity & Direct Map: 0-4GB (Indices 0, 1, 2, 3)
    ; Higher Half Map: -2GB and -1GB (Indices 510, 511)
    
    mov eax, boot_pd
    or eax, 0b11
    mov [boot_pdpt + 0 * 8], eax       ; Identity 0-1GB
    mov [boot_pdpt + 510 * 8], eax     ; High-Half 0-1GB (at -2GB)

    mov eax, boot_pd + 4096
    or eax, 0b11
    mov [boot_pdpt + 1 * 8], eax       ; Identity 1-2GB
    mov [boot_pdpt + 511 * 8], eax     ; High-Half 1-2GB (at -1GB)

    mov eax, boot_pd + 8192
    or eax, 0b11
    mov [boot_pdpt + 2 * 8], eax       ; Identity 2-3GB

    mov eax, boot_pd + 12288
    or eax, 0b11
    mov [boot_pdpt + 3 * 8], eax       ; Identity 3-4GB

    ; 4. Map PD[0-3] to 0-4GB using 2MB huge pages
    mov edi, boot_pd
    mov eax, 0 | 0x83         ; Start at 0MB
    mov ecx, 512 * 4          ; Fill 4 Page Directories
.fill_pd:
    mov [edi], eax
    mov dword [edi + 4], 0    ; Clear upper 32 bits
    add eax, 0x200000         ; Next 2MB
    add edi, 8
    loop .fill_pd

    ; ---- Enable PAE and Long Mode ----
    mov eax, cr4
    or eax, 1 << 5 ; PAE
    mov cr4, eax

    mov eax, boot_pml4
    mov cr3, eax

    mov ecx, 0xC0000080 ; EFER
    rdmsr
    or eax, 1 << 8 ; LME
    wrmsr

    mov eax, cr0
    or eax, 1 << 31 | 1 << 0 ; PG | PE
    mov cr0, eax

    ; ---- Transition to 64-bit ----
    lgdt [gdt64_ptr]
    jmp gdt64_code:long_mode_start

align 8
gdt64:
    dq 0 ; null
gdt64_code: equ $ - gdt64
    dq (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53) ; code
gdt64_data: equ $ - gdt64
    dq (1 << 44) | (1 << 47) | (1 << 41) ; data
gdt64_ptr:
    dw $ - gdt64 - 1
    dd gdt64

[bits 64]
long_mode_start:
    mov ax, gdt64_data
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Set up higher-half stack
    mov rsp, stack_top

    ; Enter Rust
    mov rax, _start
    jmp rax

; =============================================================================
; Low-Memory Boot Sections (Paging Tables & Temporary Stack)
; =============================================================================
section .boot_bss
align 4096
boot_pml4:
    resb 4096
boot_pdpt:
    resb 4096
boot_pd:
    resb 4096 * 4  ; 4 PDs to cover 4GB if needed

boot_stack_bottom:
    resb 4096      ; Small temporary stack for 32-bit boot only
boot_stack_top:

global MULTIBOOT_INFO_PTR
global MULTIBOOT_MAGIC_VAL
MULTIBOOT_INFO_PTR:  resq 1
MULTIBOOT_MAGIC_VAL: resq 1

; =============================================================================
; Higher-Half Sections (Final Kernel Stack)
; =============================================================================
section .stack
align 4096
stack_bottom:
    resb 16384 * 8 ; 128KB stack (High Half)
stack_top:
