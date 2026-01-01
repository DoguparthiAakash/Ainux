#include "elf.h"
#include "../mm/vmm.h"
#include "../mm/pmm.h"
#include "../mm/heap.h"
#include "../libc/string.h"

extern void kprint(const char *msg);

/* User stack size: 2MB */
#define USER_STACK_SIZE (2 * 1024 * 1024)

/* User stack top address */
#define USER_STACK_TOP 0x00007FFFFFFFE000ULL

int elf_validate(const uint8_t *data, uint64_t size) {
    if (size < sizeof(elf64_header_t)) {
        return -1;
    }
    
    elf64_header_t *header = (elf64_header_t *)data;
    
    /* Check magic number */
    if (header->magic != ELF_MAGIC) {
        kprint("[ELF] Invalid magic number\n");
        return -1;
    }
    
    /* Check 64-bit */
    if (header->elf_class != 2) {
        kprint("[ELF] Not a 64-bit ELF\n");
        return -1;
    }
    
    /* Check little endian */
    if (header->endian != 1) {
        kprint("[ELF] Not little endian\n");
        return -1;
    }
    
    /* Check x86_64 */
    if (header->machine != EM_X86_64) {
        kprint("[ELF] Not x86_64 architecture\n");
        return -1;
    }
    
    /* Check executable or dynamic */
    if (header->type != ET_EXEC && header->type != ET_DYN) {
        kprint("[ELF] Not an executable file\n");
        return -1;
    }
    
    return 0;
}

int elf_load(const uint8_t *data, uint64_t size, elf_load_result_t *result) {
    if (!data || !result) return -1;
    
    /* Validate ELF */
    if (elf_validate(data, size) != 0) {
        return -1;
    }
    
    elf64_header_t *header = (elf64_header_t *)data;
    
    /* Create new address space for this process */
    vmm_address_space_t *space = vmm_create_address_space();
    if (!space) {
        kprint("[ELF] Failed to create address space\n");
        return -1;
    }
    
    /* Track highest loaded address for brk */
    uint64_t highest_addr = 0;
    
    /* Load program segments */
    for (uint16_t i = 0; i < header->phnum; i++) {
        elf64_phdr_t *phdr = (elf64_phdr_t *)(data + header->phoff + (i * header->phentsize));
        
        if (phdr->type != PT_LOAD) continue;
        
        /* Skip zero-size segments */
        if (phdr->memsz == 0) continue;
        
        /* Calculate page-aligned addresses */
        uint64_t vaddr_start = phdr->vaddr & ~0xFFFULL;
        uint64_t vaddr_end = (phdr->vaddr + phdr->memsz + 0xFFF) & ~0xFFFULL;
        uint64_t pages = (vaddr_end - vaddr_start) / PAGE_SIZE;
        
        /* Determine page flags */
        uint64_t flags = PTE_PRESENT | PTE_USER;
        if (phdr->flags & PF_W) flags |= PTE_WRITABLE;
        if (!(phdr->flags & PF_X)) flags |= PTE_NX;
        
        /* Allocate and map pages for this segment */
        for (uint64_t p = 0; p < pages; p++) {
            uint64_t virt = vaddr_start + (p * PAGE_SIZE);
            
            /* Allocate physical page */
            void *phys_page = pmm_alloc_page();
            if (!phys_page) {
                kprint("[ELF] Out of memory loading segment\n");
                vmm_destroy_address_space(space);
                return -1;
            }
            
            /* Map it */
            if (vmm_map_page(space, virt, (uint64_t)phys_page, flags) != 0) {
                kprint("[ELF] Failed to map page\n");
                pmm_free_page(phys_page);
                vmm_destroy_address_space(space);
                return -1;
            }
            
            /* Zero the page first */
            uint8_t *page_data = (uint8_t *)((uint64_t)phys_page + g_hhdm_offset);
            memset(page_data, 0, PAGE_SIZE);
        }
        
        /* Copy segment data */
        if (phdr->filesz > 0) {
            /* Calculate source and dest */
            uint64_t bytes_to_copy = phdr->filesz;
            uint64_t src_offset = phdr->offset;
            uint64_t dst_vaddr = phdr->vaddr;
            
            while (bytes_to_copy > 0) {
                /* Get physical address for this virtual page */
                uint64_t page_vaddr = dst_vaddr & ~0xFFFULL;
                uint64_t page_offset = dst_vaddr & 0xFFF;
                uint64_t phys = vmm_get_physical(space, page_vaddr);
                
                if (!phys) {
                    kprint("[ELF] Failed to get physical address for copy\n");
                    vmm_destroy_address_space(space);
                    return -1;
                }
                
                uint8_t *dst = (uint8_t *)(phys + g_hhdm_offset + page_offset);
                uint64_t copy_size = PAGE_SIZE - page_offset;
                if (copy_size > bytes_to_copy) copy_size = bytes_to_copy;
                
                memcpy(dst, data + src_offset, copy_size);
                
                bytes_to_copy -= copy_size;
                src_offset += copy_size;
                dst_vaddr += copy_size;
            }
        }
        
        /* Track highest address */
        uint64_t segment_end = phdr->vaddr + phdr->memsz;
        if (segment_end > highest_addr) {
            highest_addr = segment_end;
        }
    }
    
    /* Allocate user stack */
    uint64_t stack_pages = USER_STACK_SIZE / PAGE_SIZE;
    uint64_t stack_bottom = USER_STACK_TOP - USER_STACK_SIZE;
    
    for (uint64_t p = 0; p < stack_pages; p++) {
        void *phys_page = pmm_alloc_page();
        if (!phys_page) {
            kprint("[ELF] Out of memory allocating stack\n");
            vmm_destroy_address_space(space);
            return -1;
        }
        
        uint64_t virt = stack_bottom + (p * PAGE_SIZE);
        if (vmm_map_page(space, virt, (uint64_t)phys_page, 
                         PTE_PRESENT | PTE_WRITABLE | PTE_USER | PTE_NX) != 0) {
            kprint("[ELF] Failed to map stack page\n");
            pmm_free_page(phys_page);
            vmm_destroy_address_space(space);
            return -1;
        }
        
        /* Zero the stack page */
        uint8_t *page_data = (uint8_t *)((uint64_t)phys_page + g_hhdm_offset);
        memset(page_data, 0, PAGE_SIZE);
    }
    
    /* Fill result */
    result->entry_point = header->entry;
    /* Move stack top down slightly so RSP points to VALID memory */
    /* If RSP=TOP, a POP reads from TOP (invalid). PUSH writes to TOP-8 (valid). */
    /* Nano-C crt0 might expect arguments at RSP. */
    /* Let's set it to valid allocated byte */
    result->stack_top = USER_STACK_TOP - 16;
    result->brk = (highest_addr + 0xFFF) & ~0xFFFULL; /* Page-align brk */
    result->address_space = space;
    
    kprint("[ELF] Successfully loaded executable\n");
    
    return 0;
}

#include "vfs.h"
#include "../mm/heap.h"

int elf_load_file(const char *path, elf_load_result_t *result) {
    if (!path || !result) return -1;
    
    kprint("[ELF] Loading file: ");
    kprint(path);
    kprint("\n");
    
    if (!fs_root) {
        kprint("[ELF] ERROR: fs_root is NULL!\n");
        return -1;
    }
    
    /* Use VFS to find the file */
    /* Note: vfs_lookup currently only handles simple paths or root childs if not fully implemented. 
       Ensure vfs_lookup is robust or use a simplified lookup for now. */
    vfs_node_t *node = vfs_lookup(fs_root, path);
    if (!node) {
        kprint("[ELF] File not found\n");
        return -1;
    }
    
    uint64_t size = node->length;
    if (size == 0) {
        kprint("[ELF] File is empty\n");
        return -1;
    }
    
    /* Allocate buffer */
    uint8_t *buf = (uint8_t *)kmalloc(size);
    if (!buf) {
        kprint("[ELF] Out of memory for file buffer\n");
        return -1;
    }
    
    /* Read file */
    uint64_t bytes_read = vfs_read(node, 0, size, buf);
    if (bytes_read != size) {
        kprint("[ELF] Failed to read full file\n");
        kfree(buf);
        return -1;
    }
    
    /* Load ELF */
    int ret = elf_load(buf, size, result);
    
    kfree(buf);
    return ret;
}
