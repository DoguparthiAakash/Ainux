#ifndef HEAP_H
#define HEAP_H

#include <stdint.h>
#include <stddef.h>

/* Initialize kernel heap */
void heap_init(void);

/* Allocate memory */
void *kmalloc(size_t size);

/* Free memory (stub for now) */
void kfree(void *ptr);

#endif
