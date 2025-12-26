#ifndef H_TEST2
#define H_TEST2

#include <stdio.h>

/* extern "C" int testing (test_t one, test_t two); */
#ifdef __cplusplus
extern "C" {
#endif

int testing (char * one, char * two);
int (*etest) (char *, char *);

#ifdef __cplusplus
};
#endif

#endif


