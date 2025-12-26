#include "pmm.h"
#include <limine.h>

extern void kprint(const char *msg);

/* Limine memory map request */
__attribute__((used, section(".requests")))
static volatile struct limine_memmap_request memmap_request = {
    .id = LIMINE_MEMMAP_REQUEST,
    .revision = 0
};

/* Bitmap to track page allocation */
static uint8_t *bitmap = NULL;
static uint64_t total_pages = 0;
static uint64_t used_pages = 0;
static uint64_t bitmap_size = 0;

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

void pmm_init(void) {
    if (memmap_request.response == NULL) {
        kprint("[PMM] No memory map available!\n");
        return;
    }

    struct limine_memmap_response *memmap = memmap_request.response;
    
    /* Find highest usable address to determine total pages */
    uint64_t highest_addr = 0;
    for (uint64_t i = 0; i < memmap->entry_count; i++) {
        struct limine_memmap_entry *entry = memmap->entries[i];
        if (entry->type == LIMINE_MEMMAP_USABLE) {
            uint64_t top = entry->base + entry->length;
            if (top > highest_addr) {
                highest_addr = top;
            }
        }
    }

    total_pages = highest_addr / PAGE_SIZE;
    bitmap_size = (total_pages + 7) / 8;  /* Round up to bytes */

    /* Find a place to put the bitmap (use first usable region) */
    for (uint64_t i = 0; i < memmap->entry_count; i++) {
        struct limine_memmap_entry *entry = memmap->entries[i];
        if (entry->type == LIMINE_MEMMAP_USABLE && entry->length >= bitmap_size) {
            bitmap = (uint8_t *)(entry->base + 0xFFFF800000000000ULL);  /* Higher half */
            break;
        }
    }

    if (!bitmap) {
        kprint("[PMM] Failed to allocate bitmap!\n");
        return;
    }

    /* Initialize bitmap - mark all as used */
    for (uint64_t i = 0; i < bitmap_size; i++) {
        bitmap[i] = 0xFF;
    }
    used_pages = total_pages;

    /* Mark usable regions as free */
    for (uint64_t i = 0; i < memmap->entry_count; i++) {
        struct limine_memmap_entry *entry = memmap->entries[i];
        if (entry->type == LIMINE_MEMMAP_USABLE) {
            uint64_t base_page = entry->base / PAGE_SIZE;
            uint64_t page_count = entry->length / PAGE_SIZE;
            
            for (uint64_t j = 0; j < page_count; j++) {
                bitmap_clear(base_page + j);
                used_pages--;
            }
        }
    }

    /* Mark bitmap itself as used */
    uint64_t bitmap_pages = (bitmap_size + PAGE_SIZE - 1) / PAGE_SIZE;
    uint64_t bitmap_base_page = ((uint64_t)bitmap - 0xFFFF800000000000ULL) / PAGE_SIZE;
    for (uint64_t i = 0; i < bitmap_pages; i++) {
        bitmap_set(bitmap_base_page + i);
        used_pages++;
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
    return total_pages * PAGE_SIZE;
}

uint64_t pmm_get_free_memory(void) {
    return (total_pages - used_pages) * PAGE_SIZE;
}

uint64_t pmm_get_used_memory(void) {
    return used_pages * PAGE_SIZE;
}
