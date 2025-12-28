#ifndef BOOT_INFO_H
#define BOOT_INFO_H

#include <stdint.h>

struct e820_entry {
    uint64_t base;
    uint64_t length;
    uint32_t type;
    uint32_t acpi;
} __attribute__((packed));

typedef struct {
    uint64_t framebuffer_addr;
    uint64_t framebuffer_width;
    uint64_t framebuffer_height;
    uint64_t framebuffer_pitch;
    uint64_t framebuffer_bpp;
    void* memory_map;
    uint64_t memory_map_entries;
    void* initrd_addr;
    uint64_t initrd_size;
} boot_info_t;

extern boot_info_t *g_boot_info;

#endif
