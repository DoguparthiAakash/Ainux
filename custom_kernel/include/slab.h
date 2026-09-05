#ifndef SLAB_H
#define SLAB_H

#include <stdint.h>
#include <stddef.h>
#include "spinlock.h"

/* Slab Cache Structure */

/* Slab Cache Structure */
struct slab_cache {
    char name[32];
    size_t object_size;
    void *free_list;     /* Linked list of free objects */
    void *partial_head;  /* List of partial pages (not implemented in v1) */
    spinlock_t lock;
    uint32_t total_allocs;
};

/* API */
void slab_init(void);
struct slab_cache *slab_create(const char *name, size_t size);
void *slab_alloc(struct slab_cache *cache);
void slab_free(struct slab_cache *cache, void *ptr);

#endif
