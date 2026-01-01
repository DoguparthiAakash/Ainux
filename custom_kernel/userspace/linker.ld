ENTRY(_start)

SECTIONS
{
    . = 0x400000; /* Standard database load address */
    
    .text : {
        *(.text)
    }
    
    .rodata : {
        *(.rodata)
    }
    
    .data : {
        *(.data)
    }
    
    .bss : {
        *(.bss)
    }
}
