#ifndef EX_OBJECT_H
#define EX_OBJECT_H

#include <stddef.h>
#include <stdint.h>

/* Object Types */
typedef enum {
    OB_TYPE_UNKNOWN = 0,
    OB_TYPE_PROCESS,
    OB_TYPE_THREAD,
    OB_TYPE_FILE,
    OB_TYPE_DEVICE,
    OB_TYPE_DRIVER,
    OB_TYPE_VNODE,
    OB_TYPE_MUTEX,
    OB_TYPE_SEMAPHORE
} ob_type_t;

/* Object Header - Precedes the actual object body */
/* Aligned to 16 bytes for good measure */
typedef struct {
    uint32_t magic;      /* Safety check (e.g., 'OBJH') */
    uint32_t type;       /* Object Type ID */
    volatile int ref_count; /* Atomic reference count */
    uint32_t flags;      /* State flags */
    char name[64];       /* Object Name (fixed size for zero-alloc overhead? or pointer?)
                            Fixed size 64 bytes is 64B overhead per object.
                            Pointer is 8B but requires extra malloc/free and fragmentation.
                            Let's go with fixed small name for now, or just pointer.
                            Optimization: Pointer is better for space if names are mostly empty.
                            Safe: Fixed buffer avoids pointer chasing. 
                            Let's use static buffer for robustness. */
    void (*cleanup)(void *obj); /* Destructor */
} object_header_t;

/* Public API */

/* Allocates a new object of given size + header size. 
   Returns pointer to BODY (hiding header). */
void *ob_create_object(ob_type_t type, size_t body_size, const char *name, void (*cleanup)(void*));

/* Increment Ref Count */
void ob_reference_object(void *obj);

/* Decrement Ref Count. If 0, free. */
void ob_dereference_object(void *obj);

/* Get Object Name */
const char *ob_get_name(void *obj);

/* Get Object Type */
ob_type_t ob_get_type(void *obj);

#endif
