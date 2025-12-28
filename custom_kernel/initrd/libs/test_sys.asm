; Test Advanced Syscalls (Graphics + Sleep)

; 1. Draw Blue Rect at (100,100) 200x200
MOV RAX, 400      ; SYS_DRAW_RECT
MOV RDI, 100      ; x
MOV RSI, 100      ; y
MOV RDX, 200      ; w
MOV R10, 200      ; h
MOV R8, 255       ; color (Blue: 0x0000FF, but raw int 255 is blue in RGB? depends on format. Assuming XRGB, 255 is blue)
INT 128

; 2. Sleep 1000ms
MOV RAX, 35       ; SYS_SLEEP
MOV RDI, 1000     ; ms
INT 128

; 3. Draw Green Rect at (150,150) 100x100
MOV RAX, 400
MOV RDI, 150
MOV RSI, 150
MOV RDX, 100
MOV R10, 100
MOV R8, 65280     ; 0x00FF00 (Green)
INT 128

; 4. Sleep 1000ms
MOV RAX, 35
MOV RDI, 1000
INT 128

; 5. Exit
MOV RAX, 60       ; SYS_EXIT
MOV RDI, 0
INT 128
