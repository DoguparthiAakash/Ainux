ENTRY(_start_multiboot)

SECTIONS
{
    . = 1M;

    /* 1. Identity mapped boot section (Low Memory) */
    .boot : {
        KEEP(*(.multiboot))
        src/asm/boot.o(.text .data .rodata)
    }

    /* 2. Page tables and boot stack (Low Memory, excluded from Rust BSS) */
    .boot_bss ALIGN(4K) : {
        src/asm/boot.o(.bss COMMON)
    }

    . = ALIGN(4K);
    . += 0xFFFFFFFF80000000;

    /* 3. Higher Half Kernel */
    .text ALIGN(4K) : AT(ADDR(.text) - 0xFFFFFFFF80000000) {
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - 0xFFFFFFFF80000000) {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) : AT(ADDR(.data) - 0xFFFFFFFF80000000) {
        *(.data .data.*)
    }

    .bss ALIGN(4K) (NOLOAD) : AT(ADDR(.bss) - 0xFFFFFFFF80000000) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    .got ALIGN(4K) : AT(ADDR(.got) - 0xFFFFFFFF80000000) {
        *(.got .got.*)
    }

    /DISCARD/ : {
        *(.eh_frame .eh_frame_hdr)
        *(.note .note.*)
        *(.comment)
    }
}

