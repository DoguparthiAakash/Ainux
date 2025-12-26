                                                                               
/* Implement idempotence and versioning at one go */
#ifndef H_XVECTOR
#define H_XVECTOR       "1.0.3"

#include <stdlib.h>

#ifdef H_XVECTOR_THREADED
#include <pthread.h>
#endif

#include <stdbool.h>

/* This is a re-implementation of the rsarray code. As such, much of the old
 * code remains in this version. However, to ensure consistency of use amongst
 * callers, the ability to create an array of mixed pointers/PoDs has been
 * removed from the macros. The macros themselves have been removed too.
 */
struct xvector_t {
#ifdef H_XVECTOR_THREADED
   pthread_mutex_t mutt;
#endif
   size_t alloc_len;
   size_t used_len;
   void **data;
   bool error;
};

typedef struct xvector_t xvector_t;

/**
 * \file xvector.h
 * \def XVECT_LENGTH(array)
 * \brief Returns the length of the vector \a array
 *
 * \a XVECT_LENGTH() is a macro that evaluates to the length of the given
 * vector \a array. 
 *
 * @param[in] array The vector for which the length is calculated
 *
 * @return The macro evaluates to the length of the vector \a array
 */

/**
 * \file xvector.h
 * \def XVECT_INDEX(array,idx)
 * \brief Returns the pointer stored at position \a idx in vector \a array
 *
 * \a XVECT_INDEX() is a macro which evaluates to the pointer stored at 
 * position \a idx in vector \a array.
 *
 * @param[in] array The vector to examine
 * @param[in] idx The index to retrieve
 *
 * @return The macro evaluates to the pointer stored at position \a idx
 * in vector \a array
 */


/**
 * \file xvector.h
 * \def XVECT_DEREF(array,idx,type)
 * \brief Derefences the pointer stored at \a idx in vector \a array
 *
 * \a XVECT_DEREF() is a macro which dereferences the pointer stored at
 * \a idx in vector \a array as a data of type \a type.
 *
 * @param[in] array The vector to examine
 * @param[in] idx The index to retrieve
 * @param[in] type The datatype to cast the pointer to
 *
 * @return The macro evaluates to the pointer stored at position \a idx
 * in vector \a array as a type of \a type.
 */

#define XVECT_LENGTH(array)           (xvector_length (array))
#define XVECT_INDEX(array,idx)        (array->data[idx])
#define XVECT_DEREF(array,idx,type)   *(type *)(XVECT_INDEX(array,idx))

#ifdef __cplusplus
extern "C" {
#endif

   /* Caller can set these to fine-tune the amount of memory
    * that should be allocated on each realloc.
    */
   size_t init_size;
   size_t inc_size;

   /* The functions, these should never be directly called - use
    * the macros above.
    */
   xvector_t *xvector_ins_tail (xvector_t *array, void *element);
   xvector_t *xvector_ins_head (xvector_t *array, void *element);
   xvector_t *xvector_insert (xvector_t *array, size_t position, void *element);
   size_t xvector_length (xvector_t *array);
   void *xvector_del_tail (xvector_t *array);
   void *xvector_del_head (xvector_t *array);
   void *xvector_del (xvector_t *array, size_t position);
   void xvector_free (xvector_t *array);
   void xvector_iterate (xvector_t *array, void (*fptr) (void *));
   xvector_t *xvector_join (xvector_t *a1, xvector_t *a2);

   /* Maps the predicate function on all the elements using 
    * the given void * as the first argument to predicate and
    * the elements of the array as the second argument. Returns
    * the address of the element from the array that matches the
    * element given in the void *.
    *
    * Predicate function follows same rules as for qsort.
    */
   void **xvector_find_first (xvector_t *array, void *needle, 
                             int (*predicate) (void *, void *));
   void **xvector_find_last (xvector_t *array, void *needle,
                             int (*predicate) (void *, void *));

   /* Maps the predicate function across all elements of the array
    * using the given void * as the argument and storing the results
    * in a new xvector_t * which is returned.
    */
   xvector_t *xvector_map (xvector_t *a,
                           void *(*predicate) (void *, void *),
                           void *value);

   /* Make a copy of the tsarray as a normal array of void *, terminated
    * with a null pointer
    */
   void *xvector_native (xvector_t *array);

   void *xvect_dup (void *pod, size_t len);

#ifdef __cplusplus
};
#endif


#endif
