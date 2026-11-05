; =============================================================================
; Ainux / Mithl OS Multiboot Boot Stub
; Multi-Architecture Safe Bootstrap: 32-bit Protected Mode -> 64-bit Long Mode
; Supports: Limbo PC Emulator (x86 & x86_64), QEMU, KVM, VirtualBox, VMware, Bare Metal
; Dynamic CPUID capability detection (PAE, PGE, NX, SSE, Long Mode)
; Provides interactive 32-bit hardware diagnostics if 64-bit Long Mode unsupported
; =============================================================================

global _start_multiboot
global boot_pml4
extern _start               ; Rust entry: fn _start() -> !

; Exported symbols for Rust kernel memory & CPU management
global MULTIBOOT_INFO_PTR
global MULTIBOOT_MAGIC_VAL
global BOOT_CPU_VENDOR
global BOOT_CPU_SIGNATURE
global BOOT_CPU_FLAGS_EDX
global BOOT_CPU_EXT_EDX
global BOOT_IS_64BIT

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
MULTIBOOT_INFO_PTR:  dq 0    ; Physical address of multiboot_info
MULTIBOOT_MAGIC_VAL: dq 0    ; Magic number from EAX

BOOT_CPU_VENDOR:     times 16 db 0
BOOT_CPU_SIGNATURE:  dd 0
BOOT_CPU_FLAGS_EDX:  dd 0
BOOT_CPU_EXT_EDX:    dd 0
BOOT_IS_64BIT:       dd 0

zero_idt:
    dw 0
    dd 0

; Table of PD base addresses for the unified fill loop
align 8
pd_table:
    dd boot_pd0              ; PD index 0: maps 0x00000000 - 0x3FFFFFFF
    dd boot_pd1              ; PD index 1: maps 0x40000000 - 0x7FFFFFFF
    dd boot_pd2              ; PD index 2: maps 0x80000000 - 0xBFFFFFFF
    dd boot_pd3              ; PD index 3: maps 0xC0000000 - 0xFFFFFFFF

; Serial and diagnostic strings
str_crlf:
    db 13, 10, 0

str_banner:
    db "================================================================================", 13, 10
    db "      Mithl OS / Ainux -- Multi-Architecture CPU Hardware Diagnostics", 13, 10
    db "================================================================================", 13, 10, 0

str_detected_32bit:
    db "[!] 32-Bit x86 CPU Architecture Detected (IA-32 Protected Mode)", 13, 10
    db "    64-bit Long Mode is NOT available with current CPU configuration.", 13, 10, 13, 10, 0

str_cpu_vendor_hdr:
    db "  CPU Vendor    : ", 0

str_limbo_guide:
    db 13, 10
    db ">> How to configure Limbo PC Emulator (Android) for 64-bit Mithl OS:", 13, 10
    db "   1. Stop / Power off the VM in the Limbo app.", 13, 10
    db "   2. In VM Settings -> 'CPU / Board':", 13, 10
    db "      * Change Architecture: 'x86' -> 'x86_64'", 13, 10
    db "      * Change CPU Model   : 'qemu64', 'core2duo', or 'host'", 13, 10
    db "      * Allocate RAM       : 512 MB or 1024 MB", 13, 10
    db "   3. Start the VM -> Full 64-bit Mithl OS Desktop GUI will launch!", 13, 10, 13, 10
    db ">> Available Actions:  [R] Reboot System    [H] Halt CPU", 13, 10, 0

str_rebooting:
    db "Rebooting system...", 13, 10, 0

str_halted:
    db "System halted. Power off machine.", 13, 10, 0

; VGA text strings
vga_title:
    db "  Mithl OS / Ainux - Multi-Architecture Bootloader & Hardware Diagnostics", 0

vga_alert:
    db "  [!] 32-Bit x86 CPU Architecture Detected (IA-32 Protected Mode)", 0

vga_subalert:
    db "      64-bit Long Mode is not available with current CPU configuration.", 0

vga_hw_hdr:
    db "  -- CPU Hardware Details --", 0

vga_vendor_lbl:
    db "    CPU Vendor    : ", 0

vga_arch_lbl:
    db "    Architecture  : 32-bit x86 (IA-32 Protected Mode)", 0

vga_longmode_lbl:
    db "    Long Mode     : Disabled / Unsupported on this CPU", 0

vga_guide_hdr:
    db "  -- How to Run Full 64-bit Desktop GUI in Limbo PC Emulator --", 0

vga_step1:
    db "    1. Shut down / Power off VM in Limbo app.", 0

vga_step2:
    db "    2. Open VM Settings -> 'CPU / Board':", 0

vga_step3:
    db "       * Architecture: Change 'x86' to 'x86_64'", 0

vga_step4:
    db "       * CPU Model   : Choose 'qemu64', 'core2duo', or 'host'", 0

vga_step5:
    db "       * RAM Memory  : Set to 512 MB or 1024 MB", 0

vga_step6:
    db "    3. Boot VM -> Full 64-bit Mithl OS GUI will launch automatically!", 0

vga_ctrl_hdr:
    db "  -- Interactive Controls --", 0

vga_ctrl_keys:
    db "    Press [R] to Reboot System        Press [H] to Halt System", 0

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
    resb 32768               ; 32 KB stack
stack_top:

section .text
bits 32
_start_multiboot:
    cli
    cld

    ; Save Multiboot info (EBX = info ptr, EAX = magic)
    mov [MULTIBOOT_INFO_PTR], ebx
    mov [MULTIBOOT_MAGIC_VAL], eax

    mov esp, stack_top

    ; ─────────────────────────────────────────────────────────────────────────
    ; 1. Check if CPUID is supported (EFLAGS bit 21 toggle check)
    ; ─────────────────────────────────────────────────────────────────────────
    pushfd
    pop eax
    mov ecx, eax
    xor eax, (1 << 21)
    push eax
    popfd
    pushfd
    pop eax
    push ecx
    popfd
    xor eax, ecx
    test eax, (1 << 21)
    jz .cpu_is_32bit         ; Bit 21 didn't toggle -> No CPUID (< Pentium)

    ; ─────────────────────────────────────────────────────────────────────────
    ; 2. Probe CPUID Leaf 0 (Vendor String & Max Standard Leaf)
    ; ─────────────────────────────────────────────────────────────────────────
    xor eax, eax
    cpuid
    mov [BOOT_CPU_VENDOR], ebx
    mov [BOOT_CPU_VENDOR + 4], edx
    mov [BOOT_CPU_VENDOR + 8], ecx
    mov byte [BOOT_CPU_VENDOR + 12], 0

    cmp eax, 1
    jb .cpu_is_32bit

    ; ─────────────────────────────────────────────────────────────────────────
    ; 3. Probe CPUID Leaf 1 (Features & Signature)
    ; ─────────────────────────────────────────────────────────────────────────
    mov eax, 1
    cpuid
    mov [BOOT_CPU_SIGNATURE], eax
    mov [BOOT_CPU_FLAGS_EDX], edx

    ; Test PAE (bit 6 of EDX) - required for 64-bit paging
    test edx, (1 << 6)
    jz .cpu_is_32bit

    ; ─────────────────────────────────────────────────────────────────────────
    ; 4. Probe CPUID Leaf 0x80000000 (Max Extended Leaf)
    ; ─────────────────────────────────────────────────────────────────────────
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .cpu_is_32bit

    ; ─────────────────────────────────────────────────────────────────────────
    ; 5. Probe CPUID Leaf 0x80000001 (Extended Features / Long Mode)
    ; ─────────────────────────────────────────────────────────────────────────
    mov eax, 0x80000001
    cpuid
    mov [BOOT_CPU_EXT_EDX], edx

    ; Test Long Mode (EDX bit 29)
    test edx, (1 << 29)
    jz .cpu_is_32bit         ; 32-bit ONLY CPU (Limbo x86 / IA-32)

    ; Check if cmdline requests "arch_diag"
    mov esi, [MULTIBOOT_INFO_PTR]
    test esi, esi
    jz .proceed_64bit
    test dword [esi], (1 << 2) ; flags bit 2 = cmdline
    jz .proceed_64bit
    mov esi, [esi + 16]        ; cmdline pointer
    test esi, esi
    jz .proceed_64bit
    call check_arch_diag
    test eax, eax
    jnz .cpu_is_32bit

.proceed_64bit:
    mov dword [BOOT_IS_64BIT], 1

    ; ─────────────────────────────────────────────────────────────────────────
    ; 6. Set up Page Tables (4GB Identity Map + Higher-Half Kernel Mirror)
    ; ─────────────────────────────────────────────────────────────────────────
    mov edi, boot_pml4
    xor eax, eax
    mov ecx, 7168            ; 7 tables * 1024 dwords = 28672 bytes
    rep stosd

    ; PML4[0] -> PDPT (identity map)
    mov eax, boot_pdpt
    or eax, 0b11             ; Present + Writable
    mov [boot_pml4], eax

    ; PML4[511] -> same PDPT (higher-half: 0xFFFFFFFF80000000)
    mov [boot_pml4 + 511 * 8], eax

    ; PDPT[0..3] -> PD0..PD3
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

    ; PDPT[510] -> PD_high
    mov eax, boot_pd_high
    or eax, 0b11
    mov [boot_pdpt + 510 * 8], eax

    ; Fill 4 PDs (512 * 2MB entries each = 4GB total)
    xor ebx, ebx
.fill_pd_outer:
    mov esi, [pd_table + ebx * 4]
    xor ecx, ecx
.fill_pd_inner:
    mov eax, ebx
    shl eax, 9
    add eax, ecx
    shl eax, 21
    or eax, 0b10000011       ; Present + Writable + HugePage (2MB)

    mov [esi + ecx * 8], eax

    test ebx, ebx
    jnz .skip_high
    mov edx, eax
    ; If PGE is supported, add GLOBAL flag (bit 8)
    mov ebp, [BOOT_CPU_FLAGS_EDX]
    test ebp, (1 << 13)      ; PGE
    jz .no_global_flag
    or edx, (1 << 8)
.no_global_flag:
    mov [boot_pd_high + ecx * 8], edx
.skip_high:

    inc ecx
    cmp ecx, 512
    jne .fill_pd_inner

    inc ebx
    cmp ebx, 4
    jne .fill_pd_outer

    ; ─────────────────────────────────────────────────────────────────────────
    ; 7. Safe Dynamic Register Setup (CR4, CR3, EFER, CR0)
    ; ─────────────────────────────────────────────────────────────────────────
    ; CR4 setup: PAE is mandatory for long mode
    mov eax, cr4
    or eax, (1 << 5)         ; PAE bit 5

    ; PGE: only enable if Leaf 1 EDX bit 13 is set
    mov edx, [BOOT_CPU_FLAGS_EDX]
    test edx, (1 << 13)
    jz .skip_cr4_pge
    or eax, (1 << 7)         ; PGE bit 7
.skip_cr4_pge:

    ; FXSR: enable OSFXSR (bit 9) and OSXMMEXCPT (bit 10) if FXSR supported
    test edx, (1 << 24)
    jz .skip_cr4_fxsr
    or eax, (1 << 9) | (1 << 10)
.skip_cr4_fxsr:
    mov cr4, eax

    ; Load CR3 with PML4 base
    mov eax, boot_pml4
    mov cr3, eax

    ; EFER MSR 0xC0000080 setup
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)         ; LME (Long Mode Enable)

    ; Only set NXE (bit 11) if CPUID reports NX support
    mov edx, [BOOT_CPU_EXT_EDX]
    test edx, (1 << 20)
    jz .skip_efer_nxe
    or eax, (1 << 11)        ; NXE (No-Execute Enable)
.skip_efer_nxe:
    wrmsr

    ; CR0 setup: Enable Paging (PG) and Protection (PE), clear EM, set MP
    mov eax, cr0
    or eax, (1 << 31) | (1 << 0) ; PG | PE
    and eax, ~(1 << 2)       ; Clear EM (Emulation)
    or eax, (1 << 1)         ; Set MP (Monitor Coprocessor)
    mov cr0, eax

    ; Load 64-bit GDT and jump to 64-bit Long Mode
    lgdt [gdt64.pointer]
    jmp gdt64.code_segment:long_mode_start

; ─────────────────────────────────────────────────────────────────────────────
; 32-Bit Compatibility & Architecture Diagnostics Mode
; Enters if CPU is 32-bit only (Limbo x86 / i386) or if requested via cmdline
; ─────────────────────────────────────────────────────────────────────────────
.cpu_is_32bit:
    mov dword [BOOT_IS_64BIT], 0

    ; 1. Initialize Serial Port COM1 (0x3F8) for emulator logs
    call init_serial_32

    mov esi, str_banner
    call print_serial_32

    mov esi, str_detected_32bit
    call print_serial_32

    mov esi, str_cpu_vendor_hdr
    call print_serial_32
    mov esi, BOOT_CPU_VENDOR
    call print_serial_32
    mov esi, str_crlf
    call print_serial_32

    mov esi, str_limbo_guide
    call print_serial_32

    ; 2. Draw VGA 32-Bit Diagnostic Screen (0xB8000)
    call render_vga_32

    ; 3. Interactive polling loop for reboot or halt
.interactive_loop:
    ; Check PS/2 keyboard status
    in al, 0x64
    test al, 1
    jz .check_serial
    in al, 0x60              ; Read scancode

    cmp al, 0x13             ; 'R' key (Reboot)
    je .do_reboot
    cmp al, 0x23             ; 'H' key (Halt)
    je .do_halt

.check_serial:
    mov dx, 0x3FD
    in al, dx
    test al, 1               ; Data Ready
    jz .idle_loop
    mov dx, 0x3F8
    in al, dx
    cmp al, 'r'
    je .do_reboot
    cmp al, 'R'
    je .do_reboot
    cmp al, 'h'
    je .do_halt
    cmp al, 'H'
    je .do_halt

.idle_loop:
    hlt
    jmp .interactive_loop

.do_reboot:
    mov esi, str_rebooting
    call print_serial_32

    ; ACPI/PCI port 0xCF9 reset
    mov dx, 0xCF9
    mov al, 0x02
    out dx, al
    mov al, 0x06
    out dx, al

    ; PS/2 controller pulse reset
    mov al, 0xFE
    out 0x64, al

    ; Fallback: Triple fault
    lidt [zero_idt]
    int 3
    hlt

.do_halt:
    mov esi, str_halted
    call print_serial_32
    cli
.halt_loop:
    hlt
    jmp .halt_loop

; ─────────────────────────────────────────────────────────────────────────────
; 32-bit Helper Functions
; ─────────────────────────────────────────────────────────────────────────────
check_arch_diag:
.scan_loop:
    mov al, [esi]
    test al, al
    jz .not_found
    cmp al, 'a'
    jne .next_char
    cmp dword [esi], 'arch'
    jne .next_char
    cmp dword [esi + 4], '_dia'
    jne .next_char
    cmp byte [esi + 8], 'g'
    jne .next_char
    mov eax, 1
    ret
.next_char:
    inc esi
    jmp .scan_loop
.not_found:
    xor eax, eax
    ret

init_serial_32:
    mov dx, 0x3F9
    xor al, al
    out dx, al          ; Disable interrupts
    mov dx, 0x3FB
    mov al, 0x80
    out dx, al          ; Enable DLAB
    mov dx, 0x3F8
    mov al, 0x01
    out dx, al          ; 115200 baud
    mov dx, 0x3F9
    xor al, al
    out dx, al
    mov dx, 0x3FB
    mov al, 0x03
    out dx, al          ; 8 bits, no parity, 1 stop bit
    mov dx, 0x3FC
    mov al, 0x0B
    out dx, al          ; RTS/DSR set
    ret

print_serial_32:
.next_char:
    lodsb
    test al, al
    jz .done
    mov bl, al
.wait_tx:
    mov dx, 0x3FD
    in al, dx
    test al, 0x20
    jz .wait_tx
    mov al, bl
    mov dx, 0x3F8
    out dx, al
    jmp .next_char
.done:
    ret

render_vga_32:
    ; Clear screen: 80 * 25 cells = 2000 cells (White on Blue = 0x1F)
    mov edi, 0xB8000
    mov ecx, 2000
    mov ax, 0x1F20          ; Space with attribute 0x1F
.clr:
    mov [edi], ax
    add edi, 2
    loop .clr

    ; Title bar: Row 0 (White on Red = 0x4F)
    mov edi, 0xB8000
    mov ecx, 80
    mov ax, 0x4F20
.clr_bar:
    mov [edi], ax
    add edi, 2
    loop .clr_bar

    mov edi, 0xB8000 + 4
    mov ah, 0x4F
    mov esi, vga_title
    call vga_print_str_32

    ; Alert: Row 2 & 3 (Yellow on Blue = 0x1E)
    mov edi, 0xB8000 + (2 * 160)
    mov ah, 0x1E
    mov esi, vga_alert
    call vga_print_str_32

    mov edi, 0xB8000 + (3 * 160)
    mov ah, 0x1B            ; Cyan on Blue
    mov esi, vga_subalert
    call vga_print_str_32

    ; CPU Details: Row 5-8
    mov edi, 0xB8000 + (5 * 160)
    mov ah, 0x1F            ; White on Blue
    mov esi, vga_hw_hdr
    call vga_print_str_32

    mov edi, 0xB8000 + (6 * 160)
    mov ah, 0x17            ; Light Gray on Blue
    mov esi, vga_vendor_lbl
    call vga_print_str_32
    mov ah, 0x1E            ; Yellow on Blue
    mov esi, BOOT_CPU_VENDOR
    call vga_print_str_32

    mov edi, 0xB8000 + (7 * 160)
    mov ah, 0x17
    mov esi, vga_arch_lbl
    call vga_print_str_32

    mov edi, 0xB8000 + (8 * 160)
    mov ah, 0x1C            ; Light Red on Blue
    mov esi, vga_longmode_lbl
    call vga_print_str_32

    ; Limbo Setup Guide: Row 10-16
    mov edi, 0xB8000 + (10 * 160)
    mov ah, 0x1A            ; Light Green on Blue
    mov esi, vga_guide_hdr
    call vga_print_str_32

    mov edi, 0xB8000 + (11 * 160)
    mov ah, 0x1F
    mov esi, vga_step1
    call vga_print_str_32

    mov edi, 0xB8000 + (12 * 160)
    mov ah, 0x1F
    mov esi, vga_step2
    call vga_print_str_32

    mov edi, 0xB8000 + (13 * 160)
    mov ah, 0x1E
    mov esi, vga_step3
    call vga_print_str_32

    mov edi, 0xB8000 + (14 * 160)
    mov ah, 0x1E
    mov esi, vga_step4
    call vga_print_str_32

    mov edi, 0xB8000 + (15 * 160)
    mov ah, 0x1E
    mov esi, vga_step5
    call vga_print_str_32

    mov edi, 0xB8000 + (16 * 160)
    mov ah, 0x1A
    mov esi, vga_step6
    call vga_print_str_32

    ; Controls: Row 19 & 20
    mov edi, 0xB8000 + (19 * 160)
    mov ah, 0x1F
    mov esi, vga_ctrl_hdr
    call vga_print_str_32

    mov edi, 0xB8000 + (20 * 160)
    mov ah, 0x1E
    mov esi, vga_ctrl_keys
    call vga_print_str_32
    ret

vga_print_str_32:
.next:
    lodsb
    test al, al
    jz .done
    mov [edi], al
    mov [edi + 1], ah
    add edi, 2
    jmp .next
.done:
    ret

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
    ; Set up 64-bit data segments
    mov ax, gdt64.data_segment
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Set up stack
    mov rsp, stack_top

    ; Jump to Rust _start (higher-half linked)
    mov rax, _start
    jmp rax
