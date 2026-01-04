#include "slab.h"
#include "spinlock.h"
#include "mm/heap.h"
#include "mm/pmm.h"
#include "log.h"
#include "libc/string.h"

/* 
 * Brutal Slab Allocator 
 * Speed: O(1) allocation/free.
 * Strategy: Pre-allocates pages and splits them into fixed-size chunks.
 * Lockless? No, uses fine-grained locks per cache.
 */

#define MAX_CACHES 32
static struct slab_cache caches[MAX_CACHES];
static int next_cache_idx = 0;

void slab_init(void) {
    memset(caches, 0, sizeof(caches));
    kprint("[SLAB] Initialized. Ready for O(1) object allocation.\n");
}

struct slab_cache *slab_create(const char *name, size_t size) {
    if (next_cache_idx >= MAX_CACHES) return 0;
    
    struct slab_cache *cache = &caches[next_cache_idx++];
    strncpy(cache->name, name, 31);
    /* Align size to 8 bytes */
    if (size % 8 != 0) size += (8 - (size % 8));
    /* Ensure room for free list pointer (min 8 bytes) */
    if (size < 8) size = 8;
    
    cache->object_size = size;
    cache->free_list = 0;
    spinlock_init(&cache->lock, name);
    
    kprint("[SLAB] Created cache: "); kprint(name); kprint("\n");
    return cache;
}

static void slab_grow(struct slab_cache *cache) {
    /* Allocate 1 page (4KB) */
    void *page = kmalloc(4096); 
    /* NOTE: calling kmalloc from slab? 
       Yes, slab is layer ON TOP of generic allocator (or PMM).
       Ideally PMM. But kmalloc is easier for now. 
       Wait, kmalloc tracks size. PMM gives raw page. PMM is better for Slab. */
    // void *page = pmm_alloc_pages(1); /* Need virtual address? PMM returns PHYS */
    // Using kmalloc is safest for now as it handles mapping.
    
    if (!page) return;
    
    /* Split page into objects */
    size_t count = 4096 / cache->object_size;
    uint8_t *ptr = (uint8_t *)page;
    
    for (size_t i = 0; i < count; i++) {
        /* Link them */
        void *next_ptr = (i == count - 1) ? cache->free_list : (void*)(ptr + cache->object_size);
        *(void**)ptr = next_ptr;
        ptr += cache->object_size;
    }
    /* Incorrect linking logic above? 
       Let's redo: 
       We want ptr -> next_ptr. 
       last_object -> old_free_list. 
    */
    
    ptr = (uint8_t *)page;
    for (size_t i = 0; i < count - 1; i++) {
        void **obj = (void**)ptr;
        *obj = (void*)(ptr + cache->object_size);
        ptr += cache->object_size;
    }
    
    /* Last one points to current free list head */
    *(void**)ptr = cache->free_list;
    
    /* Update head */
    cache->free_list = page;
}

void *slab_alloc(struct slab_cache *cache) {
    uint64_t flags;
    spinlock_irq_save(flags);
    spinlock_acquire(&cache->lock);
    
    if (!cache->free_list) {
        slab_grow(cache);
    }
    
    void *obj = 0;
    if (cache->free_list) {
        obj = cache->free_list;
        cache->free_list = *(void**)obj; /* Pop */
        cache->total_allocs++;
    }
    
    spinlock_release(&cache->lock);
    spinlock_irq_restore(flags);
    
    return obj;
}

void slab_free(struct slab_cache *cache, void *ptr) {
    if (!ptr) return;
    
    uint64_t flags;
    spinlock_irq_save(flags);
    spinlock_acquire(&cache->lock);
    
    /* Push back */
    *(void**)ptr = cache->free_list;
    cache->free_list = ptr;
    
    spinlock_release(&cache->lock);
    spinlock_irq_restore(flags);
}
