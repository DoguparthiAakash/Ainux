#ifndef H_XLEX
#define H_XLEX    ("1.0.0")

#include <stdbool.h>

typedef struct xlex_t xlex_t;

#ifdef __cplusplus
extern "C" {
#endif

   xlex_t *xlex_new (char *name);
   size_t xlex_add (xlex_t *xl, char *re,
                    void (*fptr) (char *,void *,size_t,size_t));
   int xlex_exec (xlex_t *xl, const char *line, size_t line_num,
                  void *extra_arg);
   void xlex_del (xlex_t *xl);

#ifdef __cplusplus
};
#endif      /* end of function prototypes */


#endif      /* end of header              */
