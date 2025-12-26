
#ifndef H_XSARRAY
#define H_XSARRAY ("1.0.0")

typedef struct xsarray_t xsarray_t;

#include "xvector/xvector.h"

#ifdef __cplusplus
extern "C" {
#endif

   xsarray_t *xsarray_new (int (*comparison) (void *, void *));
   void xsarray_free (xsarray_t *xsa);
   size_t xsarray_length (xsarray_t *xsa);
   void *xsarray_index (xsarray_t *xsa, size_t index);

   void *xsarray_insert (xsarray_t *xsa, void *element);
   void *xsarray_find (xsarray_t *xsa, void *needle);
   void *xsarray_remove (xsarray_t *xsa, void *item);
   void *xsarray_purge (xsarray_t *xsa, void *item);
   xvector_t *xsarray_array (xsarray_t *xsa);

#ifdef __cplusplus
};
#endif

#endif
