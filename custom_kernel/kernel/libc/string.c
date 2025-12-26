#include "string.h"

void *memset(void *dest, int val, size_t len) {
    unsigned char *ptr = (unsigned char *)dest;
    while (len-- > 0)
        *ptr++ = val;
    return dest;
}

void *memcpy(void *dest, const void *src, size_t len) {
    char *d = (char *)dest;
    const char *s = (const char *)src;
    while (len--)
        *d++ = *s++;
    return dest;
}

void *memmove(void *dest, const void *src, size_t len) {
    char *d = (char *)dest;
    const char *s = (const char *)src;
    if (d < s) {
        while (len--)
            *d++ = *s++;
    } else {
        d += len;
        s += len;
        while (len--)
            *--d = *--s;
    }
    return dest;
}

int memcmp(const void *s1, const void *s2, size_t n) {
    const unsigned char *p1 = s1;
    const unsigned char *p2 = s2;
    while(n--) {
        if(*p1 != *p2) return *p1 - *p2;
        p1++; p2++;
    }
    return 0;
}

size_t strlen(const char *str) {
    size_t len = 0;
    while (str[len])
        len++;
    return len;
}

char *strcpy(char *dest, const char *src) {
    char *d = dest;
    while ((*d++ = *src++));
    return dest;
}

char *strncpy(char *dest, const char *src, size_t n) {
    char *d = dest;
    while (n > 0 && *src) {
        *d++ = *src++;
        n--;
    }
    while (n-- > 0)
        *d++ = 0;
    return dest;
}

int strcmp(const char *s1, const char *s2) {
    while (*s1 && (*s1 == *s2)) {
        s1++;
        s2++;
    }
    return *(const unsigned char *)s1 - *(const unsigned char *)s2;
}

int strncmp(const char *s1, const char *s2, size_t n) {
    while (n && *s1 && (*s1 == *s2)) {
        s1++;
        s2++;
        n--;
    }
    if (n == 0) return 0;
    return *(const unsigned char *)s1 - *(const unsigned char *)s2;
}

char *strcat(char *dest, const char *src) {
    char *ptr = dest + strlen(dest);
    while ((*ptr++ = *src++));
    return dest;
}

char *strchr(const char *s, int c) {
    while (*s != (char)c) {
        if (!*s++)
            return NULL;
    }
    return (char *)s;
}

char *strrchr(const char *s, int c) {
    const char *last = NULL;
    do {
        if (*s == (char)c)
            last = s;
    } while (*s++);
    return (char *)last;
}
