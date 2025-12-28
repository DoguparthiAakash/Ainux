[BITS 16]
[ORG 0x7C00]

start:
    ; Initialize segments
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti

    ; Print "Booting..." message
    mov si, msg_booting
    call print_string

    ; Load Stage 2
    ; We'll load 50 sectors just to be safe (Make sure Stage 2 is < 25KB)
    ; Load to 0x1000 (ES:BX = 0x0000:0x1000)
    
    mov bx, 0x1000      ; Destination address
    mov ah, 0x02        ; INT 13h - Read Sectors
    mov al, 50          ; Number of sectors to read
    mov ch, 0           ; Cylinder 0
    mov dh, 0           ; Head 0
    mov cl, 2           ; Sector 2 (Sector 1 is MBR)
    mov dl, 0x80        ; First hard drive (Simulated by QEMU)
    int 0x13
    jc disk_error       ; Jump if Carry Flag set (ERROR)

    ; Jump to Stage 2
    jmp 0x0000:0x1000

print_string:
    lodsb
    or al, al
    jz .done
    mov ah, 0x0E
    int 0x10
    jmp print_string
.done:
    ret

disk_error:
    mov si, msg_error
    call print_string
    cli
    hlt

msg_booting db "Booting Ainux...", 0x0D, 0x0A, 0
msg_error   db "Disk Read Error!", 0
    
times 510-($-$$) db 0   ; Padding
dw 0xAA55               ; Boot Signature
