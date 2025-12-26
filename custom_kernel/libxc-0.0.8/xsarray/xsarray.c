

#include "xvector/xvector.h"
#include "xsarray/xsarray.h"

/**
 * \file
 * 
 * \brief Array that remains sorted after every insertion and deletion
 *
 * Xsarray is a permanently sorted array. Every insertion is performed
 * such that the array remains sorted after the insertion. Every deletion
 * leaves the array sorted. Searches are performed using a binary search.
 * Items can be searched for by object reference (pointer to specific
 * object) or by value using the function supplied in xsarray_new().
 * 
 * \sa xsarray_new(), xsarray_free(), xsarray_length(), xsarray_index(),
 *    xsarray_insert(), xsarray_remove(), xsarray_find(), xsarray_purge(),
 *    xsarray_array()
 *
 * xsarray is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 */

struct xsarray_t {
   xvector_t *array;
   int (*comparison) (void *, void *);
};

/**
 * \brief Create a new sorted array (xsarray_t) for use by caller
 *
 * Creates and returns a new sorted array (xsarray_t). The parameter
 * \a comparison is a function that will be used by the other xsarray() 
 * functions to rank the ordinality of each item that gets inserted. The
 * comparison function is needed to ensure that the array remains
 * sorted after every insertion.
 *
 * The comparison function \a comparison takes two arguments, \a A and \a B,
 * and returns -1 if \a A < \a B, 0 if \a A = \a B and 1 if \a A > \a B. For
 * example, when storing strings in an xsarray, the caller can simply
 * use the standard function \a strcmp() as the argument to xsarray_new().
 *
 * The caller must free the return value with \a xsarray_free() when 
 * the return value is no longer required.
 * 
 * \sa xsarray_free(), xsarray_length(), xsarray_index(),
 *    xsarray_insert(), xsarray_remove(), xsarray_find(), xsarray_purge(),
 *    xsarray_array()
 *
 * @param[in] comparison The comparison function. See above for a thorough
 *    explanation of this function.
 *
 * @return On success a new xsarray_t is returned, which the caller must
 *    free at some point with \a xsarray_free().  On failure \a NULL is
 *    returned.
 */
xsarray_t *xsarray_new (int (*comparison) (void *, void *))
{
   xsarray_t *ret = malloc (sizeof *ret);
   if (!ret) return NULL;
   ret->comparison = comparison;
   ret->array = NULL;
   return ret;
}

/**
 * \brief Frees an xsarray_t object
 *
 * Frees an xsarray_t object which must
 * have been returned from a call to xsarray_new(). Only the array \a xsa
 * is freed and not the individual elements of the array. The caller is
 * still responsible for freeing each element of the array before calling
 * this function.
 *
 * \sa xsarray_new(), xsarray_length(), xsarray_index(),
 *    xsarray_insert(), xsarray_remove(), xsarray_find(), xsarray_purge(),
 *    xsarray_array()
 *
 * @param[in] xsa The array to free. This array must have been returned from
 * a successful call to xsarray_new()
 *
 * @return Nothing. A side-effect of freeing up an \a xarray_t object occurrs.
 */
void xsarray_free (xsarray_t *xsa)
{
   if (xsa) {
      if (xsa->array) {
         xvector_free (xsa->array);
      }
      free (xsa);
   }
}

/**
 * \brief Returns the length of the xsarray_t array object
 *
 * Returns the number of elements stored in the sorted array object \a xsa.
 *
 * \sa xsarray_new(), xsarray_free(), xsarray_index(),
 *    xsarray_insert(), xsarray_remove(), xsarray_find(), xsarray_purge(),
 *    xsarray_array()
 *
 * @param[in] xsa The array to determine the length of.
 *
 * @return The length of the sorted array \a xsa.
 */
size_t xsarray_length (xsarray_t *xsa)
{
   if (!xsa) return 0;
   return XVECT_LENGTH (xsa->array);
}

/**
 * \brief Returns the element at position \a index in array \a xsa
 *
 * Returns the element at position \a index in array \a xsa. NULL is
 * returned if \a index is outside the bounds of the array.
 *
 * \sa xsarray_new(), xsarray_free(), xsarray_length(),
 *    xsarray_insert(), xsarray_remove(), xsarray_find(), xsarray_purge(),
 *    xsarray_array()
 *
 *
 * @param[in] xsa The array to return element \a index from
 * @param[in] index The index of the element within \a xsa that must be 
 *    returned
 *
 * @return The element at position \a index in sorted array \a xsa.
 */
void *xsarray_index (xsarray_t *xsa, size_t index)
{
   if (!xsa) return NULL;
   return XVECT_INDEX (xsa->array, index);
}

/**
 * \brief Inserts an item into the sorted array
 *
 * Inserts the given element \a element into the sorted array \a xsa. The
 * insertion is performed such that the sorted array \a xsa remains in a
 * sorted position after this function returns. The function
 * \a comparison that was specified as the argument to \a xsarray_new() 
 * in the creation of \a xsa is used to determine the new element's position
 * within the array \a xsa.
 *
 * \sa xsarray_new(), xsarray_free(), xsarray_length(), xsarray_index(),
 *    xsarray_remove(), xsarray_find(), xsarray_purge(),
 *    xsarray_array()
 *
 * @param[in] xsa The array to insert the new element \a element into
 * @param[in] element The element to insert into sorted array \a xsa 
 *
 * @return On success the element that was inserted is returned. On failure
 * \a NULL is returned.
 */
void *xsarray_insert (xsarray_t *xsa, void *element)
{
   // TODO: Change this to use a binary search
   // find the position to insert into
   if (!xsa || !element) return NULL;
   size_t pos;
   for (pos=0; pos<XVECT_LENGTH(xsa->array); pos++) {
      if (xsa->comparison (XVECT_INDEX (xsa->array, pos), element)>=0) break;
   }
   // Do the insertion?
   xvector_t *tmp = xvector_insert (xsa->array, pos, element);
   if (!tmp) {
      return NULL;
   }
   xsa->array = tmp;
   return element;
}

/**
 * \brief Searches a sorted array for an element
 *
 * Searches the sorted array \a xsa for element \a needle, using the
 * function \a comparison to determine equality and returns the
 * first occurrence of an element that matches \a needle. Note that
 * the function \a comparison was specified as an argument to 
 * \a xsarray_new() during the creation of the sorted array \a xsa.
 *
 * As only the first match is returned and a sorted array object (xsarray_t)
 * may contain items that are identical, and even contain duplicates, it's
 * up to the caller to ensure that the item returned is the correct object.
 * For more refined access to the sorted array the caller may use
 * xsarray_array() to gain access to the actual array.
 * 
 * \sa xsarray_new(), xsarray_free(), xsarray_length(), xsarray_index(),
 *    xsarray_insert(), xsarray_remove(), xsarray_purge(),
 *    xsarray_array()
 *
 * @param[in] xsa The array to search
 * @param[in] needle The element to search for. The comparison function
 *    is used to determine equality
 *
 * @return On success the first element that matches \a needle is returned.
 * On failure or if no match could be made \a NULL is returned.
 */
void *xsarray_find (xsarray_t *xsa, void *needle)
{
   // TODO: Implement a binary search
   if (!xsa || !needle) return NULL;
   size_t pos = (size_t)-1;
   for (size_t i=0; i<XVECT_LENGTH(xsa->array); i++) {
      if (xsa->comparison (XVECT_INDEX (xsa->array, i), needle)>=0) {
         pos = i;
         break;
      }
   }
   if (pos==(size_t)-1) return NULL; // Element not found
   return XVECT_INDEX (xsa->array, pos);
}

/**
 * \brief Removes the given object from the sorted array
 *
 * Searches the sorted array \a xsa for element \a item using pointer equality
 * to ensure that the actual object \a item itself is removed. Elements which
 * match \a item in a comparison performed with the function \a comparison are
 * not matched. For a more detailed explanation of the function \a comparison 
 * see \a xsarray_new().
 *
 * Note that only the first object found that matches \a item is removed. To
 * remove all the elements in the sorted array that match \a item use the
 * the function \a xsarray_purge(). For more fine-grained access to the 
 * sorted array use \a xsarray_array(), which returns the actual array.
 *
 * \sa xsarray_new(), xsarray_free(), xsarray_length(), xsarray_index(),
 *    xsarray_insert(), xsarray_find(), xsarray_purge(),
 *    xsarray_array()
 *
 * @param[in] xsa The array from which to remove \a item
 * @param[in] item The item to remove from \a xsa. Note that a pointer
 * comparison is made to determine equality of \a item with the elements of
 * of the sorted array \a xsa
 *
 * @return On success the removed object is returned.
 * On failure or if no match could be made \a NULL is returned.
 */
void *xsarray_remove (xsarray_t *xsa, void *item)
{
   // TODO: Implement a binary search
   if (!xsa || !item) return NULL;
   size_t pos = (size_t)-1;
   for (size_t i=0; i<XVECT_LENGTH(xsa->array); i++) {
      if (item == XVECT_INDEX (xsa->array, i)) {
         pos = i;
         break;
      }
   }
   if (pos==(size_t)-1) return NULL; // Element not found
   return xvector_del (xsa->array, pos);
}

/**
 * \brief Removes all elements in sorted array which match \a item
 *
 * Searches the sorted array \a xsa for all elements that match \a item
 * using pointer equality. All the elements that match are removed from the
 * sorted array \a xsa. To remove only a single element from the sorted 
 * array \a xsa see \a xsarray_remove().
 *
 * For more fine-grained access to the 
 * sorted array use \a xsarray_array(), which returns the actual array.
 *
 * \sa xsarray_new(), xsarray_free(), xsarray_length(), xsarray_index(),
 *    xsarray_insert(), xsarray_remove(), xsarray_find()
 *
 * @param[in] xsa The array from which to remove \a item
 * @param[in] item The item to remove from \a xsa. Note that a pointer
 * comparison is made to determine equality of \a item with the elements of
 * of the sorted array \a xsa. All elements which match \a item are removed
 *
 * @return On success the removed object is returned, no matter how many
 * occurrences of \a item were found in sorted array \a xsa.
 * On failure or if no match could be made \a NULL is returned.
 */
void *xsarray_purge (xsarray_t *xsa, void *item)
{
   // TODO: Implement a binary search
   if (!xsa || !item) return NULL;
   void *ret = NULL;
   for (size_t i=0; i<XVECT_LENGTH(xsa->array); i++) {
      void *tmp = XVECT_INDEX (xsa->array, i);
      ret = tmp;
      while (i < XVECT_LENGTH (xsa->array) && tmp==item) {
         tmp = xvector_del (xsa->array, i);
         if (!tmp) {
            break;
         }
         tmp = XVECT_INDEX (xsa->array, i);
      }
   }
   return ret;
}

/**
 * \brief Returns a sorted vector from the sorted array
 *
 * Returns the vector object \a xvector_t that is maintained by the
 * sorted array object \a xsa. Note that any changes to the returned
 * vector object is visible to the sorted array \a xsa, so the caller should
 * note the following when making changes to the returned \a xvector_t object:
 *   - Elements can be removed with \a xvector_del_head(),
 *    \a xvector_del_tail() and \a xvector_del().
 *   - Elements can be dereferenced with \a XVECT_INDEX()
 *   - Elements that are added with \a xvector_ins_tail(), 
 *       \a xvector_ins_head() and xvector_insert() will cause the array to
 *       become unsorted.
 *
 * In general, the caller may remove and dereference items from the
 * returned \a xvector_t array object but the caller may not add new items
 * to the returned \a xvector_t object.
 *
 * \sa xsarray_new(), xsarray_free(), xsarray_length(), xsarray_index(),
 *    xsarray_insert(), xsarray_remove(), xsarray_find(), xsarray_purge()
 *
 * @param[in] xsa The sorted array to use for the contents of the returned
 * vector
 *
 * @return On success a vector object \a xvector_t is returned, which the
 * caller should not attempt to free. The \a xvector_t object gets freed
 * when xsarray_free() is called.
 * On failure \a NULL is returned.
 */
xvector_t *xsarray_array (xsarray_t *xsa)
{
   if (xsa) return xsa->array;
   else return NULL;
}
