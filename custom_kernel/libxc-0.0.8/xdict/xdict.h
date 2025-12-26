
#ifndef  H_XDICT
#define  H_XDICT  ("1.0.0")

typedef struct xdict_t xdict_t;

#ifdef __cplusplus
extern "C" {
#endif

   xdict_t *xdict_new (int (*cmp) (void *, void *), void *(*cpy) (void *),
                       void (*del) (void *));
   void xdict_del (xdict_t *dict);

   void *xdict_set (xdict_t *dict, void *index, void *element);
   void *xdict_get (xdict_t *dict, void *index);

   void xdict_iterate (xdict_t *dict, void (*fptr) (void *, void *));
   void *xdict_map (xdict_t *dict, void *(*fptr) (void *, void *));

   char **xdict_list_indices (xdict_t *dict);

#ifdef __cplusplus
};
#endif



#endif
