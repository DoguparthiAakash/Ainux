/**
 * \file 
 *
 * \brief Implementation of a vector datatype for C
 *
 * A vector datatype for C that allows growable arrays. (Shrinking not
 * supported yet). The container itself is thread-safe and needs to be
 * linked with pthreads. Note that xvector only stores pointers; no
 * deep copying is done of the actual pointers.
 *
 * Shortcut macros exist to retrieve the length, pointer to element and 
 * element. 
 * 
 * \sa XVECT_DEREF(), XVECT_LENGTH(), XVECT_INDEX() 
 * xvector_ins_head(), xvector_length(), xvector_ins_tail(),
 *    xvector_free(), xvector_insert(),
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * xvector is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 *
 */

#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#include "xvector/xvector.h"

#define DIAGS(...) \
   fprintf (stderr,  __VA_ARGS__)

size_t init_size = 5;
size_t inc_size = 2;

static xvector_t *xp_checkspace (xvector_t *a)
{
	// Check if this is the first time we are running this
	if (!init_size) {
		init_size = 5;
		inc_size = 2;
	}
   /* New xvector_t needed? */
   if (!a) {
      a = malloc (sizeof *a);
      if (!a) {
         DIAGS ("out of memory, returning\n");
         return NULL;
      }
#ifdef H_XVECTOR_THREADED
      pthread_mutex_t tmp = PTHREAD_MUTEX_INITIALIZER;
      a->mutt = tmp;
#endif
      a->used_len = 0;
      a->alloc_len = init_size;
      a->data = malloc (sizeof *a->data * a->alloc_len);
      if (!a->data) {
         free (a);
         DIAGS ("out of memory, returning\n");
         return NULL;
      }
      return a;
   }
   
   /* Did we reached the end of the current allocation? */
#ifdef H_XVECTOR_THREADED
   pthread_mutex_lock (&a->mutt);
#endif
   if (a->used_len==a->alloc_len) { /* If so, extend the array */
      void *tmp = realloc (a->data, sizeof a->data *
                                    (a->alloc_len + inc_size));
      if (!tmp) {
         DIAGS ("Out of memory\n");
         return a;
      }
      a->data = tmp;
      a->alloc_len += inc_size;
   }
#ifdef H_XVECTOR_THREADED
   pthread_mutex_unlock (&a->mutt);
#endif

   /* Struct should now contain enough space */
   return a;
}

/**
 * \brief Append element to the vector
 *
 * Appends the given element to the vector, growing the vector if necessary.
 * \a element must be a pointer width or less in size. 
 *
 * \sa xvector_ins_head(), xvector_free(), xvector_insert(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector to which to add the element. If the vector
 * is \a NULL then a new vector is created
 * @param[in] element The pointer to add to the vector
 *
 * @return On success a \a vector_t \a * is returned and the old value of
 * \a array is not valid. The \a element is guaranteed to be the last element
 * of the returned vector. The vector must be freed with \a xvector_free().
 * On failure \a NULL is returned and \a array is left unchanged.
 */
xvector_t *xvector_ins_tail (xvector_t *array, void *element)
{
   array = xp_checkspace (array);
   if (!array) {
      DIAGS ("Out of memory\n");
      return NULL;
   }
#ifdef H_XVECTOR_THREADED
   pthread_mutex_lock (&array->mutt);
#endif
   array->data[array->used_len++] = element;
#ifdef H_XVECTOR_THREADED
   pthread_mutex_unlock (&array->mutt);
#endif
   return array;
}

/**
 * \brief Insert element at first position in the vector
 *
 * Inserts the given element to the vector at the first position, growing the 
 * vector if necessary. \a element must be a pointer width or less in size. 
 *
 * \sa xvector_ins_tail(), xvector_free(), xvector_insert(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector into which to insert the element. If the vector
 * is \a NULL then a new vector is created
 * @param[in] element The pointer to insert into the vector
 *
 * @return On success a \a vector_t \a * is returned and the old value of
 * \a array is not valid. The \a element is guaranteed to be the first element
 * of the returned vector. The vector must be freed with \a xvector_free().
 * On failure \a NULL is returned and \a array is left unchanged.
 */
xvector_t *xvector_ins_head (xvector_t *array, void *element)
{
   array = xp_checkspace (array);
   if (!array) {
      DIAGS ("Out of memory\n");
      return NULL;
   }
#ifdef H_XVECTOR_THREADED
   pthread_mutex_lock (&array->mutt);
#endif
   memmove (&array->data[1], &array->data[0],
      sizeof *array->data * array->used_len++);
   array->data[0] = element;
#ifdef H_XVECTOR_THREADED
   pthread_mutex_unlock (&array->mutt);
#endif
   return array;
}

/**
 * \brief Insert element at specified position in the vector
 *
 * Inserts the given element \a element into the vector at the position 
 * \a position, growing the vector if necessary. \a element must be a pointer 
 * width or less in size. All array elements from \a position onwards
 * are moved one element forward to make space for the element being inserted.
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector into which to insert the element. If the vector
 * is \a NULL then a new vector is created
 * @param[in] position The position in the array into which the new element 
 *    would be inserted
 * @param[in] element The pointer to insert into the vector
 *
 * @return On success a \a vector_t \a * is returned and the old value of
 * \a array is not valid. 
 * The vector must be freed with \a xvector_free().
 * On failure \a NULL is returned and \a array is left unchanged.
 */
xvector_t *xvector_insert (xvector_t *array, size_t position, void *element)
{
   if (position==0) 
      return xvector_ins_head (array, element);
   if (position>=XVECT_LENGTH (array)) 
      return xvector_ins_tail (array, element);

   array = xp_checkspace (array);
   if (!array) {
      DIAGS ("Out of memory\n");
      return NULL;
   }
#ifdef H_XVECTOR_THREADED
   pthread_mutex_lock (&array->mutt);
#endif
   memmove (&array->data[position+1], &array->data[position],
      (sizeof array->data[0] * (XVECT_LENGTH(array) - position)));
   array->data[position] = element;
   array->used_len++;
#ifdef H_XVECTOR_THREADED
   pthread_mutex_unlock (&array->mutt);
#endif
   return array;
}

/**
 * \brief Return the length of the vector
 *
 * Returns the length of the vector in number of elements.  Reading 
 * past the length results in undefined behaviour. A shortcut macro exists
 * that performs the same function as \a xvector_length().
 * 
 *
 * \sa XVECT_LENGTH(), xvector_ins_head(), xvector_ins_tail(), xvector_free(),
 *    xvector_iterate(), xvector_del_head(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector to be counted.
 * If the vector is \a NULL then a \a NULL is returned and the vector
 * remains unchanged.
 *
 * @return On success the length of the array is returned. On failure
 * (size_t)-1 is returned.
 */
size_t xvector_length (xvector_t *array)
{
   if (!array) return 0;
   return array->used_len;
}

/**
 * \brief Remove the last element of the vector
 *
 * Removes the last element of the vector, shrinking the vector if necessary.
 * The position of the other elements of the vector remain unchanged.
 * 
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector from which to remove the last element.
 * If the vector is \a NULL then a \a NULL is returned and the vector
 * remains unchanged.
 *
 * @return On success a pointer to the removed element is returned and
 * the vector will have one less element. 
 * On failure \a NULL is returned.
 */
void *xvector_del_tail (xvector_t *array)
{
   if (!array) return NULL;
#ifdef H_XVECTOR_THREADED
   pthread_mutex_lock (&array->mutt);
#endif
   void *ret = array->data[array->used_len-1];
   array->used_len--;
#ifdef H_XVECTOR_THREADED
   pthread_mutex_unlock (&array->mutt);
#endif
   return ret;
}

/**
 * \brief Remove the first element of the vector
 *
 * Removes the first element of the vector, shrinking the vector if necessary.
 * All the elements of the vector are shifted one position to the left;
 * the second element becomes the new first element, the third element 
 * becomes the new second element, etc.
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_iterate(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector from which to remove the first element.
 * If the vector is \a NULL then a \a NULL is returned and the vector
 * remains unchanged.
 *
 * @return On success a pointer to the removed element is returned and
 * the vector will have one less element, with all the remaining elements
 * being shifted one position to the left. 
 * On failure \a NULL is returned.
 */
void *xvector_del_head (xvector_t *array)
{
   if (!array) return NULL;
   void *ret = array->data[0];
   /* If init_size is exactly one && length of a is one, this would
    * result in access violations. Since its a read violation, I don't
    * really care about it for now. Can be fixed.
    */
#ifdef H_XVECTOR_THREADED
   pthread_mutex_lock (&array->mutt);
#endif
   memmove (&array->data[0], &array->data[1],
      (sizeof *array->data * (XVECT_LENGTH (array) - 1)));
   array->used_len--;
   void *retu = array->used_len>0 ? ret : NULL;
#ifdef H_XVECTOR_THREADED
   pthread_mutex_unlock (&array->mutt);
#endif
   return retu;
}

/**
 * \brief Remove the specified element from the vector
 *
 * Removes the element at position \a position from the vector \a array.
 * All other elements succeeding position \a position are moved one
 * position down so that the element as \a (position+1) before the call
 * to this function is now at position \a position.
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), 
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector from which to remove the first element.
 * If the vector is \a NULL then a \a NULL is returned and the vector
 * remains unchanged.
 * @param[in] position The position of the element to rmove from array
 *    \a array.
 *
 * @return On success a pointer to the removed element is returned and
 * the vector will have one less element.
 * On failure \a NULL is returned.
 */
void *xvector_del (xvector_t *array, size_t position)
{
   if (!array) return NULL;
   if (position==0)
      return xvector_del_head (array);
   if (position>=XVECT_LENGTH (array))
      return xvector_del_tail (array);

   void *ret = XVECT_INDEX (array, position);
#ifdef H_XVECTOR_THREADED
   pthread_mutex_lock (&array->mutt);
#endif
   memmove (&array->data[position], &array->data[position + 1],
      (sizeof *array->data * (XVECT_LENGTH (array) - position -1)));
   array->used_len--;
   void *retu = array->used_len>0 ? ret : NULL;
#ifdef H_XVECTOR_THREADED
   pthread_mutex_unlock (&array->mutt);
#endif
   return retu;
}

/**
 * \brief Free the vector
 *
 * Free the vector. None of the elements themselves are freed, only the
 * container holding them; the caller is responsible for ensuring that
 * they still have a reference to use to free the elements of the vector.
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector to free.
 *
 * @return Nothing
 */
void xvector_free (xvector_t *array)
{
   if (array) {
#ifdef H_XVECTOR_THREADED
      pthread_mutex_lock (&array->mutt);
#endif
      if (array->data && array->alloc_len) {
         free (array->data);
      }
#ifdef H_XVECTOR_THREADED
      pthread_mutex_unlock (&array->mutt);
      pthread_mutex_destroy (&array->mutt);
#endif
      free (array);
   }
}

/**
 * \brief Iterate over each element of the vector
 *
 * Iterate over each element of the vector in the order that they are
 * stored (i.e. left to right) and on each iteration apply the function
 * \a f() using the element of the vector as an argument.
 *
 * Logically, the following pseudocode applies:
 * \verbatim

      foreach element from vector do
         call function fptr(element)
      done

   \endverbatim
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector to iterate over. If the vector
 * is \a NULL then no action is taken and \a xvector_iterate returns 
 * immediately
 * @param[in] fptr The predicate function to apply on every single element of
 * \a array
 *
 * @return Nothing
 */
void xvector_iterate (xvector_t *array, void (*fptr) (void *))
{
   if (!array) return;
   size_t i;
   for (i=0; i<array->used_len; i++) {
      fptr (array->data[i]);
   }
}


/**
 * \brief Join two vectors into a single large one.
 *
 * Join array \a a1 and array \a a2 into a single large array which is
 * returned as an xvector_t *. The arguments are not modified and the 
 * contents of the arrays \a a1 and \a a2 are bit-by-bit copied.
 * The \a xvector_t \a * that is returned must be free()ed by the caller.
 *
 * 
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native(), xvector_iterate()
 *
 * @param[in] a1 The first array 
 * @param[in] a2 The second array 
 *
 * @return A pointer to a vector_t that contains all elements of \a a1 and 
 * \a a2, with all the elements of \a a1 before any of the elements of \a a2.
 * The relative position of each element within each array is unchanged.
 */
xvector_t *xvector_join (xvector_t *a1, xvector_t *a2)
{
   xvector_t *ret = NULL;
   for (size_t i=0; i<XVECT_LENGTH (a1); i++) {
      xvector_t *tmp = xvector_ins_tail (ret, XVECT_INDEX (a1, i));
      if (!tmp) goto errorexit;
      ret = tmp;
   }
   for (size_t i=0; i<XVECT_LENGTH (a2); i++) {
      xvector_t *tmp = xvector_ins_tail (ret, XVECT_INDEX (a2, i));
      if (!tmp) goto errorexit;
      ret = tmp;
   }
   return ret;
errorexit:
   xvector_free (ret);
   return NULL;
}
/**
 * \brief Find the first occurrence of \a needle in array
 *
 * Uses the predicate function \a predicate() to compare each element to
 * \a needle, starting at element 0. The first match causes the search to end.
 * The function pointer \a predicate has to take two arguments, \a A and \a B,
 * both void pointers, and must return 0 if they match, 1 if A > B and -1 if
 * A < B.
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector in which to iterate the search over. If the
 * vector
 * is \a NULL then no action is taken and \a xvector_find_first returns NULL.
 * @param[in] needle The pointer to the search object (see above)
 * @param[in] predicate The comparison function
 *
 * @return On success the first match in all the elements (as determined by
 * the \a predicate function) is returned. On failure \a NULL is returned.
 */
void **xvector_find_first (xvector_t *array, void *needle, 
                          int (*predicate) (void *, void *))
{
   for (size_t i=0; i<XVECT_LENGTH(array); i++) {
      void *second = XVECT_INDEX(array, i);
      if (predicate (needle, second)==0) return &array->data[i];
   }
   return NULL;
}

/**
 * \brief Find the last occurrence of \a needle in array
 *
 * Uses the predicate function \a predicate() to compare each element to
 * \a needle, starting at the last element and working backward. The first
 * match when searching from the end causes the search to end.
 * The function pointer \a predicate has to take two arguments, \a A and \a B,
 * both void pointers, and must return 0 if they match, 1 if A > B and -1 if
 * A < B.
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector in which to iterate the search over. If the
 * vector
 * is \a NULL then no action is taken and \a xvector_find_last returns NULL.
 * @param[in] needle The pointer to the search object (see above)
 * @param[in] predicate The comparison function
 *
 * @return On success the last match in all the elements (as determined by
 * the \a predicate function) is returned. On failure \a NULL is returned.
 */
void **xvector_find_last (xvector_t *array, void *needle,
                         int (*predicate) (void *, void *))
{
   for (int i=XVECT_LENGTH(array); i>0; i--) {
      void *second = XVECT_INDEX(array, i-1);
      if (predicate (needle, second)==0) return &array->data[i-1];
   }
   return NULL;
}

/**
 * \brief Maps the given function across all elements of the vector
 *
 * Iterates over the given vector \a array in order of position from
 * lowest to highest, and for each element invokes the \a predicate function
 * using the element as the first argument and the \a value as the 
 * second argument to the \a predicate function. The return value of
 * all the invocations are stored in a vector which is returned to the
 * caller.
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_find_first(), xvector_find_last(), xvector_dup(),
 *    xvector_native()
 *
 * @param[in] array The vector over which the function will map. If the
 * vector
 * is \a NULL then no action is taken and \a xvector_map returns NULL.
 * @param[in] array The vector to map the function \a func across
 * @param[in] predicate The function that gets mapped across all elements
 * @param[in] value The value that gets used as the second argument to the
 * predicate function (each element of the array is used as the first
 * argument)
 *
 * @return On success a vector containing all the return values from the
 * mapping is returned to the caller. The caller is responsible for 
 * freeing the returned vector. On failure, the error component of the
 * vector is set to true and the possibly incomplete vector is returned to
 * the caller.
 */
xvector_t *xvector_map (xvector_t *array,
                        void *(*predicate) (void *, void *),
                        void *value)
{
   if (!array) return NULL;
   xvector_t *ret = NULL;
   for (size_t i=0; i<XVECT_LENGTH(array); i++) {
      xvector_t *tmp =
         xvector_ins_tail (ret, predicate (XVECT_INDEX (array, i), value));
      if (!tmp) {
         ret->error = true;
         return ret;
      }
      ret = tmp;
   }
   return ret;
}

/**
 * \brief Return a normal array of elements from the vector
 *
 * The elements of the vector (all pointers) are copied to an array
 * of void pointers and returned to the caller. The returned array
 * is NULL-terminated, in that the final element (which is not part of the
 * vector) is a NULL pointer. The caller must free the returned array.
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(), xvector_dup(),
 *
 * @param[in] array The vector to convert to plain array form. If the 
 * vector \a array is NULL, then a plain array of exactly one element is 
 * returned. The single element in the returned array of void pointers is NULL.
 *
 * @return On success an array of void pointers is returned, with the array
 * being terminated by a NULL pointer. The caller must free this array.
 * Note that it is the array only that must be freed and not each element
 * in the array (which the vector will still reference). Note that if the
 * \a array passed into \a xvector_native() is NULL and/or empty, then 
 * the return value is still an array of void pointers, albeit with only
 * a single element, namely the terminating NULL pointer. The returned
 * value must still be free()ed.
 */
void *xvector_native (xvector_t *array)
{
   if (!array) {
      void **ret = malloc (sizeof *ret);
      if (!ret) return NULL;
      *ret = NULL;
      return ret;
   }
   void **ret = malloc (sizeof *ret * (XVECT_LENGTH (array) + 1));
   if (!ret) return NULL;
   size_t i;
   for (i=0; i<XVECT_LENGTH (array); i++) {
      ret[i] = XVECT_INDEX (array, i);
   }
   ret[i] = NULL;
   return ret;
}

/**
 * \brief Return a copy of the given Plain Old Data element
 *
 * Makes and returns a copy of the given \a pod (Plain Old Data) which is
 * no more than \a len length in bytes. This is simply an alternative to
 * \a memcpy(), and it is suggested that the caller use that function
 * instead.
 *
 * \sa xvector_ins_head(), xvector_ins_tail(), xvector_free(), xvector_length()
 *    xvector_iterate(), xvector_del_head(), xvector_del_tail(), xvector_del()
 *    xvector_map(), xvector_find_first(), xvector_find_last(),
 *    xvector_native()
 *
 * @param[in] pod The address of the existing data object
 * @param[in] len The length of the above data object
 *
 * @return On success an exact bitwise copy of the contents at \a pod
 * of length \a len is returned. On failure \a NULL is returned.
 */
void *xvect_dup (void *pod, size_t len)
{
   if (!pod) return NULL;
   void *ret = malloc (len);
   if (!ret) {
      DIAGS ("duplication failed, returning\n");
      return NULL;
   }
   memcpy (ret, pod, len);
   return ret;
}

