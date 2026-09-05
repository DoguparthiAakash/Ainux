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

/* Random Number Generation */
#define RAND_MAX 32767
int rand(void);
void srand(unsigned int seed);
char *itoa(int value, char *str, int base);

#endif
