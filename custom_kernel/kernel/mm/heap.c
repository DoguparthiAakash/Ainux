#include "heap.h"

// Replaced by Rust implementation in memory_rs
#if 0
#include "pmm.h"

extern void kprint(const char *msg);

/* Simple bump allocator */
static uint8_t *heap_start = NULL;
static uint8_t *heap_current = NULL;
static uint64_t heap_size = 0;

#define HEAP_SIZE (32 * 1024 * 1024)  /* 32MB heap */

void heap_init(void) {
    /* Allocate pages for heap */
    uint64_t pages_needed = (HEAP_SIZE + PAGE_SIZE - 1) / PAGE_SIZE;
    
    /* Request CONTIGUOUS physical memory */
    void *phys_addr = pmm_alloc_pages(pages_needed);
    
    if (!phys_addr) {
        kprint("[HEAP] Failed to allocate contiguous memory!\n");
        return;
    }

    /* Map to higher half */
    heap_start = (uint8_t *)((uint64_t)phys_addr + 0xFFFF800000000000ULL);
    heap_current = heap_start;
    heap_size = pages_needed * PAGE_SIZE;

    kprint("[HEAP] Initialized (Contiguous)\n");
}

void *kmalloc(size_t size) {
    if (!heap_start) {
        return NULL;
    }

    /* Align to 8 bytes */
    size = (size + 7) & ~7;

    if ((uint64_t)(heap_current - heap_start) + size > heap_size) {
        return NULL;  /* Out of heap memory */
    }

    void *ptr = heap_current;
    heap_current += size;
    return ptr;
}

void kfree(void *ptr) {
    /* Bump allocator doesn't support free */
    (void)ptr;
}
#endif
