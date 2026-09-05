#ifndef VMM_H
#define VMM_H

#include <stdint.h>
#include <stddef.h>
#include "pmm.h"

/* Page table entry flags */
#define PTE_PRESENT     (1ULL << 0)
#define PTE_WRITABLE    (1ULL << 1)
#define PTE_USER        (1ULL << 2)
#define PTE_WRITETHROUGH (1ULL << 3)
#define PTE_NOCACHE     (1ULL << 4)
#define PTE_ACCESSED    (1ULL << 5)
#define PTE_DIRTY       (1ULL << 6)
#define PTE_HUGE        (1ULL << 7)
#define PTE_GLOBAL      (1ULL << 8)
#define PTE_NX          (1ULL << 63)

/* Address masks */
#define PAGE_MASK       (~0xFFFULL)
#define PTE_ADDR_MASK   0x000FFFFFFFFFF000ULL

/* Page table structure (4-level for x86_64) */
typedef uint64_t pte_t;

/* Page table - each level has 512 entries */
typedef struct {
    pte_t entries[512];
} __attribute__((aligned(4096))) page_table_t;

/* Virtual address space structure */
typedef struct vmm_address_space {
    uint64_t pml4_phys;         /* Physical address of PML4 */
    page_table_t *pml4_virt;    /* Virtual address of PML4 */
    uint64_t start;             /* Start of user space */
    uint64_t end;               /* End of user space */
} vmm_address_space_t;

/* VMM Initialization */
void vmm_init(void);

/* Create a new address space (for new processes) */
vmm_address_space_t *vmm_create_address_space(void);

/* Destroy an address space */
void vmm_destroy_address_space(vmm_address_space_t *space);

/* Switch to an address space (load CR3) */
void vmm_switch_address_space(vmm_address_space_t *space);

/* Get current address space */
vmm_address_space_t *vmm_get_kernel_address_space(void);

/* Map a page */
int vmm_map_page(vmm_address_space_t *space, uint64_t virt, uint64_t phys, uint64_t flags);

/* Unmap a page */
int vmm_unmap_page(vmm_address_space_t *space, uint64_t virt);

/* Get physical address for virtual */
uint64_t vmm_get_physical(vmm_address_space_t *space, uint64_t virt);

/* Map a range of pages */
int vmm_map_range(vmm_address_space_t *space, uint64_t virt_start, uint64_t phys_start, 
                  uint64_t size, uint64_t flags);

/* Allocate virtual memory region */
void *vmm_alloc_region(vmm_address_space_t *space, uint64_t size, uint64_t flags);

/* Free virtual memory region */
void vmm_free_region(vmm_address_space_t *space, void *addr, uint64_t size);

/* Invalidate TLB for a page */
static inline void vmm_invlpg(uint64_t addr) {
    __asm__ volatile("invlpg (%0)" : : "r"(addr) : "memory");
}

/* Flush entire TLB (reload CR3) */
static inline void vmm_flush_tlb(void) {
    uint64_t cr3;
    __asm__ volatile("mov %%cr3, %0" : "=r"(cr3));
    __asm__ volatile("mov %0, %%cr3" : : "r"(cr3) : "memory");
}

/* Get CR3 */
static inline uint64_t vmm_read_cr3(void) {
    uint64_t cr3;
    __asm__ volatile("mov %%cr3, %0" : "=r"(cr3));
    return cr3;
}

/* Set CR3 */
static inline void vmm_write_cr3(uint64_t cr3) {
    __asm__ volatile("mov %0, %%cr3" : : "r"(cr3) : "memory");
}

#endif
