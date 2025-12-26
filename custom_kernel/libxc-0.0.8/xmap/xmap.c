/**
 * \file 
 *
 * \brief Implementation of a map container datatype for C
 *
 * A map datatype for C. \a xmap permits the caller to store arbitrary
 * data in a map with a \a size_t key. 
 *
 * xmap is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 *
 */

#include <stdlib.h>

#include "xerror/xerror.h"
#include "xvector/xvector.h"
#include "xmap.h"

/* TODO: replace this with a binary linked-list when I get the opportunity.
 * Until I do (and xmap provides okay performance for my needs), this
 * will have to do.
 */

struct map_t {
   size_t key;
   void *value;
};

typedef struct map_t map_t;

struct xmap_t {
   void *(*cpyfunc) (void *);
   void (*freefunc) (void *);
   xvector_t *array;
};

static int xmap_local_cmp (void *rhs, void *lhs)
{
   map_t *r = rhs, *l = lhs;
   if (r->key>l->key) return 1;
   if (r->key<l->key) return -1;
   return 0;
}

static map_t *xmap_local_insert
   (xmap_t *xm, size_t key, void *value, bool with_replacement)
{
   map_t *newmap = malloc (sizeof *newmap);
   if (!newmap) {
      return NULL;
   }
   newmap->key = key;
   newmap->value = xm->cpyfunc ? xm->cpyfunc (value) : value;
   if (!newmap->value) {
      XLOG ("Unable to clone value for storage\n");
      free (newmap);
      return NULL;
   }

   // Does item itself already exist?
   void **elm = xvector_find_first (xm->array, newmap, xmap_local_cmp);
   if (elm) {
      // Caller doesn't want to replace existing value
      if (!with_replacement) {
         free (newmap->value);
         free (newmap);
         return NULL;
      }
      // Caller does want to replace existing value
      map_t *tmp = *elm;
      xm->freefunc ? xm->freefunc (tmp->value) : 0;
      free (tmp);
      *elm = newmap;
      return newmap->value;
   } else {
      // Item does not already exist
      xvector_t *tmp = xvector_ins_tail (xm->array, newmap);
      if (!tmp) {
         XLOG ("Unable to insert new map\n");
         free (newmap->value);
         free (newmap);
         return NULL;
      }
      xm->array = tmp;
      return newmap->value;
   }
   XLOG ("Reached default case, program corrupt\n");
   return NULL;
}

/**
 * \brief Create a new xmap data structure.
 *
 * Creates a new map data structure container. The \a xmap_t* return value
 * must be freed with a call to xmap_del. Xmap stores each element as a
 * \a key/value pair with the \a key being of type \a size_t and the value
 * being a pointer to a user-allocated C object. The caller must supply
 * functions for cloning/copying and freeing these C objects. The key may
 * not be zero.
 *
 * \a cpyfunc() is a function that takes a single void * as an argument and
 * returns a copy of the element pointed to by the void *. The caller must
 * ensure that \a cpyfunc() returns an object that is a separate copy of
 * the argument.
 * 
 * \a freefunc() is a function that takes a single void * as an argument
 * and free's all resources represented by that void *.
 * 
 * \a cpyfunc is a function that the
 * xmap library uses to clone the elements stored in the map.
 *
 * \sa xmap_insert(), xmap_replace(), xmap_del(), xmap_remove(),
 * xmap_find(), xmap_map(), xmap_getkeys(),
 *
 * @param[in] cpyfunc The caller-supplied function that is responsible for
 *    making copies of existing values stored in the map. If this value is
 *    NULL then a bitwise shallow copy is performed on the value being 
 *    inserted into the container.
 * @param[in] freefunc The caller-supplied function that is responsible for
 *    freeing all resources used by the values stored in the map. If this
 *    argument is NULL then no freeing of values will occur when \a xmap
 *    moves or deletes any of the values stored in the container.
 *
 * @return On success a new \a xmap_t object is returned. This object must be
 *    freed using \a xmap_del(). On failure NULL is returned.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xmap_t *xmap1 = xmap_new (xstrdup, free);

      xmap_insert (xmap1, 1, "one");
      xmap_insert (xmap1, 2, "two");
      xmap_insert (xmap1, 3, "three");

      xmap_del (xmap1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
xmap_t *xmap_new (void *(*cpyfunc) (void *), void (*freefunc) (void *))
{
   xmap_t *ret = malloc (sizeof *ret);
   if (!ret) goto errorexit;
   ret->array = NULL;
   ret->cpyfunc = cpyfunc;
   ret->freefunc = freefunc;
   return ret;

errorexit:
   xmap_del (ret); 
   return NULL;
}

/**
 * \brief Inserts a new key/value pair into the map.
 *
 * Inserts the given key/value pair into the map. If the key aleady exists
 * in the container the function fails and returns NULL and the original
 * value stored with the given key is maintained unchanged. A copy of the 
 * argument \a value is made before storing the key/value pair in the
 * xmap container. Check the documentation for xmap_new() for a description
 * of the \a cpyfunc() and \a freefunc() functions.
 *
 * \sa xmap_new(), xmap_replace(), xmap_del(), xmap_remove(),
 * xmap_find(), xmap_map(), xmap_getkeys(),
 *
 * @param[in] xm The container to work on.
 *
 * @param[in] key The key to use to identify this entry in the container. If 
 *    this key already exists within the \a xmap container then this function
 *    returns NULL and no changes are made to the container. The key may not
 *    be zero.
 *
 * @param[in] value The value to store under this key. A copy of the value is
 *    stored and not the one supplied by caller as this argument. The copy
 *    of \a value is made using the \a cpyfunc() argument used when the 
 *    container was created with \a xmap_new().
 *
 * @return On success a pointer to the newly created value is returned.
 *    On failure NULL is returned.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xmap_t *xmap1 = xmap_new (xstrdup, free);

      char *one = xmap_insert (xmap1, 1, "one");
      char *two = xmap_insert (xmap1, 2, "two");
      char *three = xmap_insert (xmap1, 3, "three");
      // This fails and returns NULL, which is assigned to 'four'
      char *four = xmap_insert (xmap1, 3, "four");

      xmap_del (xmap1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
void *xmap_insert (xmap_t *xm, size_t key, void *value)
{
   return xmap_local_insert (xm, key, value, false);
}

/**
 * \brief Inserts or replaces new key/value pair in the map
 *
 * Inserts the given key/value pair into the map. If the key aleady exists
 * in the container the existing value is freed using the function 
 * \a freefunc() specified in \a xmap_new().  A copy of the 
 * argument \a value is made before storing the key/value pair in the
 * xmap container. Check the documentation for xmap_new() for a description
 * of the \a cpyfunc() and \a freefunc() functions.
 *
 *
 * \sa xmap_new(), xmap_insert(), xmap_del(), xmap_remove(),
 * xmap_find(), xmap_map(), xmap_getkeys(),
 *
 * @param[in] xm The container to work on.
 *
 * @param[in] key The key to use to identify this entry in the container. If 
 *    this key already exists within the \a xmap container then it is replaced
 *    and the map clears up and releases memory as appropriate. The key may
 *    not be zero.
 *
 * @param[in] value The value to store under this key. A copy of the value is
 *    stored and not the one supplied by caller as this argument. The copy
 *    of \a value is made using the \a cpyfunc() argument used when the 
 *    container was created with \a xmap_new().
 *
 * @return On success a pointer to the newly created value is returned.
 *    On failure NULL is returned.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xmap_t *xmap1 = xmap_new (xstrdup, free);

      char *one = xmap_replace (xmap1, 1, "one");
      char *two = xmap_replace (xmap1, 2, "two");
      char *three = xmap_replace (xmap1, 3, "three");
      // This succeeds and 'four' is stored under the key '3'
      char *four = xmap_replace (xmap1, 3, "four");

      xmap_del (xmap1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
void *xmap_replace (xmap_t *xm, size_t key, void *value)
{
   return xmap_local_insert (xm, key, value, true);
}

/**
 * \brief Delete an xmap data structure.
 *
 * Deletes the \a xmap structure given in \a xm. All resources used by the
 * container including all stored keys and all stored values are released.
 * The freeing of values is performed by the \a freefunc() function supplied
 * to xmap_new(). There is no guaranteed order for the deletion of the
 * values.
 *
 * \sa xmap_new(), xmap_insert(), xmap_replace(), xmap_remove(),
 * xmap_find(), xmap_map(), xmap_getkeys(),
 *
 * @param[in] xm The xmap container to free. All values held in this container
 *    are freed using the freefunc() function supplied to \a xmap_new().
 *
 * @return Nothing.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xmap_t *xmap1 = xmap_new (xstrdup, free);

      xmap_insert (xmap1, 1, "one");
      xmap_insert (xmap1, 2, "two");
      xmap_insert (xmap1, 3, "three");

      xmap_del (xmap1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
void xmap_del (xmap_t *xm)
{
   if (xm) {
      for (size_t i=0; i<XVECT_LENGTH (xm->array); i++) {
         map_t *tmp = XVECT_INDEX (xm->array, i);
         xm->freefunc ? xm->freefunc (tmp->value) : 0;
         free (tmp);
      }
      xvector_free (xm->array);
      free (xm);
   }
}

/**
 * \brief Removes the specified key/value pair from the container.
 *
 * Removes the pair consisting of key \a key from the given container \a xm.
 * The value associated with \a key is freed using the \a freefunc() argument
 * given to \a xmap_new() when this container was created. 
 *
 * If \a key is not found in the container, \a false is returned and no changes
 * are made to the container.
 *
 * \sa xmap_new(), xmap_insert(), xmap_replace(), xmap_del(), 
 * xmap_find(), xmap_map(), xmap_getkeys(),
 *
 * @param[in] xm The \a xmap container to work on.
 * @param[in] key The key to use to identify which key/value pair must be
 *    removed from the container.
 *
 * @return On success \a true is returned. On failure \a false is returned.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xmap_t *xmap1 = xmap_new (xstrdup, free);

      xmap_insert (xmap1, 1, "one");
      xmap_insert (xmap1, 2, "two");
      xmap_insert (xmap1, 3, "three");

      // Remove the middle item.
      xmap_remove (xmap1, 2);

      xmap_del (xmap1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
bool xmap_remove (xmap_t *xm, size_t key)
{
   for (size_t i=0; i<XVECT_LENGTH(xm->array); i++) {
      map_t *tmp = XVECT_INDEX (xm->array, i);
      if (tmp->key==key) {
         xm->freefunc ? xm->freefunc (tmp->value) : 0;
         free (tmp);
         xvector_del (xm->array, i);
         return true;
      }
   }
   return false;
}

/**
 * \brief Returns the value associated with \a key
 *
 * Returns, for the given container \a xm, the value stored with key \a key. If
 * the key does not exist NULL is returned.
 *
 * \sa xmap_new(), xmap_insert(), xmap_replace(), xmap_del(), xmap_replace(),
 * xmap_map(), xmap_getkeys(),
 *
 * @param[in] xm The \a xmap container to work on.
 * @param[in] key The key to use to identify the value to be returned.
 *
 * @return On success a pointed to the value is returned. The caller must not
 *    free this pointer. On failure NULL is returned.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xmap_t *xmap1 = xmap_new (xstrdup, free);

      xmap_insert (xmap1, 1, "one");
      xmap_insert (xmap1, 2, "two");
      xmap_insert (xmap1, 3, "three");

      // Returns the string 'two'
      char *two = xmap_find (xmap1, 2);

      xmap_del (xmap1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
const void *xmap_find (xmap_t *xm, size_t key)
{
   for (size_t i=0; i<XVECT_LENGTH (xm->array); i++) {
      map_t *tmp = XVECT_INDEX (xm->array, i);
      if (tmp->key==key) {
         return tmp->value;
      }
   }
   return NULL;
}

/**
 * \brief Maps the given predicate over all elements in the container.
 *
 * Maps the given predicate function \a pred over all items in the container
 * \a xm. The return value of predicate is stored in an array of void *. The
 * function \a pred MUST NOT return NULL, even in the event of an error. This
 * is because the returned value of \a xmap_map is an array of pointers which
 * is terminated with a NULL pointer. If \a pred were to return a NULL pointer
 * the caller will  not be able to determine where the returned array ends.
 *
 * The order of execution over each key/value pair is not guaranteed and 
 * must not be relied upon by the caller. The caller is responsible for
 * freeing the return value.
 *
 * \sa xmap_new(), xmap_insert(), xmap_replace(), xmap_del(), xmap_replace(),
 * xmap_find(), xmap_getkeys(),
 *
 * @param[in] xm The \a xmap container to work on.
 * @param[in] pred The predicate that will be executed for each of the 
 *    key/value pairs stored in the container.
 *
 * @return On success an array of void * is returned. Each element in the
 *    returned array is the return value of a single invocation of \a pred with
 *    the exception of the last element which is NULL and is used to signify
 *    the end of the array. 
 *    The order of execution over each key/value pair in the container is
 *    not guaranteed and the caller is responsible for freeing the returned
 *    array.
 *
 * \a Example:
 * \verbatim
   static void *p1 (size_t k, void *v) {
      printf ("%zu - %s\n", k, (char *)v);
      return v;
   }

   int main (void)
   {
      xmap_t *xmap1 = xmap_new (xstrdup, free);

      xmap_insert (xmap1, 1, "one");
      xmap_insert (xmap1, 2, "two");
      xmap_insert (xmap1, 3, "three");

      // Print out all elements
      char **results = xmap_map (xmap1, p1);
      free (results);

      xmap_del (xmap1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
void **xmap_map (xmap_t *xm, void *(*pred) (size_t, void **))
{
   void **ret = malloc (sizeof *ret * (XVECT_LENGTH (xm->array) + 1));
   ret [XVECT_LENGTH (xm->array)] = NULL;
   if (!ret) {
      XLOG ("Out of memory error\n");
      return NULL;
   }
   for (size_t i=0; i<XVECT_LENGTH (xm->array); i++) {
      map_t *tmp = XVECT_INDEX (xm->array, i);
      ret[i] = pred (tmp->key, &tmp->value);
   }
   return ret;
}

/**
 * \brief Returns all the keys stored in the container.
 *
 * Returns all the keys stored in the container as an allocated \a size_t
 * array terminated with a (size_t)0 value. The keys are not guaranteed to 
 * be in any particular order and the caller is responsible for freeing the
 * returned array.
 *
 * \sa xmap_new(), xmap_insert(), xmap_replace(), xmap_del(), xmap_replace(),
 * xmap_find(), xmap_map(),
 *
 * @param[in] xm The \a xmap container to examine for keys.
 *
 * @return On success an array of size_t * is returned. Each element in the
 *    returned array is a key into the container \a xm with the
 *    the exception of the last element which is 0 and is used to signify
 *    the end of the array. 
 *    The order of keys in the container is
 *    not guaranteed and the caller is responsible for freeing the returned
 *    array.
 *
 * \a Example:
 * \verbatim

   int main (void)
   {
      xmap_t *xmap1 = xmap_new (xstrdup, free);

      xmap_insert (xmap1, 1, "one");
      xmap_insert (xmap1, 2, "two");
      xmap_insert (xmap1, 3, "three");

      // Print out all elements
      size_t *keys = xmap_getkeys (xmap1);
      for (size_t i=0; keys[i]!=0; i++) {
         printf ("Found key %zu\n", keys[i]);
      }
      free (keys);

      xmap_del (xmap1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
size_t *xmap_getkeys (xmap_t *xm)
{
   size_t *ret = malloc (sizeof *ret * (XVECT_LENGTH (xm->array) + 1));
   ret[XVECT_LENGTH(xm->array)] = 0;
   if (!ret) {
      XLOG ("No memory to store list of keys\n");
      return NULL;
   }
   for (size_t i=0; i<XVECT_LENGTH(xm->array); i++) {
      map_t *tmp = XVECT_INDEX (xm->array, i);
      ret[i] = tmp->key;
   }
   return ret;
}


