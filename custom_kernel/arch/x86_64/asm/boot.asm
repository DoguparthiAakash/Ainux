; =============================================================================
; Ainux Multiboot Boot Stub — 32-bit Protected Mode → 64-bit Long Mode
; Passes Multiboot info struct pointer to Rust kernel_main(info_ptr: u64)
; Maps first 4GB identity + higher-half for ACPI/hardware access
;
; OPTIMIZED: Merged PD loops, fixed STOSD count, NX+PGE enabled
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
    dd 0x00000007            ; Flags: ALIGN(0) | MEMINFO(1) | VIDEO(2)
    dd -(0x1BADB002 + 0x00000007) ; Checksum
    dd 0                     ; header_addr
    dd 0                     ; load_addr
    dd 0                     ; load_end_addr
    dd 0                     ; bss_end_addr
    dd 0                     ; entry_addr
    dd 0                     ; mode_type (0 = linear graphics)
    dd 0                     ; width (auto)
    dd 0                     ; height (auto)
    dd 32                    ; depth (32 bpp preferred)

section .data
align 8
MULTIBOOT_INFO_PTR: dq 0    ; Will hold physical address of multiboot_info
MULTIBOOT_MAGIC_VAL: dq 0   ; Will hold the magic number from EAX

; Table of PD base addresses for the unified fill loop
align 8
pd_table:
    dd boot_pd0              ; PD index 0: maps 0x00000000 - 0x3FFFFFFF
    dd boot_pd1              ; PD index 1: maps 0x40000000 - 0x7FFFFFFF
    dd boot_pd2              ; PD index 2: maps 0x80000000 - 0xBFFFFFFF
    dd boot_pd3              ; PD index 3: maps 0xC0000000 - 0xFFFFFFFF

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

    ; ---- Zero out ALL page tables in one shot ----
    ; 7 tables × 4096 bytes = 28672 bytes = 7168 dwords
    ; Tables are contiguous in BSS: pml4, pdpt, pd0, pd1, pd2, pd3, pd_high
    mov edi, boot_pml4
    xor eax, eax
    mov ecx, 7168            ; 7 × 1024 dwords = 7 × 4096 bytes
    rep stosd

    ; ---- PML4[0] → PDPT (identity map) ----
    mov eax, boot_pdpt
    or eax, 0b11               ; Present + Writable
    mov [boot_pml4], eax

    ; ---- PML4[511] → same PDPT (higher-half) ----
    mov [boot_pml4 + 511 * 8], eax

    ; ---- PDPT[0..3] → PD0..PD3 ----
    mov eax, boot_pd0
    or eax, 0b11
    mov [boot_pdpt], eax

    mov eax, boot_pd1
    or eax, 0b11
    mov [boot_pdpt + 1 * 8], eax

    mov eax, boot_pd2
    or eax, 0b11
    mov [boot_pdpt + 2 * 8], eax

    mov eax, boot_pd3
    or eax, 0b11
    mov [boot_pdpt + 3 * 8], eax

    ; ---- PDPT[510] → PD_high (higher-half: 0xFFFFFFFF80000000) ----
    mov eax, boot_pd_high
    or eax, 0b11
    mov [boot_pdpt + 510 * 8], eax

    ; ---- Unified PD fill: 4 PDs × 512 entries each ----
    ; Maps full 4GB identity using 2MB huge pages
    ; PD_high mirrors PD0 for higher-half kernel access
    ;
    ; Outer loop: ebx = PD index (0..3), using pd_table for base addresses
    ; Inner loop: ecx = entry index (0..511)
    ; Physical address = (ebx * 512 + ecx) << 21
    ; Flags: Present(0) + Writable(1) + HugePage(7) = 0x83
    ; Kernel PD_high entries also get GLOBAL(8) = 0x183

    xor ebx, ebx             ; PD index = 0
.fill_pd_outer:
    mov esi, [pd_table + ebx * 4]  ; ESI = base of current PD
    xor ecx, ecx             ; entry index = 0

.fill_pd_inner:
    ; Compute physical page: (ebx * 512 + ecx) * 2MB
    mov eax, ebx
    shl eax, 9               ; eax = ebx * 512
    add eax, ecx             ; eax = ebx * 512 + ecx
    shl eax, 21              ; eax = physical address (2MB aligned)
    or eax, 0b10000011       ; Present + Writable + HugePage

    mov [esi + ecx * 8], eax ; Write PD entry

    ; For PD0 (ebx==0): also mirror into boot_pd_high with GLOBAL bit
    test ebx, ebx
    jnz .skip_high
    mov edx, eax
    or edx, (1 << 8)         ; Add GLOBAL flag for kernel mappings
    mov [boot_pd_high + ecx * 8], edx
.skip_high:

    inc ecx
    cmp ecx, 512
    jne .fill_pd_inner

    inc ebx
    cmp ebx, 4
    jne .fill_pd_outer

    ; ---- Enable PAE (CR4.PAE bit 5) + PGE (CR4.PGE bit 7) ----
    ; PGE enables GLOBAL bit in page table entries for TLB persistence
    mov eax, cr4
    or eax, (1 << 5) | (1 << 7)
    mov cr4, eax

    ; ---- Load PML4 into CR3 ----
    mov eax, boot_pml4
    mov cr3, eax

    ; ---- Enable Long Mode + NX (EFER.LME bit 8 + EFER.NXE bit 11) ----
    ; NX enables the No-Execute bit in page tables for W^X enforcement
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8) | (1 << 11)
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
