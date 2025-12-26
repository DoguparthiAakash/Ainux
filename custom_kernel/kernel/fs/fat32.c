#include "fat32.h"
#include "../mm/heap.h"

extern void kprint(const char *msg);

/* For now, we'll create a simple stub that uses InitRD */
/* TODO: Add real disk driver and FAT32 parsing */

static int fat32_initialized = 0;

/* Helper: String compare */
static int str_cmp(const char *s1, const char *s2) {
    while (*s1 && *s2 && *s1 == *s2) {
        s1++;
        s2++;
    }
    return *s1 - *s2;
}

/* Helper: String copy */
static void str_cpy(char *dest, const char *src, size_t max) {
    size_t i = 0;
    while (i < max - 1 && src[i]) {
        dest[i] = src[i];
        i++;
    }
    dest[i] = '\0';
}

int fat32_init(void) {
    kprint("[FAT32] Initializing (stub)...\n");
    
    /* TODO: 
     * 1. Detect disk via ATA/AHCI
     * 2. Read boot sector
     * 3. Parse FAT tables
     * 4. Find root directory
     */
    
    fat32_initialized = 1;
    kprint("[FAT32] Initialized (using InitRD fallback)\n");
    return 0;
}

void fat32_list_files(void) {
    if (!fat32_initialized) {
        kprint("[FAT32] Not initialized!\n");
        return;
    }
    
    kprint("[FAT32] File listing not yet implemented\n");
    kprint("[FAT32] Use InitRD commands instead\n");
}

int fat32_read_file(const char *filename, uint8_t **data, uint32_t *size) {
    if (!fat32_initialized) {
        return -1;
    }
    
    kprint("[FAT32] File reading not yet implemented\n");
    return -1;
}
