#ifndef GDT_H
#define GDT_H

#include <stdint.h>

/* GDT Segment Selectors */
#define GDT_KERNEL_CODE 0x08
#define GDT_KERNEL_DATA 0x10
#define GDT_USER_DATA   0x18
#define GDT_USER_CODE   0x20
#define GDT_TSS         0x28

/* For switching to Ring 3 */
#define GDT_USER_CODE_RPL3 (GDT_USER_CODE | 3)
#define GDT_USER_DATA_RPL3 (GDT_USER_DATA | 3)

struct gdt_desc {
    uint16_t limit;
    uint64_t base;
} __attribute__((packed));

struct gdt_entry {
    uint16_t limit_low;
    uint16_t base_low;
    uint8_t  base_middle;
    uint8_t  access;
    uint8_t  granularity;
    uint8_t  base_high;
} __attribute__((packed));

/* TSS in long mode - extended to span 2 GDT entries */
struct tss_entry {
    uint32_t reserved0;
    uint64_t rsp0;          /* Stack pointer for Ring 0 */
    uint64_t rsp1;          /* Stack pointer for Ring 1 */
    uint64_t rsp2;          /* Stack pointer for Ring 2 */
    uint64_t reserved1;
    uint64_t ist1;          /* Interrupt Stack Table 1 */
    uint64_t ist2;
    uint64_t ist3;
    uint64_t ist4;
    uint64_t ist5;
    uint64_t ist6;
    uint64_t ist7;
    uint64_t reserved2;
    uint16_t reserved3;
    uint16_t iopb_offset;   /* I/O Permission Bit Map Offset */
} __attribute__((packed));

void gdt_init(void);
void tss_set_rsp0(uint64_t rsp0);

#endif
