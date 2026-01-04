#include "pmm.h"
#include "../boot_info.h"

extern void kprint(const char *msg);
extern void early_serial_puts(const char *s); /* Add prototype */
extern void early_serial_hex64(uint64_t v);   /* Add prototype */

/* Bitmap to track page allocation */
static uint8_t *bitmap = NULL;
static uint64_t total_pages = 0;
static uint64_t used_pages = 0;
static uint64_t bitmap_size = 0;
static uint64_t pmm_usable_memory = 0;

/* Helper: Set bit in bitmap */
static void bitmap_set(uint64_t bit) {
    bitmap[bit / 8] |= (1 << (bit % 8));
}

/* Helper: Clear bit in bitmap */
static void bitmap_clear(uint64_t bit) {
    bitmap[bit / 8] &= ~(1 << (bit % 8));
}

/* Helper: Test bit in bitmap */
static int bitmap_test(uint64_t bit) {
    return bitmap[bit / 8] & (1 << (bit % 8));
}

void pmm_init(struct boot_info *info) {
    struct multiboot_mmap_entry *entries = (struct multiboot_mmap_entry *)info->memory_map_addr;
    uint64_t count = info->memory_map_entries;

    if (entries == NULL || count == 0) {
        kprint("[PMM] No memory map available!\n");
        return;
    }

    /* Find highest usable address to determine total pages */
    uint64_t highest_addr = 0;
    for (uint64_t i = 0; i < count; i++) {
        struct multiboot_mmap_entry *entry = &entries[i];
        if (entry->type == MULTIBOOT_MEMORY_AVAILABLE) {
            uint64_t top = entry->addr + entry->len;
            if (top > highest_addr) {
                highest_addr = top;
            }
        }
    }

    /* Use Detected Memory */
    total_pages = highest_addr / PAGE_SIZE;
    bitmap_size = (total_pages + 7) / 8;  /* Round up to bytes */

extern char _kernel_end[];
#define KERNEL_VIRT_BASE 0xFFFFFFFF80000000ULL

    /* Find a place to put the bitmap (use first usable region after kernel) */
    /* Calculate physical address of kernel end */
    uint64_t phys_kernel_end = (uint64_t)_kernel_end - KERNEL_VIRT_BASE;
    phys_kernel_end += 1024 * 1024; /* Safety margin */

    /* Calculate Usable Memory (Sum of Available Regions) */
    pmm_usable_memory = 0;
    for (uint64_t i = 0; i < count; i++) {
        struct multiboot_mmap_entry *entry = &entries[i];
        if (entry->type == MULTIBOOT_MEMORY_AVAILABLE) {
            uint64_t start = entry->addr;
            uint64_t end = start + entry->len;
            
            if (end > start) {
                pmm_usable_memory += (end - start);
            }
        }
    }

    for (uint64_t i = 0; i < count; i++) {
        struct multiboot_mmap_entry *entry = &entries[i];
        if (entry->type == MULTIBOOT_MEMORY_AVAILABLE) {
            uint64_t start = entry->addr;
            uint64_t end = start + entry->len;

            /* If region overlaps or is below kernel end, adjust start */
            if (start < phys_kernel_end) {
                if (end > phys_kernel_end) {
                    start = phys_kernel_end;
                } else {
                    continue; /* Region entirely below kernel */
                }
            }

            /* No Cap: Use HHDM to access high memory for bitmap */
            if (end > start && (end - start) >= bitmap_size) {
                 /* Use Limine HHDM Mapping (0xFFFF800000000000 + Phys) */
                 bitmap = (uint8_t *)(start + 0xFFFF800000000000ULL);
                 break;
            }
        }
    }

    if (!bitmap) {
        kprint("[PMM] Failed to allocate bitmap! (Need < 1GB memory)\n");
        return;
    }

    /* DEBUG: Print chosen bitmap address */
    early_serial_puts("[PMM] Bitmap allocated at: 0x");
    early_serial_hex64((uint64_t)bitmap);
    early_serial_puts("\n");

    /* Initialize bitmap - mark all as used */
    /* Accessing bitmap might crash if not mapped. We hope it fell in first 1GB */
    early_serial_puts("[PMM] Clearing bitmap...\n");
    for (uint64_t i = 0; i < bitmap_size; i++) {
        bitmap[i] = 0xFF;
    }
    used_pages = total_pages;
    early_serial_puts("[PMM] Bitmap cleared.\n");

    /* Mark usable regions as free */
    for (uint64_t i = 0; i < count; i++) {
        struct multiboot_mmap_entry *entry = &entries[i];
        if (entry->type == MULTIBOOT_MEMORY_AVAILABLE) {
            uint64_t base_page = entry->addr / PAGE_SIZE;
            uint64_t page_count = entry->len / PAGE_SIZE;
            
            for (uint64_t j = 0; j < page_count; j++) {
                if ((base_page + j) < total_pages) {
                     bitmap_clear(base_page + j);
                     used_pages--;
                }
            }
        }
    }

    /* Mark bitmap itself as used */
    early_serial_puts("[PMM] Marking bitmap used...\n");
    uint64_t bitmap_mem_addr;
    if ((uint64_t)bitmap < 0xFFFFFFFFULL) {
        bitmap_mem_addr = (uint64_t)bitmap;
    } else {
        bitmap_mem_addr = (uint64_t)bitmap - 0xFFFF800000000000ULL;
    }
    
    early_serial_puts("[PMM] Bitmap Phys Addr: 0x");
    early_serial_hex64(bitmap_mem_addr);
    early_serial_puts("\n");

    uint64_t bitmap_base_page = bitmap_mem_addr / PAGE_SIZE;
    uint64_t bitmap_num_pages = (bitmap_size + PAGE_SIZE - 1) / PAGE_SIZE;
    
    for (uint64_t i = 0; i < bitmap_num_pages; i++) {
        /* Crash likely here if bitmap unmapped? */
        if ((bitmap_base_page + i) < total_pages) {
             bitmap_set(bitmap_base_page + i);
             used_pages++;
        }
    }
    early_serial_puts("[PMM] Bitmap marked.\n");

    /* Mark Kernel Code/Data as used */
    /* Use valid kernel end symbol from linker */
    uint64_t reserved_size = phys_kernel_end + (1024*1024); /* Kernel + 1MB Safety */
    uint64_t reserved_pages = (reserved_size + PAGE_SIZE - 1) / PAGE_SIZE;

    for (uint64_t i = 0; i < reserved_pages; i++) {
         if (i < total_pages && !bitmap_test(i)) {
             bitmap_set(i);
             used_pages++;
         }
    }

    kprint("[PMM] Initialized\n");
}

void *pmm_alloc_page(void) {
    for (uint64_t i = 0; i < total_pages; i++) {
        if (!bitmap_test(i)) {
            bitmap_set(i);
            used_pages++;
            return (void *)(i * PAGE_SIZE);
        }
    }
    return NULL;  /* Out of memory */
}

/* Allocate multiple contiguous pages */
void *pmm_alloc_pages(uint64_t count) {
    if (count == 0) return NULL;
    
    for (uint64_t i = 0; i < total_pages; i++) {
        if (!bitmap_test(i)) {
            /* Found a free page, check if next 'count-1' are also free */
            int found = 1;
            for (uint64_t j = 1; j < count; j++) {
                if ((i + j) >= total_pages || bitmap_test(i + j)) {
                    found = 0;
                    i += j; /* Skip ahead */
                    break;
                }
            }
            
            if (found) {
                /* Mark all as used */
                for (uint64_t j = 0; j < count; j++) {
                    bitmap_set(i + j);
                    used_pages++;
                }
                return (void *)(i * PAGE_SIZE);
            }
        }
    }
    return NULL;
}

void pmm_free_page(void *page) {
    uint64_t page_num = (uint64_t)page / PAGE_SIZE;
    if (page_num < total_pages && bitmap_test(page_num)) {
        bitmap_clear(page_num);
        used_pages--;
    }
}

uint64_t pmm_get_total_memory(void) {
    /* Return actual Usable RAM found in map */
    return pmm_usable_memory;
}

uint64_t pmm_get_free_memory(void) {
    /* Count free bits in bitmap dynamically to be accurate */
    return (total_pages - used_pages) * PAGE_SIZE;
}

uint64_t pmm_get_used_memory(void) {
    /* Used = Total (Usable) - Free */
    if (pmm_get_free_memory() > pmm_usable_memory) return 0;
    return pmm_usable_memory - pmm_get_free_memory();
}
