; =============================================================================
; Ainux Multiboot Boot Stub — 32-bit Protected Mode → 64-bit Long Mode
; Passes Multiboot info struct pointer to Rust kernel_main(info_ptr: u64)
; Maps first 4GB identity + higher-half for ACPI/hardware access
; =============================================================================

global _start_multiboot
global boot_pml4
extern _start               ; Rust entry: fn _start() -> !

; We store the multiboot info pointer in a global so Rust can read it
global MULTIBOOT_INFO_PTR
global MULTIBOOT_MAGIC_VAL

section .multiboot
align 4
    dd 0x1BADB002            ; Multiboot1 magic
    dd 0x00000003            ; Flags: ALIGN(0) | MEMINFO(1)
    dd -(0x1BADB002 + 0x00000003) ; Checksum

section .data
align 8
MULTIBOOT_INFO_PTR: dq 0    ; Will hold physical address of multiboot_info
MULTIBOOT_MAGIC_VAL: dq 0   ; Will hold the magic number from EAX

section .bss
align 4096
boot_pml4:
    resb 4096
boot_pdpt:
    resb 4096
boot_pd0:                   ; PD for 0-1GB
    resb 4096
boot_pd1:                   ; PD for 1-2GB
    resb 4096
boot_pd2:                   ; PD for 2-3GB
    resb 4096
boot_pd3:                   ; PD for 3-4GB
    resb 4096
boot_pd_high:               ; PD for higher-half kernel
    resb 4096
stack_bottom:
    resb 32768               ; 32 KB stack (generous for SMP init)
stack_top:

section .text
bits 32
_start_multiboot:
    ; Save Multiboot info (EBX = info ptr, EAX = magic)
    mov [MULTIBOOT_INFO_PTR], ebx
    mov [MULTIBOOT_MAGIC_VAL], eax

    mov esp, stack_top

    ; ---- Zero out page tables ----
    mov edi, boot_pml4
    xor eax, eax
    mov ecx, 4096
    rep stosd
    mov edi, boot_pdpt
    mov ecx, 4096
    rep stosd
    mov edi, boot_pd0
    mov ecx, 4096
    rep stosd
    mov edi, boot_pd1
    mov ecx, 4096
    rep stosd
    mov edi, boot_pd2
    mov ecx, 4096
    rep stosd
    mov edi, boot_pd3
    mov ecx, 4096
    rep stosd
    mov edi, boot_pd_high
    mov ecx, 4096
    rep stosd

    ; ---- PML4[0] → PDPT (identity map) ----
    mov eax, boot_pdpt
    or eax, 0b11               ; Present + Writable
    mov [boot_pml4], eax

    ; ---- PML4[511] → same PDPT (higher-half) ----
    mov [boot_pml4 + 511 * 8], eax

    ; ---- PDPT[0] → PD0 (0-1GB) ----
    mov eax, boot_pd0
    or eax, 0b11
    mov [boot_pdpt], eax

    ; ---- PDPT[1] → PD1 (1-2GB) ----
    mov eax, boot_pd1
    or eax, 0b11
    mov [boot_pdpt + 1 * 8], eax

    ; ---- PDPT[2] → PD2 (2-3GB) ----
    mov eax, boot_pd2
    or eax, 0b11
    mov [boot_pdpt + 2 * 8], eax

    ; ---- PDPT[3] → PD3 (3-4GB) ----
    mov eax, boot_pd3
    or eax, 0b11
    mov [boot_pdpt + 3 * 8], eax

    ; ---- PDPT[510] → PD_high (higher-half: 0xFFFFFFFF80000000) ----
    mov eax, boot_pd_high
    or eax, 0b11
    mov [boot_pdpt + 510 * 8], eax

    ; ---- Fill PD0: 512 × 2MB = 1GB at 0x00000000 ----
    mov ecx, 0
.map_pd0:
    mov eax, ecx
    shl eax, 21              ; eax = ecx * 2MB
    or eax, 0b10000011       ; Present + Writable + HugePage
    mov [boot_pd0 + ecx * 8], eax
    mov [boot_pd_high + ecx * 8], eax  ; Same mapping for higher-half kernel
    inc ecx
    cmp ecx, 512
    jne .map_pd0

    ; ---- Fill PD1: 512 × 2MB = 1GB at 0x40000000 ----
    mov ecx, 0
.map_pd1:
    mov eax, ecx
    add eax, 512             ; offset by 512 entries (1GB)
    shl eax, 21
    or eax, 0b10000011
    mov [boot_pd1 + ecx * 8], eax
    inc ecx
    cmp ecx, 512
    jne .map_pd1

    ; ---- Fill PD2: 512 × 2MB = 1GB at 0x80000000 ----
    mov ecx, 0
.map_pd2:
    mov eax, ecx
    add eax, 1024            ; offset by 1024 entries (2GB)
    shl eax, 21
    or eax, 0b10000011
    mov [boot_pd2 + ecx * 8], eax
    inc ecx
    cmp ecx, 512
    jne .map_pd2

    ; ---- Fill PD3: 512 × 2MB = 1GB at 0xC0000000 ----
    mov ecx, 0
.map_pd3:
    mov eax, ecx
    add eax, 1536            ; offset by 1536 entries (3GB)
    shl eax, 21
    or eax, 0b10000011
    mov [boot_pd3 + ecx * 8], eax
    inc ecx
    cmp ecx, 512
    jne .map_pd3

    ; ---- Enable PAE (CR4 bit 5) ----
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; ---- Load PML4 into CR3 ----
    mov eax, boot_pml4
    mov cr3, eax

    ; ---- Enable Long Mode (EFER.LME, MSR 0xC0000080 bit 8) ----
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; ---- Enable Paging + Protection (CR0.PG | CR0.PE) ----
    mov eax, cr0
    or eax, (1 << 31) | (1 << 0)
    mov cr0, eax

    ; ---- Load 64-bit GDT and far jump to long mode ----
    lgdt [gdt64.pointer]
    jmp gdt64.code_segment:long_mode_start

; =============================================================================
; 64-bit GDT
; =============================================================================
align 8
gdt64:
    dq 0                                                    ; Null
.code_segment equ $ - gdt64
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53)              ; Code: Exec, Descriptor, Present, 64-bit
.data_segment equ $ - gdt64
    dq (1<<44) | (1<<47) | (1<<41)                        ; Data: Descriptor, Present, Writable
.pointer:
    dw $ - gdt64 - 1
    dq gdt64

; =============================================================================
; 64-bit Long Mode Entry
; =============================================================================
bits 64
long_mode_start:
    ; Set up data segments
    mov ax, gdt64.data_segment
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Set up a proper stack in the higher half
    mov rsp, stack_top

    ; Jump to Rust _start (higher-half linked)
    mov rax, _start
    jmp rax
