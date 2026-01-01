/* Tell the linker that we want an x86_64 ELF64 output file */
OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)

/* We want the symbol _start to be our entry point */
ENTRY(_start)

/* Define the memory layout */
SECTIONS
{
    /* Load the kernel at 1MB */
    . = 1M;

    .boot :
    {
        /* Ensure the Multiboot header comes first */
        KEEP(*(.multiboot_header))
        /* Add Limine requests */
        KEEP(*(.limine_requests_v2))
        KEEP(*(.limine_requests_v1))
        *(.boot.text)
        *(.boot.data)
        *(.boot.bss)
    } :boot

    /* Move to higher half for the rest of the kernel */
    . += 0xffffffff80000000;

    .text : AT(ADDR(.text) - 0xffffffff80000000)
    {
        *(.text .text.*)
    } :text

    .rodata : AT(ADDR(.rodata) - 0xffffffff80000000)
    {
        *(.rodata .rodata.*)
    } :text

    .data : AT(ADDR(.data) - 0xffffffff80000000)
    {
        *(.data .data.*)
    } :text

    .bss : AT(ADDR(.bss) - 0xffffffff80000000)
    {
        *(COMMON)
        *(.bss .bss.*)
        . = ALIGN(4096);
        _kernel_end = .;
    } :text

    /* Discard unused sections */
    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
        *(.requests*) /* Discard Limine requests */
    }
}

PHDRS
{
    boot PT_LOAD FLAGS(7); /* RWE */
    text PT_LOAD FLAGS(7); /* RWE */
}


