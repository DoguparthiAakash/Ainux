
#ifndef H_XTREE
#define H_XTREE ("1.0.0")

typedef struct node_t xtree_t;

#ifdef __cplusplus
extern "C" {
#endif

   xtree_t *xtree_add_child (xtree_t *parent, void *element);
   void *xtree_del (xtree_t *node);
   xtree_t *xtree_get_child (xtree_t *node, size_t i);
   xtree_t *xtree_get_parent (xtree_t *node);
   xtree_t *xtree_get_payload (xtree_t *node);

   void xtree_apply (xtree_t *node, void (*func) (void *, void *), void *extra);
   xtree_t *xtree_map (xtree_t *node, void *(*func) (void *, void *), 
                                                                   void *extra);

#ifdef __cplusplus
};
#endif

#endif
