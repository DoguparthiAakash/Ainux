#include "vmm.h"
#include "heap.h"
#include "../libc/string.h"

extern void kprint(const char *msg);

/* Kernel address space */
static vmm_address_space_t kernel_space;

/* Helper: Physical to virtual conversion for kernel */
static inline void *phys_to_virt(uint64_t phys) {
    return (void *)(phys + g_hhdm_offset);
}

/* Helper: Virtual to physical for kernel addresses */
static inline uint64_t virt_to_phys(void *virt) {
    return (uint64_t)virt - g_hhdm_offset;
}

/* Get page table entry indices from virtual address */
static inline uint16_t pml4_index(uint64_t virt) {
    return (virt >> 39) & 0x1FF;
}

static inline uint16_t pdpt_index(uint64_t virt) {
    return (virt >> 30) & 0x1FF;
}

static inline uint16_t pd_index(uint64_t virt) {
    return (virt >> 21) & 0x1FF;
}

static inline uint16_t pt_index(uint64_t virt) {
    return (virt >> 12) & 0x1FF;
}

void vmm_init(void) {
    /* Get current CR3 (set up by boot.asm) */
    uint64_t cr3 = vmm_read_cr3();
    
    kernel_space.pml4_phys = cr3 & PTE_ADDR_MASK;
    kernel_space.pml4_virt = phys_to_virt(kernel_space.pml4_phys);
    kernel_space.start = 0xFFFFFFFF80000000ULL;
    kernel_space.end = 0xFFFFFFFFFFFFFFFFULL;
    
    kprint("[VMM] Initialized with boot page tables\n");
}

vmm_address_space_t *vmm_get_kernel_address_space(void) {
    return &kernel_space;
}

/* Get or create next level page table */
static page_table_t *get_or_create_table(pte_t *entry, uint64_t flags) {
    if (*entry & PTE_PRESENT) {
        return phys_to_virt(*entry & PTE_ADDR_MASK);
    }
    
    /* Allocate new page table */
    void *page = pmm_alloc_page();
    if (!page) return NULL;
    
    page_table_t *table = phys_to_virt((uint64_t)page);
    memset(table, 0, sizeof(page_table_t));
    
    /* Set entry to point to new table */
    *entry = (uint64_t)page | PTE_PRESENT | PTE_WRITABLE | (flags & PTE_USER);
    
    return table;
}

int vmm_map_page(vmm_address_space_t *space, uint64_t virt, uint64_t phys, uint64_t flags) {
    if (!space || !space->pml4_virt) return -1;
    
    /* Get indices */
    uint16_t pml4_i = pml4_index(virt);
    uint16_t pdpt_i = pdpt_index(virt);
    uint16_t pd_i = pd_index(virt);
    uint16_t pt_i = pt_index(virt);
    
    /* Walk/create page tables */
    page_table_t *pml4 = space->pml4_virt;
    
    page_table_t *pdpt = get_or_create_table(&pml4->entries[pml4_i], flags);
    if (!pdpt) return -1;
    
    page_table_t *pd = get_or_create_table(&pdpt->entries[pdpt_i], flags);
    if (!pd) return -1;
    
    page_table_t *pt = get_or_create_table(&pd->entries[pd_i], flags);
    if (!pt) return -1;
    
    /* Set final mapping */
    pt->entries[pt_i] = (phys & PTE_ADDR_MASK) | flags | PTE_PRESENT;
    
    /* Invalidate TLB for this page */
    vmm_invlpg(virt);
    
    return 0;
}

int vmm_unmap_page(vmm_address_space_t *space, uint64_t virt) {
    if (!space || !space->pml4_virt) return -1;
    
    uint16_t pml4_i = pml4_index(virt);
    uint16_t pdpt_i = pdpt_index(virt);
    uint16_t pd_i = pd_index(virt);
    uint16_t pt_i = pt_index(virt);
    
    page_table_t *pml4 = space->pml4_virt;
    
    /* Check if tables exist */
    if (!(pml4->entries[pml4_i] & PTE_PRESENT)) return -1;
    page_table_t *pdpt = phys_to_virt(pml4->entries[pml4_i] & PTE_ADDR_MASK);
    
    if (!(pdpt->entries[pdpt_i] & PTE_PRESENT)) return -1;
    page_table_t *pd = phys_to_virt(pdpt->entries[pdpt_i] & PTE_ADDR_MASK);
    
    if (!(pd->entries[pd_i] & PTE_PRESENT)) return -1;
    page_table_t *pt = phys_to_virt(pd->entries[pd_i] & PTE_ADDR_MASK);
    
    /* Clear the mapping */
    pt->entries[pt_i] = 0;
    
    vmm_invlpg(virt);
    
    return 0;
}

uint64_t vmm_get_physical(vmm_address_space_t *space, uint64_t virt) {
    if (!space || !space->pml4_virt) return 0;
    
    uint16_t pml4_i = pml4_index(virt);
    uint16_t pdpt_i = pdpt_index(virt);
    uint16_t pd_i = pd_index(virt);
    uint16_t pt_i = pt_index(virt);
    
    page_table_t *pml4 = space->pml4_virt;
    
    if (!(pml4->entries[pml4_i] & PTE_PRESENT)) return 0;
    page_table_t *pdpt = phys_to_virt(pml4->entries[pml4_i] & PTE_ADDR_MASK);
    
    if (!(pdpt->entries[pdpt_i] & PTE_PRESENT)) return 0;
    
    /* Check for 1GB huge page */
    if (pdpt->entries[pdpt_i] & PTE_HUGE) {
        return (pdpt->entries[pdpt_i] & ~0x3FFFFFFFULL) | (virt & 0x3FFFFFFFULL);
    }
    
    page_table_t *pd = phys_to_virt(pdpt->entries[pdpt_i] & PTE_ADDR_MASK);
    
    if (!(pd->entries[pd_i] & PTE_PRESENT)) return 0;
    
    /* Check for 2MB huge page */
    if (pd->entries[pd_i] & PTE_HUGE) {
        return (pd->entries[pd_i] & ~0x1FFFFFULL) | (virt & 0x1FFFFFULL);
    }
    
    page_table_t *pt = phys_to_virt(pd->entries[pd_i] & PTE_ADDR_MASK);
    
    if (!(pt->entries[pt_i] & PTE_PRESENT)) return 0;
    
    return (pt->entries[pt_i] & PTE_ADDR_MASK) | (virt & 0xFFF);
}

int vmm_map_range(vmm_address_space_t *space, uint64_t virt_start, uint64_t phys_start,
                  uint64_t size, uint64_t flags) {
    uint64_t pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
    
    for (uint64_t i = 0; i < pages; i++) {
        uint64_t virt = virt_start + (i * PAGE_SIZE);
        uint64_t phys = phys_start + (i * PAGE_SIZE);
        
        if (vmm_map_page(space, virt, phys, flags) != 0) {
            /* Rollback on failure */
            for (uint64_t j = 0; j < i; j++) {
                vmm_unmap_page(space, virt_start + (j * PAGE_SIZE));
            }
            return -1;
        }
    }
    
    return 0;
}

vmm_address_space_t *vmm_create_address_space(void) {
    vmm_address_space_t *space = (vmm_address_space_t *)kmalloc(sizeof(vmm_address_space_t));
    if (!space) return NULL;
    
    /* Allocate PML4 */
    void *pml4_page = pmm_alloc_page();
    if (!pml4_page) {
        kfree(space);
        return NULL;
    }
    
    space->pml4_phys = (uint64_t)pml4_page;
    space->pml4_virt = phys_to_virt((uint64_t)pml4_page);
    space->start = 0x0000000000400000ULL;  /* User space starts at 4MB */
    space->end = 0x00007FFFFFFFFFFFULL;    /* Top of user canonical address */
    
    /* Clear the table */
    memset(space->pml4_virt, 0, sizeof(page_table_t));
    
    /* Copy kernel mappings (top half of PML4) */
    page_table_t *kernel_pml4 = kernel_space.pml4_virt;
    for (int i = 256; i < 512; i++) {
        space->pml4_virt->entries[i] = kernel_pml4->entries[i];
    }
    
    return space;
}

void vmm_destroy_address_space(vmm_address_space_t *space) {
    if (!space) return;
    
    /* Free user-space page tables (indices 0-255) */
    for (int pml4_i = 0; pml4_i < 256; pml4_i++) {
        if (!(space->pml4_virt->entries[pml4_i] & PTE_PRESENT)) continue;
        
        page_table_t *pdpt = phys_to_virt(space->pml4_virt->entries[pml4_i] & PTE_ADDR_MASK);
        
        for (int pdpt_i = 0; pdpt_i < 512; pdpt_i++) {
            if (!(pdpt->entries[pdpt_i] & PTE_PRESENT)) continue;
            if (pdpt->entries[pdpt_i] & PTE_HUGE) continue; /* Don't free huge page tables */
            
            page_table_t *pd = phys_to_virt(pdpt->entries[pdpt_i] & PTE_ADDR_MASK);
            
            for (int pd_i = 0; pd_i < 512; pd_i++) {
                if (!(pd->entries[pd_i] & PTE_PRESENT)) continue;
                if (pd->entries[pd_i] & PTE_HUGE) continue;
                
                /* Free page table */
                pmm_free_page((void *)(pd->entries[pd_i] & PTE_ADDR_MASK));
            }
            
            pmm_free_page((void *)(pdpt->entries[pdpt_i] & PTE_ADDR_MASK));
        }
        
        pmm_free_page((void *)(space->pml4_virt->entries[pml4_i] & PTE_ADDR_MASK));
    }
    
    /* Free PML4 itself */
    pmm_free_page((void *)space->pml4_phys);
    kfree(space);
}

void vmm_switch_address_space(vmm_address_space_t *space) {
    if (!space) return;
    vmm_write_cr3(space->pml4_phys);
}

void *vmm_alloc_region(vmm_address_space_t *space, uint64_t size, uint64_t flags) {
    if (!space) return NULL;
    
    uint64_t pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
    
    /* Simple linear search for free virtual region */
    /* TODO: Implement a proper virtual memory region allocator */
    static uint64_t next_alloc = 0x0000000010000000ULL; /* Start at 256MB */
    
    uint64_t virt_start = next_alloc;
    next_alloc += pages * PAGE_SIZE;
    
    /* Allocate and map pages */
    for (uint64_t i = 0; i < pages; i++) {
        void *page = pmm_alloc_page();
        if (!page) {
            /* Cleanup on failure */
            for (uint64_t j = 0; j < i; j++) {
                uint64_t virt = virt_start + (j * PAGE_SIZE);
                uint64_t phys = vmm_get_physical(space, virt);
                vmm_unmap_page(space, virt);
                pmm_free_page((void *)phys);
            }
            return NULL;
        }
        
        vmm_map_page(space, virt_start + (i * PAGE_SIZE), (uint64_t)page, flags);
    }
    
    return (void *)virt_start;
}

void vmm_free_region(vmm_address_space_t *space, void *addr, uint64_t size) {
    if (!space || !addr) return;
    
    uint64_t pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
    uint64_t virt_start = (uint64_t)addr;
    
    for (uint64_t i = 0; i < pages; i++) {
        uint64_t virt = virt_start + (i * PAGE_SIZE);
        uint64_t phys = vmm_get_physical(space, virt);
        
        if (phys) {
            vmm_unmap_page(space, virt);
            pmm_free_page((void *)phys);
        }
    }
}
