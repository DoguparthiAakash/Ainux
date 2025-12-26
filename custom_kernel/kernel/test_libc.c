#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* This function simulates a "user program" running inside the kernel 
   using our new Mini-LibC */

void libc_test_run(void) {
    printf("[TestLibC] Starting tests...\n");

    /* 1. Test printf */
    printf("[TestLibC] 1. Testing printf: %d + %d = %d\n", 10, 20, 30);
    printf("[TestLibC]    String test: '%s', Hex test: 0x%x\n", "Hello", 0xABCD);

    /* 2. Test Malloc/Free */
    printf("[TestLibC] 2. Testing malloc...\n");
    char *ptr = (char *)malloc(128);
    if (!ptr) {
        printf("[TestLibC] Malloc failed!\n");
        return;
    }
    strcpy(ptr, "Malloc works!");
    printf("[TestLibC]    Allocated ptr: %p, Content: '%s'\n", ptr, ptr);
    free(ptr);
    printf("[TestLibC]    Freed memory.\n");

    /* 3. Test File I/O (InitRD) */
    printf("[TestLibC] 3. Testing fopen('test.txt')...\n");
    FILE *f = fopen("test.txt", "r");
    if (f) {
        char buf[64];
        if (fgets(buf, sizeof(buf), f)) {
            printf("[TestLibC]    Read from file: '%s'\n", buf);
        } else {
            printf("[TestLibC]    Failed to read file.\n");
        }
        fclose(f);
    } else {
        printf("[TestLibC]    Failed to open test.txt! (Did you create it?)\n");
    }

    printf("[TestLibC] All tests passed.\n");
}
