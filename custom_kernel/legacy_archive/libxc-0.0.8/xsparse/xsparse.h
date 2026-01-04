
#ifndef  H_XSPARSE
#define  H_XSPARSE  ("1.0.0")

#include "xdict/xdict.h"

typedef struct xsparse_t xsparse_t;

#ifdef __cplusplus
extern "C" {
#endif

   xsparse_t *xsparse_new (void);
   void xsparse_del (xsparse_t *sa);
   void *xsparse_set (xsparse_t *sa, size_t index, void *value);
   void *xsparse_get (xsparse_t *sa, size_t index);
   void xsparse_iterate (xsparse_t *sa, void (*fptr) (size_t, void *));
   size_t xsparse_last (xsparse_t *sa);
   size_t xsparse_count (xsparse_t *sa);
 
#ifdef __cplusplus
};
#endif

#endif
