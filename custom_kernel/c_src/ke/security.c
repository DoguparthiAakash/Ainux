#include "security.h"
#include "log.h"
#include "ke/sched/sched.h"

int validate_user_pointer(const void *ptr, uint64_t size) {
    uint64_t addr = (uint64_t)ptr;
    
    if (addr == 0) return 0; // NULL checking
    if (addr > USER_SPACE_LIMIT) return 0;
    if (addr + size > USER_SPACE_LIMIT) return 0;
    if (addr + size < addr) return 0; // Overflow check
    
    return 1;
}

int validate_user_string(const char *ptr, uint64_t max_len) {
    if (!validate_user_pointer(ptr, 1)) return 0;
    
    /* We can't easily scan length without potentially hitting page fault if it crosses page boundary
       into invalid memory. But we can check up to max_len conservatively. */
    
    // For "Brutal" mode, we might want to check every page? 
    // Simply checking the start is < LIMIT is basic.
    // If string is long and crosses into kernel space, we catch it.
    
    for (uint64_t i = 0; i < max_len; i++) {
        if ((uint64_t)(ptr + i) > USER_SPACE_LIMIT) return 0;
        // if (ptr[i] == 0) return 1; // Unsafe read if we don't know it's mapped!
        // Usage of this function implies we might read it.
    }
    
    return 1;
}

int copy_from_user(void *dest, const void *src, uint64_t size) {
    if (!validate_user_pointer(src, size)) {
        kprint_color(KLOG_COLOR_RED, "[SECURITY] Invalid User Pointer (Read) detected!\n");
        return -1;
    }
    
    // Perform copy
    uint8_t *d = (uint8_t*)dest;
    const uint8_t *s = (const uint8_t*)src;
    for (uint64_t i = 0; i < size; i++) {
        d[i] = s[i];
    }
    return 0;
}

int copy_to_user(void *dest, const void *src, uint64_t size) {
    if (!validate_user_pointer(dest, size)) {
        kprint_color(KLOG_COLOR_RED, "[SECURITY] Invalid User Pointer (Write) detected!\n");
        return -1;
    }
    
    uint8_t *d = (uint8_t*)dest;
    const uint8_t *s = (const uint8_t*)src;
    for (uint64_t i = 0; i < size; i++) {
        d[i] = s[i];
    }
    return 0;
}
