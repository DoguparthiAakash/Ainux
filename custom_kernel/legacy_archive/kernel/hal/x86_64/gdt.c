#include "gdt.h"
#include <stddef.h>

/* 7 entries: Null, KCode, KData, UCode, UData, TSS (low), TSS (high) */
struct gdt_entry gdt[7];
struct gdt_desc gdtr;

/* TSS instance */
static struct tss_entry tss __attribute__((aligned(16)));

/* Kernel interrupt/syscall stack */
static uint8_t kernel_stack[16384] __attribute__((aligned(16)));

static void gdt_set_gate(int num, uint64_t base, uint64_t limit, uint8_t access, uint8_t gran) {
    gdt[num].base_low = (base & 0xFFFF);
    gdt[num].base_middle = (base >> 16) & 0xFF;
    gdt[num].base_high = (base >> 24) & 0xFF;

    gdt[num].limit_low = (limit & 0xFFFF);
    gdt[num].granularity = ((limit >> 16) & 0x0F);

    gdt[num].granularity |= (gran & 0xF0);
    gdt[num].access = access;
}

/* Set TSS descriptor - TSS in long mode takes 16 bytes (2 GDT entries) */
static void gdt_set_tss(int num, uint64_t base, uint32_t limit) {
    /* Low 8 bytes */
    gdt[num].limit_low = limit & 0xFFFF;
    gdt[num].base_low = base & 0xFFFF;
    gdt[num].base_middle = (base >> 16) & 0xFF;
    gdt[num].access = 0x89; /* Present, 64-bit TSS (Available) */
    gdt[num].granularity = ((limit >> 16) & 0x0F); /* Limit bits 16-19, no flags in high nibble for TSS */
    gdt[num].base_high = (base >> 24) & 0xFF;
    
    /* High 8 bytes - stored in next GDT entry */
    uint32_t base_upper = (base >> 32) & 0xFFFFFFFF;
    gdt[num + 1].limit_low = base_upper & 0xFFFF;
    gdt[num + 1].base_low = (base_upper >> 16) & 0xFFFF;
    gdt[num + 1].base_middle = 0;
    gdt[num + 1].access = 0;
    gdt[num + 1].granularity = 0;
    gdt[num + 1].base_high = 0;
}

void tss_set_rsp0(uint64_t rsp0) {
    tss.rsp0 = rsp0;
}

void gdt_init(void) {
    /* 0: Null Descriptor */
    gdt_set_gate(0, 0, 0, 0, 0);

    /* 1: Kernel Code Segment (0x08)
       Access: Present(1) | Ring0(00) | Code/Data(1) | Executable(1) | Readable(1) -> 0x9A
       Flags: LongMode(1) | 4KB Granularity(1) -> 0xA0
    */
    gdt_set_gate(1, 0, 0, 0x9A, 0xA0);

    /* 2: Kernel Data Segment (0x10)
       Access: Present(1) | Ring0(00) | Code/Data(1) | Writeable(1) -> 0x92
    */
    gdt_set_gate(2, 0, 0, 0x92, 0x00);
    
    /* 3: User Data Segment (0x18)
       Access: Present(1) | Ring3(11) | Code/Data(1) | Writeable(1) -> 0xF2
    */
    gdt_set_gate(3, 0, 0, 0xF2, 0x00);

    /* 4: User Code Segment (0x20)
       Access: Present(1) | Ring3(11) | Code/Data(1) | Executable(1) | Readable(1) -> 0xFA
       Flags: LongMode(1) -> 0xA0
    */
    gdt_set_gate(4, 0, 0, 0xFA, 0xA0);
    
    /* Initialize TSS */
    for (size_t i = 0; i < sizeof(tss); i++) {
        ((uint8_t *)&tss)[i] = 0;
    }
    tss.rsp0 = (uint64_t)&kernel_stack[sizeof(kernel_stack)]; /* Top of kernel stack */
    tss.iopb_offset = sizeof(struct tss_entry); /* No I/O bitmap */
    
    /* 5-6: TSS Descriptor (0x28) - spans 2 entries in long mode */
    gdt_set_tss(5, (uint64_t)&tss, sizeof(struct tss_entry) - 1);

    gdtr.limit = sizeof(gdt) - 1;
    gdtr.base = (uint64_t)&gdt;

    /* Load GDT */
    __asm__ volatile ("lgdt %0" : : "m"(gdtr));

    /* Reload Segments with kernel selectors (0x10 is offset of entry 2) */
    __asm__ volatile (
        "mov $0x10, %%ax \n"
        "mov %%ax, %%ds \n"
        "mov %%ax, %%es \n"
        "mov %%ax, %%fs \n"
        "mov %%ax, %%gs \n"
        "mov %%ax, %%ss \n"
        : : : "ax"
    );

    /* Reload Code Segment (0x08 is offset of entry 1) */
    __asm__ volatile (
        "pushq $0x08 \n"
        "leaq 1f(%%rip), %%rax \n"
        "pushq %%rax \n"
        "lretq \n"
        "1: \n"
        : : : "rax", "memory"
    );
    
    /* Load TSS */
    __asm__ volatile (
        "mov %0, %%ax \n"
        "ltr %%ax \n"
        : : "i"(GDT_TSS) : "ax"
    );
}
