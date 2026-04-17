; =============================================================================
; Ainux AP Trampoline — Real Mode → Protected Mode → Long Mode
; Assembled as flat binary, copied to 0x8000 at runtime.
; 
; Data at fixed offsets (patched by kernel before SIPI):
;   [0x0FF0] = CR3 value (8 bytes)
;   [0x0FF8] = ap_entry address (8 bytes)
;
; Build: nasm -f bin src/asm/ap_trampoline.asm -o src/asm/ap_trampoline.bin
; =============================================================================

[org 0x8000]
[bits 16]

ap_trampoline_start:
    cli
    cld

    ; Zero all data segments
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax

    ; Load 32-bit GDT
    lgdt [ap_gdt32_ptr]

    ; Enable Protected Mode (CR0.PE)
    mov eax, cr0
    or eax, 1
    mov cr0, eax

    ; Far jump to 32-bit code (flush pipeline)
    jmp 0x08:ap_pm_entry

; =============================================================================
[bits 32]
ap_pm_entry:
    ; Set up 32-bit data segments
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    ; Enable PAE (CR4.PAE = bit 5)
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax

    ; Load CR3 from data area (BSP's page tables)
    mov eax, [0x8FF0]
    mov cr3, eax

    ; Enable Long Mode (EFER.LME = bit 8)
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)
    wrmsr

    ; Enable Paging (CR0.PG = bit 31)
    mov eax, cr0
    or eax, (1 << 31)
    mov cr0, eax

    ; Load 64-bit GDT
    lgdt [ap_gdt64_ptr]

    ; Far jump to 64-bit code
    jmp 0x08:ap_lm_entry

; =============================================================================
[bits 64]
ap_lm_entry:
    ; Set up 64-bit data segments
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Set temporary stack (4KB at 0x9000)
    mov rsp, 0x9000

    ; Load ap_entry address from data area
    mov rax, [0x8FF8]

    ; Jump to Rust ap_entry()
    jmp rax

; =============================================================================
; 32-bit GDT (for PM transition)
; =============================================================================
align 8
ap_gdt32:
    dq 0x0000000000000000       ; 0x00: Null
    dq 0x00CF9A000000FFFF       ; 0x08: 32-bit Code (base=0, limit=4GB, exec, read)
    dq 0x00CF92000000FFFF       ; 0x10: 32-bit Data (base=0, limit=4GB, read, write)
ap_gdt32_end:

ap_gdt32_ptr:
    dw ap_gdt32_end - ap_gdt32 - 1
    dd ap_gdt32

; =============================================================================
; 64-bit GDT (for LM transition)
; =============================================================================
align 8
ap_gdt64:
    dq 0x0000000000000000       ; 0x00: Null
    dq 0x00209A0000000000       ; 0x08: 64-bit Code (L=1, P=1, S=1, Type=Exec/Read)
    dq 0x0000920000000000       ; 0x10: 64-bit Data (P=1, S=1, Type=Read/Write)
ap_gdt64_end:

ap_gdt64_ptr:
    dw ap_gdt64_end - ap_gdt64 - 1
    dd ap_gdt64

; =============================================================================
; Pad to ensure data area is at known offsets from 0x8000
; Data area: [0xFF0..0x1000] patched by kernel at runtime
; =============================================================================
times (0xFF0 - ($ - ap_trampoline_start)) db 0x90

ap_data_cr3:    dq 0            ; [0x8FF0] = BSP's CR3 (patched by kernel)
ap_data_entry:  dq 0            ; [0x8FF8] = ap_entry address (patched by kernel)
