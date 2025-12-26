#include "gdt.h"

struct gdt_entry gdt[3];
struct gdt_desc gdtr;

static void gdt_set_gate(int num, uint64_t base, uint64_t limit, uint8_t access, uint8_t gran) {
    gdt[num].base_low = (base & 0xFFFF);
    gdt[num].base_middle = (base >> 16) & 0xFF;
    gdt[num].base_high = (base >> 24) & 0xFF;

    gdt[num].limit_low = (limit & 0xFFFF);
    gdt[num].granularity = ((limit >> 16) & 0x0F);

    gdt[num].granularity |= (gran & 0xF0);
    gdt[num].access = access;
}

void gdt_init(void) {
    /* 0: Null Descriptor */
    gdt_set_gate(0, 0, 0, 0, 0);

    /* 1: Kernel Code Segment. 
       Access: Present(1) | Ring0(00) | Code/Data(1) | Executable(1) | Readable(1) | Accessed(0) -> 10011010b -> 0x9A
       Flags: LongMode(1) | 4KB Granularity(1) -> 10100000 -> 0xA0 (Lower nibble is high limit)
       Limit: 0 (Ignored in 64-bit mode basically, but usually set to 0 or max)
    */
    gdt_set_gate(1, 0, 0, 0x9A, 0xA0);

    /* 2: Kernel Data Segment.
       Access: Present(1) | Ring0(00) | Code/Data(1) | Writeable(1) | Accessed(0) -> 10010010b -> 0x92
       Flags: LongMode(0)? Actually for data, LongMode bit doesn't exist same way, usually 0xC0 or 0xA0. 
       Let's use 0xC0 (4KB, 32-bit protected) or just 0 in 64-bit? 
       Limine usually sets it up, but let's replicate standard 64-bit data.
       Access: 0x92. Gran: 0x00 is fine usually or 0xC0.
    */
    gdt_set_gate(2, 0, 0, 0x92, 0xC0); // 0xC0? Long mode data is usually just ignored.

    gdtr.limit = sizeof(gdt) - 1;
    gdtr.base = (uint64_t)&gdt;

    /* Load GDT */
    __asm__ volatile ("lgdt %0" : : "m"(gdtr));

    /* Reload Segments */
    /* Return to 64-bit mode by pushing code seg and returning? */
    /* Or just simple movs for data segments, and a far jump/ret for CS */
    
    /* Reload Data Segments (0x10 is offset of entry 2) */
    __asm__ volatile (
        "mov $0x10, %ax \n"
        "mov %ax, %ds \n"
        "mov %ax, %es \n"
        "mov %ax, %fs \n"
        "mov %ax, %gs \n"
        "mov %ax, %ss \n"
    );

    /* Reload Code Segment (0x08 is offset of entry 1) */
    /* push 0x08, push return_address, retfq (far return) */
    __asm__ volatile (
        "pushq $0x08 \n"
        "leaq 1f(%%rip), %%rax \n"
        "pushq %%rax \n"
        "lretq \n"
        "1: \n"
        : : : "rax", "memory"
    );
}
