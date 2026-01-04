#include "object.h"
#include "mm/heap.h"
#include "libc/string.h"
#include "libc/stdio.h" /* For debug print if needed */

#define OB_MAGIC 0x4F424A48 /* 'OBJH' */

/* Helper to get header from body pointer */
static object_header_t *get_header(void *obj) {
    return (object_header_t *)((uint8_t *)obj - sizeof(object_header_t));
}

void *ob_create_object(ob_type_t type, size_t body_size, const char *name, void (*cleanup)(void*)) {
    size_t total_size = sizeof(object_header_t) + body_size;
    
    /* Alloc Memory */
    /* Calls kmalloc (Rust allocator) */
    void *mem = kmalloc(total_size);
    if (!mem) return NULL;
    
    object_header_t *header = (object_header_t *)mem;
    header->magic = OB_MAGIC;
    header->type = type;
    header->ref_count = 1; /* Start with 1 reference */
    header->flags = 0;
    header->cleanup = cleanup;
    
    if (name) {
        /* Robust strncpy logic */
        int i=0;
        while(name[i] && i < 63) { header->name[i] = name[i]; i++; }
        header->name[i] = 0;
    } else {
        header->name[0] = 0;
    }
    
    /* Return pointer to body */
    return (void *)((uint8_t *)mem + sizeof(object_header_t));
}

void ob_reference_object(void *obj) {
    if (!obj) return;
    object_header_t *header = get_header(obj);
    if (header->magic != OB_MAGIC) return; /* Safety check */
    
    /* TODO: Atomic increment */
    header->ref_count++;
}

void ob_dereference_object(void *obj) {
    if (!obj) return;
    object_header_t *header = get_header(obj);
    if (header->magic != OB_MAGIC) return;
    
    /* TODO: Atomic decrement */
    header->ref_count--;
    
    if (header->ref_count <= 0) {
        /* Cleanup */
        if (header->cleanup) {
            header->cleanup(obj);
        }
        
        /* Free the whole memory block (header + body) */
        kfree(header);
    }
}

const char *ob_get_name(void *obj) {
    if (!obj) return "";
    object_header_t *header = get_header(obj);
    if (header->magic != OB_MAGIC) return "";
    return header->name;
}

ob_type_t ob_get_type(void *obj) {
    if (!obj) return OB_TYPE_UNKNOWN;
    object_header_t *header = get_header(obj);
    if (header->magic != OB_MAGIC) return OB_TYPE_UNKNOWN;
    return (ob_type_t)header->type;
}
