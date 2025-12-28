#include "pmm.h"
#include "boot_info.h"

extern void kprint(const char *msg);

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
    if (!g_boot_info || !g_boot_info->memory_map) {
        kprint("[PMM] No memory map available!\n");
        return;
    }

    struct e820_entry *entries = (struct e820_entry *)g_boot_info->memory_map;
    uint64_t count = g_boot_info->memory_map_entries;
    
    /* Find highest usable address to determine total pages */
    uint64_t highest_addr = 0;
    for (uint64_t i = 0; i < count; i++) {
        struct e820_entry *entry = &entries[i];
        if (entry->type == 1) { /* 1 = USABLE */
            uint64_t top = entry->base + entry->length;
            if (top > highest_addr) {
                highest_addr = top;
            }
        }
    }

    total_pages = highest_addr / PAGE_SIZE;
    bitmap_size = (total_pages + 7) / 8;  /* Round up to bytes */

    /* Find a place to put the bitmap (use first usable region) */
    /* Must be careful: E820 addresses are physical. 
       We are in Higher Half, but assuming Identity map of lower 4GB or accessing physical?
       Wait, in 64-bit mode we access virtual.
       Our Identity map covers 0-2MB. (In stage2)
       If bitmap needs to go higher, we might crash if not mapped.
       However, PMM usually needs to map itself.
       For now, let's try to find a spot in the first 2MB if possible, or assume we can access physical memory directly 
       if we had a direct map (which we don't fully have yet, only 2MB).
       
       CRITICAL: We only mapped 0-2MB identity and Higher Half -> 0-2MB.
       If RAM is > 2MB, we cannot access it yet without mapping it!
       We need to map all physical memory to HHDM (Higher Half Direct Map) or similar.
       Or just map it on demand.
       
       Simplification: 
       We will place the bitmap in the known mapped area if possible, or just pray?
       Actually, `kernel.bin` is at 0x100000. It's about 400KB.
       So 0x100000 - 0x170000 is used.
       0x170000 to 0x200000 is free (approx 500KB).
       Bitmap for 2GB RAM: 2*10^9 / 4096 / 8 = 64KB.
       It fits easily in the space after kernel before 2MB.
       So we search for a usable region that includes [end_of_kernel, 2MB].
    */
    
    /* Hardcoded placement attempt: after kernel (approx 1MB mark + size) */
    /* Let's find a usable region that can hold bitmap_size */
    
    for (uint64_t i = 0; i < count; i++) {
        struct e820_entry *entry = &entries[i];
        if (entry->type == 1 && entry->length >= bitmap_size) {
            /* Check if this region is within our mapped window (0-2MB) */
            /* Or we just use a fixed address we expect to be free, like 0x500000 (5MB) - Wait 5MB is NOT mapped */
            /* We MUST use < 2MB */
            
            uint64_t candidate = entry->base;
            /* Align to page */
            if (candidate % PAGE_SIZE != 0) candidate += PAGE_SIZE - (candidate % PAGE_SIZE);
            
            /* Skip low memory < 1MB to be safe (BIOS, Bootloader, Kernel) */
            if (candidate < 0x180000) candidate = 0x180000; 
            
            if (candidate + bitmap_size <= entry->base + entry->length && candidate < 0x200000) {
                 /* We can use this. Access via identity map (lower half) for now, or if HH is mapped to 0, use HH offset?
                    We mapped 0xFFFFFFFF80000000 -> 0x00100000 (kernel load).
                    Wait, `stage2.asm` mapped:
                    Map kernel load address (0x100000) to Higher Half
                    Virtual 0xFFFFFFFF80000000 -> PDPT[510] -> PD[0] (which is mapped to 0??)
                    
                    In stage2:
                    Identity Map First 2MB in PD using 2MB Pages
                    PD[0] -> 0x00000000 | Present | RW | HugePage
                    
                    So Virtual 0 -> Phys 0  (0-2MB)
                    And Virtual 0xFFFFFFFF80000000 -> Phys 0 (0-2MB)??
                    
                    Let's check stage2:
                    `mov dword [0x11000 + 510*8], 0x12003` (PDPT[510] -> PD)
                    So 0xFF...... uses PD at 0x12000.
                    PD[0] (at 0x12000) is 0x83 (Phys 0).
                    
                    Yes! Virtual 0xFFFFFFFF80000000 maps to Physical 0.
                    So address 0xFFFFFFFF80180000 maps to Physical 0x180000.
                 */
                 bitmap = (uint8_t *)(candidate + 0xFFFFFFFF80000000ULL); 
                 break;
            }
        }
    }

    if (!bitmap) {
        kprint("[PMM] Failed to allocate bitmap (needs to be in first 2MB)!\n");
        return;
    }

    /* Initialize bitmap - mark all as used */
    for (uint64_t i = 0; i < bitmap_size; i++) {
        bitmap[i] = 0xFF; /* All used */
    }
    used_pages = total_pages;

    /* Mark usable regions as free */
    for (uint64_t i = 0; i < count; i++) {
        struct e820_entry *entry = &entries[i];
        if (entry->type == 1) {
            uint64_t base_page = entry->base / PAGE_SIZE;
            uint64_t page_count = entry->length / PAGE_SIZE;
            
            for (uint64_t j = 0; j < page_count; j++) {
                if ((base_page + j) < total_pages) {
                     bitmap_clear(base_page + j);
                     used_pages--;
                }
            }
        }
    }

    /* Mark bitmap itself as used */
    uint64_t bitmap_pages = (bitmap_size + PAGE_SIZE - 1) / PAGE_SIZE;
    uint64_t bitmap_start_virt = (uint64_t)bitmap;
    uint64_t bitmap_start_phys = bitmap_start_virt - 0xFFFFFFFF80000000ULL;
    uint64_t bitmap_base_page = bitmap_start_phys / PAGE_SIZE;
    
    for (uint64_t i = 0; i < bitmap_pages; i++) {
        bitmap_set(bitmap_base_page + i);
        used_pages++;
    }
    
    /* Mark Kernel and Bootloader areas as used (0-2MB basically) */
    /* Just mark first 512 pages (2MB) as used to be safe */
    for (uint64_t i=0; i<512; i++) {
        if (!bitmap_test(i)) {
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
    return total_pages * PAGE_SIZE;
}

uint64_t pmm_get_free_memory(void) {
    return (total_pages - used_pages) * PAGE_SIZE;
}

uint64_t pmm_get_used_memory(void) {
    return used_pages * PAGE_SIZE;
}
