ENTRY(_start_multiboot)

PHDRS
{
    boot PT_LOAD FLAGS(5);
    text PT_LOAD FLAGS(5);
    data PT_LOAD FLAGS(6);
}

SECTIONS
{
    . = 1M;

    .boot : {
        KEEP(*(.multiboot))
        src/asm/boot.o(.text)
        src/asm/boot.o(.data)
        src/asm/boot.o(.rodata)
    } :boot

    .boot_bss (NOLOAD) : {
        src/asm/boot.o(.bss)
        src/asm/boot.o(COMMON)
    } :boot

    . += 0xFFFFFFFF80000000;

    .text ALIGN(4K) : AT(ADDR(.text) - 0xFFFFFFFF80000000) {
        *(.text .text.*)
    } :text

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - 0xFFFFFFFF80000000) {
        *(.rodata .rodata.*)
    } :data

    .data ALIGN(4K) : AT(ADDR(.data) - 0xFFFFFFFF80000000) {
        *(.data .data.*)
    } :data

    .bss ALIGN(4K) : AT(ADDR(.bss) - 0xFFFFFFFF80000000) {
        *(.bss .bss.*)
        *(COMMON)
    } :data

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }

    .got ALIGN(4K) : AT(ADDR(.got) - 0xFFFFFFFF80000000) {
        *(.got .got.*)
    } :data
}

