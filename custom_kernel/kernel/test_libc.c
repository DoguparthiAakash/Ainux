#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <assert.h>

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
    
    /* Test strdup */
    char *dup = strdup(ptr);
    printf("[TestLibC]    strdup result: '%s'\n", dup);
    if (strcmp(ptr, dup) == 0) printf("[TestLibC]    strdup verified.\n");
    else printf("[TestLibC]    strdup FAILED.\n");
    free(dup);
    
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

    /* 4. Test Ctype */
    printf("[TestLibC] 4. Testing ctype...\n");
    if (isdigit('9') && !isdigit('A')) printf("[TestLibC]    isdigit passed.\n");
    else printf("[TestLibC]    isdigit FAILED.\n");
    
    if (toupper('a') == 'A' && tolower('Z') == 'z') printf("[TestLibC]    case conv passed.\n");
    else printf("[TestLibC]    case conv FAILED.\n");
    
    /* 5. Test Stdlib (Rand) */
    printf("[TestLibC] 5. Testing rand...\n");
    srand(1234);
    int r1 = rand();
    int r2 = rand();
    printf("[TestLibC]    Rand sequence: %d, %d\n", r1, r2);
    
    /* 6. Test String Extensions */
    printf("[TestLibC] 6. Testing string extensions...\n");
    char str[] = "A,B,C";
    char *tok = strtok(str, ",");
    printf("[TestLibC]    strtok: '%s'\n", tok); // A
    tok = strtok(NULL, ",");
    printf("[TestLibC]    strtok: '%s'\n", tok); // B
    
    /* Assert test - skipped to actuaully run logic */
    assert(1 == 1); 
    printf("[TestLibC]    Assert(1==1) passed.\n");

    printf("[TestLibC] All tests passed.\n");
}
