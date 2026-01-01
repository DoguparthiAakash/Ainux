#ifndef BOOT_INFO_H
#define BOOT_INFO_H

#include <stdint.h>

/* Multiboot2 Memory Map Type */
#define MULTIBOOT_MEMORY_AVAILABLE        1
#define MULTIBOOT_MEMORY_RESERVED         2
#define MULTIBOOT_MEMORY_ACPI_RECLAIMABLE 3
#define MULTIBOOT_MEMORY_NVS              4
#define MULTIBOOT_MEMORY_BADRAM           5

struct multiboot_mmap_entry {
    uint64_t addr;
    uint64_t len;
    uint32_t type;
    uint32_t zero;
} __attribute__((packed));

typedef struct boot_info {
    uint64_t framebuffer_addr;
    uint64_t framebuffer_width;
    uint64_t framebuffer_height;
    uint64_t framebuffer_pitch;
    uint64_t framebuffer_bpp;
    uint64_t memory_map_addr;     /* Physical address of the array of struct multiboot_mmap_entry */
    uint64_t memory_map_entries;  /* Number of entries */
    uint64_t initrd_addr;
    uint64_t initrd_size;
    uint64_t rsdp; /* Root System Description Pointer (ACPI) */
} __attribute__((packed)) boot_info_t;

extern struct boot_info g_boot_info;
extern uint64_t g_hhdm_offset;

#endif
