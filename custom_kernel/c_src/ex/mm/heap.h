#ifndef HEAP_H
#define HEAP_H

#include <stdint.h>
#include <stddef.h>

/* Initialize heap with dynamic HHDM offset */
void heap_init(uint64_t hhdm_offset);

/* Allocate memory */
void *kmalloc(size_t size);

/* Free memory (stub for now) */
void kfree(void *ptr);

#endif
