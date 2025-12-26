#ifndef H_XMAP
#define H_XMAP    ("1.0.0")

#include <stdlib.h>

#include "xvector/xvector.h"

typedef struct xmap_t xmap_t;

#ifdef __cplusplus
extern "C" {
#endif

   xmap_t *xmap_new (void *(*cpyfunc) (void *), void (*freefunc) (void *));
   void *xmap_insert (xmap_t *xm, size_t key, void *value);
   void *xmap_replace (xmap_t *xm, size_t key, void *value);
   void xmap_del (xmap_t *xm);
   bool xmap_remove (xmap_t *xm, size_t key);
   const void *xmap_find (xmap_t *xm, size_t key);
   void **xmap_map (xmap_t *xm, void *(*pred) (size_t, void **));
   size_t *xmap_getkeys (xmap_t *xm);

#ifdef __cplusplus
};
#endif  


#endif 
