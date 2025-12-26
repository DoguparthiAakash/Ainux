#ifndef _STDLIB_H
#define _STDLIB_H

#include <stddef.h>

void *malloc(size_t size);
void free(void *ptr);
void *calloc(size_t nmemb, size_t size);
void *realloc(void *ptr, size_t size);

int abs(int j);
long labs(long j);
int atoi(const char *nptr);
long atol(const char *nptr);
void exit(int status);

#endif
