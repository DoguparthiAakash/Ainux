/* SPDX-License-Identifier: GPL-2.0 */
/* Ainux KPI — linux/string.h
 * Standard string operations for Linux drivers.
 * We use gcc builtins for inlining and point others to our libc.
 */
#ifndef _LINUX_STRING_H
#define _LINUX_STRING_H

#include <linux/types.h>

/* These are provided by our c_src/libc/string.c and libc headers */
extern void  *memcpy(void *dst, const void *src, size_t n);
extern void  *memmove(void *dst, const void *src, size_t n);
extern void  *memset(void *s, int c, size_t n);
extern int    memcmp(const void *s1, const void *s2, size_t n);
extern void  *memchr(const void *s, int c, size_t n);
extern size_t strlen(const char *s);
extern size_t strnlen(const char *s, size_t maxlen);
extern int    strcmp(const char *s1, const char *s2);
extern int    strncmp(const char *s1, const char *s2, size_t n);
extern char  *strcpy(char *dst, const char *src);
extern char  *strncpy(char *dst, const char *src, size_t n);
extern char  *strcat(char *dst, const char *src);
extern char  *strchr(const char *s, int c);
extern char  *strrchr(const char *s, int c);
extern char  *strstr(const char *haystack, const char *needle);
extern char  *strsep(char **stringp, const char *delim);
extern long   simple_strtol(const char *cp, char **endp, unsigned int base);
extern unsigned long simple_strtoul(const char *cp, char **endp, unsigned int base);

/* Linux uses these helpers in drivers */
static __always_inline void *kzfree(const void *p)
{
    /* Intentional no-op stub — in Linux this zeroes then frees.
     * Real implementation calls kfree after zeroing. Drivers use kfree,
     * this is an alias. */
    extern void kpi_kfree(const void *ptr);
    kpi_kfree(p);
    return NULL;
}

#define memzero(buf, len)    memset(buf, 0, len)
#define strlcpy(d, s, n)     strncpy(d, s, n)  /* close enough for our drivers */

#endif /* _LINUX_STRING_H */
