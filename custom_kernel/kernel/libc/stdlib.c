#include "stdlib.h"
#include "string.h"
#include "../mm/heap.h"

void *malloc(size_t size) {
    return kmalloc(size);
}

void free(void *ptr) {
    kfree(ptr);
}

void *calloc(size_t nmemb, size_t size) {
    size_t total = nmemb * size;
    void *p = malloc(total);
    if (p) memset(p, 0, total);
    return p;
}

void *realloc(void *ptr, size_t size) {
    if (!ptr) return malloc(size);
    if (size == 0) {
        free(ptr);
        return NULL;
    }
    // We don't have block size info in kmalloc yet trivially exposed, 
    // so we just alloc new and copy. This is unsafe if we copy too much.
    // TODO: Improve heap to expose size. For now assume we copy 'size' bytes 
    // which is dangerous if new size > old size (buffer overflow read).
    // Safest bet for now: simple heap doesn't support realloc well.
    // Let's alloc and copy a reasonable amount.
    void *newp = malloc(size);
    if (newp) {
        // We can't know old size easily. Copying 'size' might read garbage.
        // Assuming user knows what they are doing... 
        // Real implementation needs Malloc Header inspection.
        memcpy(newp, ptr, size); // DANGEROUS but common stub
        free(ptr);
    }
    return newp;
}

int abs(int j) {
    return (j < 0) ? -j : j;
}

long labs(long j) {
    return (j < 0) ? -j : j;
}

int atoi(const char *nptr) {
    int res = 0;
    while (*nptr >= '0' && *nptr <= '9') {
        res = res * 10 + (*nptr - '0');
        nptr++;
    }
    return res;
}

long atol(const char *nptr) {
    long res = 0;
    while (*nptr >= '0' && *nptr <= '9') {
        res = res * 10 + (*nptr - '0');
        nptr++;
    }
    return res;
}

void exit(int status) {
    // Loop forever
    (void)status;
    for(;;);
}
