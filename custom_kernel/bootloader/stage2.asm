[BITS 16]
[ORG 0x1000]

stage2_entry:
    ; Init Serial for Debug
    call init_serial
    mov al, '1'
    call print_char_serial

    ; 1. Enable A20 Line (Robust)
    call check_a20
    je .a20_success
    
    ; Try BIOS
    mov ax, 0x2401
    int 0x15
    call check_a20
    je .a20_success
    
    ; Try Fast A20
    in al, 0x92
    or al, 2
    out 0x92, al
    call check_a20
    je .a20_success
    
    ; Failed
    mov al, 'F'
    call print_char_serial
    cli
    hlt

.a20_success:
    mov al, '2'
    call print_char_serial
    jmp get_map_start

check_a20:
    push ds
    push es
    push di
    push si
    
    xor ax, ax
    mov ds, ax
    not ax
    mov es, ax      ; ES = FFFF
    
    mov di, 0x0500
    mov si, 0x0510
    
    mov al, [ds:di]
    push ax
    
    mov al, 0x00
    mov [ds:di], al
    mov al, 0xFF
    mov [es:si], al
    cmp byte [ds:di], 0xFF
    
    pop ax
    mov [ds:di], al
    
    pop si
    pop di
    pop es
    pop ds
    
    je .a20_off_ret ; If equal, aliasing occurred (A20 OFF)
    
    ; A20 ON
    xor ax, ax      ; ZF=1
    ret

.a20_off_ret:
    or ax, 1        ; ZF=0
    ret

get_map_start:

    ; 2. Get Memory Map (E820)
    ; Store at 0x8000 (temporary buffer)
    ; We will count entries in BP
    mov di, 0x8004          ; Offset 4 to leave room for count
    xor ebx, ebx
    xor bp, bp
    mov edx, 0x534D4150     ; 'SMAP'
    mov eax, 0xE820
    mov ecx, 24             ; Entry size
    int 0x15
    jc .e820_failed

.e820_loop:
    inc bp
    add di, 24
    mov eax, 0xE820
    mov ecx, 24
    int 0x15
    jc .e820_done
    test ebx, ebx
    jnz .e820_loop

.e820_done:
    mov [0x8000], bp        ; Store count at 0x8000
    jmp .vbe_setup

.e820_failed:
    ; Handle error (halt)
    mov al, 'E'
    call print_char_serial
    cli
    hlt

.vbe_setup:
    mov al, '3'
    call print_char_serial

    ; 3. Setup VBE
    ; Get Info
    mov ax, 0x4F01
    mov cx, 0x118           ; Mode 1024x768x24/32
    mov di, 0x9000          ; Buffer for Mode Info
    int 0x10
    cmp ax, 0x004F
    jne .vbe_fail

    ; Set Mode
    mov ax, 0x4F02
    mov bx, 0x118 | 0x4000  ; Mode | LFB
    int 0x10
    cmp ax, 0x004F
    jne .vbe_fail
    
    jmp .load_kernel

.vbe_fail:
    ; Non-fatal VBE failure (headless mode?)
    mov al, 'V'
    call print_char_serial
    ; Clear 0x9000 buffer to 0 to indicate no VBE
    push di
    push cx
    push ax
    xor ax, ax
    mov di, 0x9000
    mov cx, 256
    rep stosw
    pop ax
    pop cx
    pop di
    ; Continue to load kernel

.load_kernel:
    mov al, '4'
    call print_char_serial

    ; 4. Load Kernel to 1MB (0x100000)
    ; Since we are in Real Mode, we can't access > 1MB easily without Unreal Mode.
    ; But I'll use a loop: Read to 0xC000 (buffer), enable Unreal Mode (Big Real Mode), copy to 1MB+.

    ; Now we can write to > 1MB using 32-bit address prefix
    ; Load Kernel logic
    
    mov dword [current_sector], 51
    mov word [sectors_left], 200    ; Load 100KB

.load_loop:
    cmp word [sectors_left], 0
    je switch_pmode
    
    ; Ensure DS=0, ES=0 for BIOS
    xor ax, ax
    mov ds, ax
    mov es, ax
    
    ; Read 1 sector to buffer 0xC000
    mov ah, 0x02
    mov al, 1
    mov ch, 0
    mov dh, 0
    
    mov si, dap             ; Disk Address Packet
    mov eax, [current_sector]
    mov [dap_lba], eax
    mov ah, 0x42
    mov dl, 0x80            ; First HDD
    int 0x13
    jc .disk_error
    
    ; Enable Unreal Mode on FS (Per Sector)
    cli
    lgdt [gdt_descriptor]
    mov eax, cr0
    or al, 1
    mov cr0, eax
    jmp $+2
    mov bx, 0x10
    mov fs, bx              ; Load 4GB limit into FS
    mov eax, cr0
    and al, 0xFE
    mov cr0, eax
    sti
    
    ; Copy using FS (Unreal)
    mov esi, 0xC000
    mov edi, [kernel_load_addr]
    
    mov ecx, 128            ; 512 bytes / 4
    
    .copy_loop:
        mov eax, [ds:esi]
        mov [fs:edi], eax   ; Write to high mem using FS
        add esi, 4
        add edi, 4
        loop .copy_loop
        
    mov edi, [kernel_load_addr]
    add edi, 512
    mov [kernel_load_addr], edi
    
    inc dword [current_sector]
    dec word [sectors_left]
    
    jmp .load_loop

.copy_fail:
    mov al, 'C'
    call print_char_serial
    cli
    hlt

.disk_error:
    mov al, 'D'
    call print_char_serial
    cli
    hlt
    
init_serial:
    mov dx, 0x3f8 + 1
    mov al, 0x00
    out dx, al
    
    mov dx, 0x3f8 + 3
    mov al, 0x80
    out dx, al
    
    mov dx, 0x3f8 + 0
    mov al, 0x03
    out dx, al
    
    mov dx, 0x3f8 + 1
    mov al, 0x00
    out dx, al
    
    mov dx, 0x3f8 + 3
    mov al, 0x03
    out dx, al
    
    mov dx, 0x3f8 + 2
    mov al, 0xc7
    out dx, al
    
    mov dx, 0x3f8 + 4
    mov al, 0x0b
    out dx, al
    ret

print_char_serial:
    push dx
    push ax
    mov dx, 0x3f8
    out dx, al
    pop ax
    pop dx
    ret

switch_pmode:
    ; Switch to Protected Mode
    cli
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:pmode_entry

align 4
dap:
    db 0x10
    db 0
    dw 1                    ; 1 sector
    dw 0xC000               ; Offset
    dw 0                    ; Segment
dap_lba:
    dd 0
    dd 0

align 8
gdt_int15:
    ; 0: Null
    dq 0
    ; 8: Reserved
    dq 0
    ; 16: Source (0:C000). Limit=FFFF. Access=93.
    ; Limit[0:15]=FFFF, Base[0:15]=C000, Base[16:23]=00, Access=93, Lim[16:19]=0, Base[24:31]=0
    ; 00 00 93 C0 00 C0 FF FF
    db 0xFF, 0xFF, 0x00, 0xC0, 0x00, 0x93, 0x00, 0x00
    ; 24: Dest (Template)
    db 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x93, 0x00, 0x00
    ; 32: Padding
    dq 0
    dq 0

current_sector dd 51    ; LBA 51 (immediately follows 50 sectors of Stage 2 + 1 MBR)
kernel_load_addr dd 0x100000
sectors_left dw 0

; GDT
gdt_start:
    dq 0x0000000000000000   ; Null
    dq 0x00CF9A000000FFFF   ; Code (32-bit)
    dq 0x00CF92000000FFFF   ; Data (32-bit)
gdt_end:
gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dd gdt_start

[BITS 32]
pmode_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x90000

    ; ... (Keep rest of pmode_entry until long_mode_entry) ...

    ; 6. Setup Long Mode Paging
    ; We need a Page Map Level 4 (PML4), PDPT, PD, PT.
    ; Map 0-2MB Identity.
    ; Map Higher Half (0xFFFFFFFF80000000) -> 0x100000.
    
    ; Clear Memory for Page Tables (use 0x10000-0x15000)
    mov edi, 0x10000
    mov ecx, 0x5000 / 4
    xor eax, eax
    rep stosd
    
    ; PML4 at 0x10000
    ; PDPT at 0x11000
    ; PD_LOW at 0x12000
    ; PD_HIGH at 0x13000
    
    ; Link PML4[0] -> PDPT (Identity Map)
    mov dword [0x10000], 0x11003 ; Present | RW
    
    ; Link PML4[511] -> PDPT (Higher Half)
    mov dword [0x10000 + 511*8], 0x11003
    
    ; Link PDPT[0] -> PD_LOW (Identity Map)
    mov dword [0x11000], 0x12003
    
    ; Link PDPT[510] -> PD_HIGH (Higher Half Map)
    mov dword [0x11000 + 510*8], 0x13003
    
    ; Identity Map First 2MB in PD_LOW -> Physical 0
    mov dword [0x12000], 0x00000083 ; Present | RW | HugePage
    
    ; Map Higher Half (First 2MB) in PD_HIGH -> Physical 0x000000 (Because 2MB pages must be 2MB aligned!)
    ; So Virtual ...80000000 -> Physical 0.
    ; Virtual ...80100000 -> Physical 1MB. (Where kernel is)
    mov dword [0x13000], 0x00000083 ; Present | RW | HugePage 
    
    ; Enable PAE
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax
    
    ; Load CR3
    mov eax, 0x10000
    mov cr3, eax
    
    ; Enable Long Mode (EFER.LME)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr
    
    ; Enable Paging
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax
    
    ; Jump to 64-bit Long Mode
    ; Need GDT with 64-bit descriptor
    lgdt [gdt64_descriptor]
    jmp 0x08:long_mode_entry

[BITS 64]
long_mode_entry:
    ; Setup Segments
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    
    ; Prepare Boot Info Struct
    mov rdi, 0x7000         ; BOOT_INFO stored here
    
    ; Copy VBE Info to boot_info
    ; FB Addr
    mov eax, [0x9028]
    mov [rdi], rax
    
    ; Width (16-bit at 0x9012)
    xor rax, rax
    mov ax, [0x9012]
    mov [rdi+8], rax
    
    ; Height (16-bit at 0x9014)
    xor rax, rax
    mov ax, [0x9014]
    mov [rdi+16], rax
    
    ; Pitch (16-bit at 0x9010)
    xor rax, rax
    mov ax, [0x9010]
    mov [rdi+24], rax
    
    ; BPP (8-bit at 0x9019)
    xor rax, rax
    mov al, [0x9019]
    mov [rdi+32], rax
    
    ; Pass E820 map (Count at 0x8000, buffer at 0x8004)
    mov qword [rdi+40], 0x8004 ; Memory Map Addr
    xor rax, rax
    mov ax, [0x8000]
    mov [rdi+48], rax          ; Count
    
    ; InitRD (TODO: Actually load it)
    mov qword [rdi+56], 0
    mov qword [rdi+64], 0
    
    ; Call Kernel
    mov rax, 0xffffffff80100000
    call rax
    
    cli
    hlt

; 64-bit GDT
gdt64_start:
    dq 0x0000000000000000   ; Null
    dq 0x00209A0000000000   ; Code (64-bit, Ring 0) -> 0x08
    dq 0x0000920000000000   ; Data (64-bit, Ring 0) -> 0x10
gdt64_end:
gdt64_descriptor:
    dw gdt64_end - gdt64_start - 1
    dd gdt64_start

times 50*512 - ($ - $$) db 0  ; Padding to 50 sectors
