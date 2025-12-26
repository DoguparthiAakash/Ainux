#ifndef PMM_H
#define PMM_H

#include <stdint.h>
#include <stddef.h>

#define PAGE_SIZE 4096

/* Initialize physical memory manager from Limine memory map */
void pmm_init(void);

/* Allocate a physical page (4KB) */
void *pmm_alloc_page(void);

/* Allocate contiguous physical pages */
void *pmm_alloc_pages(uint64_t count);

/* Free a physical page */
void pmm_free_page(void *page);

/* Get memory statistics */
uint64_t pmm_get_total_memory(void);
uint64_t pmm_get_free_memory(void);
uint64_t pmm_get_used_memory(void);

#endif
