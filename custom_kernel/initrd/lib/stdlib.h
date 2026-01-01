#ifndef _STDLIB_H
#define _STDLIB_H

/* Nano-C Standard Library */

/* Memory Management */
/* malloc and free are built-in keywords/intrinsics in Nano-C */

void *calloc(int num, int size) {
    int total = num * size;
    void *ptr = malloc(total);
    if (ptr) {
        memset(ptr, 0, total);
    }
    return ptr;
}

void exit(int status) {
    /* exit keyword */
    exit(status);
}

/* Random */
int rand() {
    /* rand keyword */
    return rand();
}

#endif
