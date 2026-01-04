

#include "xvector/xvector.h"
#include "xtree/xtree.h"

/**
 * \file
 * 
 * \brief Create and traverse a tree structure.
 *
 * A C library to create and traverse multi-children trees. Each child node
 * points to a single parent node and each parent node may have multiple
 * child nodes. Child nodes are stored in an ordered list for each parent,
 * in the order that they are created. Each node whether parent or child
 * may also store a payload - the element of data that is to be stored in
 * the tree.
 * 
 * xtree is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 */

typedef struct node_t node_t;

struct node_t {
   node_t *parent;
   xvector_t *children;
   void *payload; 
};

/**
 * \brief Create and add a node to a parent node
 *
 * Creates and returns a new node, with \a payload as the payload and \a parent 
 * as the parent. If \a parent is \a NULL then the node that is created and
 * returned is a root node of a tree and can be used as a parent for subsequent
 * \a xtree_add_child() invocations.
 *
 * @param[in] parent The parent of the node to be created. If parent is NULL
 *    then the returned node is a toplevel node of a new tree
 * @param[in] element The payload to be stored in the newly created node
 *
 * @return On success a newly created node with payload set to \a payload
 * and parent set to \a parent is returned. On failure \a NULL is returned.
 */
xtree_t *xtree_add_child (xtree_t *parent, void *element)
{
   node_t *newnode = malloc (sizeof *newnode);
   if (!newnode) return NULL;
   newnode->parent = parent;
   newnode->payload = element;
   newnode->children = NULL;
   if (!parent) return newnode;
   xvector_t *tmp = xvector_ins_tail (parent->children, newnode);
   if (!tmp) {
      free (newnode);
      return NULL;
   }
   parent->children = tmp;
   return newnode;
}

/**
 * \brief Deletes a node and all it's children
 *
 * Deletes the node \a node, together with all it's children. All resources
 * used by the node and it's children other than the payload data is freed.
 *
 * @param[in] node The node to delete
 *
 * @return On success the payload of the deleted node is returned (the 
 * payload of the node's children are discarded). It might be useful to the 
 * caller to receive the last payload value of the node before it was 
 * freed. On failure \a NULL is returned.
 */
void *xtree_del (xtree_t *node)
{
   for (size_t i=0; i<XVECT_LENGTH (node->children); i++) {
      node_t *child = XVECT_INDEX (node->children, i);
      xtree_del (child);
   }
   void *element = node->payload;
   xvector_free (node->children);
   free (node);
   return element;
}

/**
 * \brief Returns a child node of the specified node
 *
 * Returns the \a i'th child of this node (children in a node are stored
 * in the order that they are created).
 *
 * @param[in] node The node that contains the child that the caller is 
 *    asking for.
 * @param[in] i The index of the child within the \a node specified above
 *
 * @return On success the \a i'th child of this node is returned. On failure
 * \a NULL is returned.
 */
xtree_t *xtree_get_child (xtree_t *node, size_t i)
{
   if (!node) return NULL;
   if (i > XVECT_LENGTH (node->children)) return NULL;
   return XVECT_INDEX (node->children, i);
}

/**
 * \brief Retrieve the parent of this node
 *
 * Returns the parent node of \a node, if it exists. If \a node or it's
 * parent does not exist, returns \a NULL instead.
 *
 * @param[in] node The node to examine for a parent
 *
 * @return \a NULL is returned if either \a node or it's parent does not
 * exist, in all other cases the parent of the given node \a node is returned.
 */
xtree_t *xtree_get_parent (xtree_t *node)
{
   if (!node) return NULL;
   return node->parent;
}

/**
 * \brief Retrieves the payload stored in this node
 *
 * Retrieves the payload stored in this node. If neither the node nor the
 * payload exist, \a NULL is returned.
 *
 * @param[in] node The node to examine for a payload
 *
 * @return \a NULL is returned if either the node \a node or it's payload
 * doesn't exist. In all other cases the payload is returned.
 */
xtree_t *xtree_get_payload (xtree_t *node)
{
   if (!node) return NULL;
   return node->payload;
}

/**
 * \brief Traverse the tree, applying a function to each node.
 *
 * The tree \a node is traversed and on each visit to a node the function 
 * \a func is applied on the payload of the node. The tree is traversed
 * bottom up, one branch at a time starting with the first stored child
 * on each parent. 
 * 
 * The function \a func should accept two arguments and return nothing.
 * The first argument to \a func is the payload of the node and the second
 * argument is the caller supplied \a extra argument. In this way, using
 * the \a extra argument, the caller has flexibility to write predicate
 * functions for side-effects.
 *
 * @param[in] node The tree to traverse
 * @param[in] func The function to apply to each node's payload as the
 *    tree is traversed. The first argument to func is the node's payload
 *    data and the second argument is the \a extra argument supplied to
 *    \a xtree_apply()
 * @param[in] extra An extra argument that forms the second argument when
 *    \a func is called on a node's payload (the first argument to \a func
 *    is the node's payload data)
 *
 * @return Nothing.
 *
 * \a Example:
 *
 * \verbatim
   // Assuming that the tree is being used to store char pointers 
   static void print_payload (void *p, void *f)
   {
      char *payload = p;
      FILE *outf = f;
      fprintf (outf, "%s\n", payload);
   }

   int main (void)
   {
      xtree_t *tree_of_strings;
      ...
      // Print out entire tree
      xtree_apply (tree_of_strings, print_payload, stdout);
      return EXIT_SUCCESS;
   }
 * \endverbatim
 */
void xtree_apply (xtree_t *node, void (*func) (void *, void *), void *extra)
{
   for (size_t i=0; i<XVECT_LENGTH (node->children); i++) {
      node_t *child = XVECT_INDEX (node->children, i);
      xtree_apply (child, func, extra);
   }
   func (node->payload, extra);
}

/**
 * \brief Traverse the tree, applying a function to each node and storing
 *    the result
 *
 * The tree \a node is traversed and on each visit to a node in the tree 
 * the function 
 * \a func is applied on the payload of the node. The tree is traversed
 * bottom up, one branch at a time starting with the first stored child
 * on each parent. The result of each invocation of \a func is stored in
 * a tree which is returned to the caller.
 * 
 * The function \a func should accept two arguments and return a void pointer
 * \a(\a void \a *\a ).
 * The first argument to \a func is the payload of the node and the second
 * argument is the caller supplied \a extra argument. In this way, using
 * the \a extra argument, the caller has the flexibility to write predicate
 * functions. For example, to find a certain value within the tree the
 * caller can supply \a func as a comparison function and \a extra as the
 * value to be compared against.
 *
 * @param[in] node The tree to traverse
 * @param[in] func The function to apply to each node's payload as the
 *    tree is traversed. The first argument to func is the node's payload
 *    data and the second argument is the \a extra argument supplied to
 *    \a xtree_map().
 * @param[in] extra An extra argument that forms the second argument when
 *    \a func is called on a node's payload (the first argument to \a func
 *    is the node's payload data
 *
 * @return On success, a tree identical in structure to the original tree but
 * with the payload for each node being the result of \a func(payload,extra).
 * Any branch in the tree that experienced failure is pruned from the result.
 *
 *
 * \a Example:
 *
 * \verbatim
   // Assuming that the tree is being used to store char pointers 
   static void find_payload (void *p, void *n)
   {
      char *payload = p,
           *needle = n;
      return (void *)(strcmp (payload, needle));
   }

   int main (void)
   {
      xtree_t *tree_of_strings;
      ...
      xtree_t *hw = xtree_map (tree_of_strings, find_payload, "Hello World");
      // hw now has all the results of all the strcmp for each node in 
      // the original tree_of_strings
      ...
      return EXIT_SUCCESS;
   }
 * \endverbatim
 */
xtree_t *xtree_map (xtree_t *node, void *(*func) (void *, void *), 
                                   void *extra)
{
   node_t *root = xtree_add_child (NULL, func (node->payload, extra));
   for (size_t i=0; i<XVECT_LENGTH (node->children); i++) {
      node_t *tmp = XVECT_INDEX (node->children, i);
      if (!tmp) goto error;
      node_t *result = xtree_map (tmp, func, extra);
      xvector_t *tmp2 = xvector_ins_tail (root->children, result);
      if (!tmp2) goto error;
      root->children = tmp2;
      result->parent = root;
   }
   return root;
error:
   if (root) xtree_del (root);
   return NULL;
}
