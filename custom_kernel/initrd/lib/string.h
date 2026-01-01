#ifndef _STRING_H
#define _STRING_H

/* Nano-C String Library */

int strlen(char *s) {
    int i = 0;
    while (s[i] != 0) i++;
    return i;
}

void strcpy(char *dest, char *src) {
    int i = 0;
    while (src[i] != 0) {
        dest[i] = src[i];
        i++;
    }
    dest[i] = 0;
}

int strcmp(char *s1, char *s2) {
    int i = 0;
    while (s1[i] && s2[i]) {
        if (s1[i] != s2[i]) return s1[i] - s2[i];
        i++;
    }
    return s1[i] - s2[i];
}

void memset(void *ptr, int val, int size) {
    char *p = (char*)ptr;
    int i = 0;
    while (i < size) {
        p[i] = (char)val;
        i++;
    }
}

void memcpy(void *dest, void *src, int size) {
    char *d = (char*)dest;
    char *s = (char*)src;
    int i = 0;
    while (i < size) {
        d[i] = s[i];
        i++;
    }
}

void strcat(char *dest, char *src) {
    int len = strlen(dest);
    int i = 0;
    while (src[i]) {
        dest[len + i] = src[i];
        i++;
    }
    dest[len + i] = 0;
}

#endif
