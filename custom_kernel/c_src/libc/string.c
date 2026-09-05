#include "string.h"
#include "stdlib.h"

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

char *strncat(char *dest, const char *src, size_t n) {
    char *ptr = dest + strlen(dest);
    while (n-- && *src) {
        *ptr++ = *src++;
    }
    *ptr = '\0';
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

char *strdup(const char *s) {
    size_t len = strlen(s) + 1;
    char *new = malloc(len);
    if (!new) return NULL;
    return memcpy(new, s, len);
}

char *strstr(const char *haystack, const char *needle) {
    if (!*needle) return (char *)haystack;
    for (; *haystack; haystack++) {
        if (*haystack == *needle) {
             const char *h = haystack;
             const char *n = needle;
             while (*h && *n && *h == *n) {
                 h++; n++;
             }
             if (!*n) return (char *)haystack;
        }
    }
    return NULL;
}

size_t strspn(const char *s, const char *accept) {
    const char *p;
    const char *a;
    size_t count = 0;
    for (p = s; *p != '\0'; ++p) {
        for (a = accept; *a != '\0'; ++a) {
            if (*p == *a)
                break;
        }
        if (*a == '\0')
            return count;
        ++count;
    }
    return count;
}

size_t strcspn(const char *s, const char *reject) {
    const char *p;
    const char *r;
    size_t count = 0;
    for (p = s; *p != '\0'; ++p) {
        for (r = reject; *r != '\0'; ++r) {
            if (*p == *r)
                return count;
        }
        ++count;
    }
    return count;
}

char *strtok(char *str, const char *delim) {
    static char *saveptr;
    if (str != NULL) {
        saveptr = str;
    }
    if (saveptr == NULL || *saveptr == '\0') {
        return NULL;
    }
    char *token_start = saveptr + strspn(saveptr, delim);
    if (*token_start == '\0') {
        saveptr = NULL;
        return NULL;
    }
    char *token_end = token_start + strcspn(token_start, delim);
    if (*token_end != '\0') {
        *token_end = '\0';
        saveptr = token_end + 1;
    } else {
        saveptr = token_end;
    }
    return token_start;
}
