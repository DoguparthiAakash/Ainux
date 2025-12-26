/**
 * \file 
 *
 * \brief Sparse array implementation
 *
 * Sparse arrays are arrays that have many empty slots but still behave like
 * arrays with constant access times. For example, it's possible to have
 * an array of 4.3 million elements, but only access every third or fourth
 * element. With a normal array those unused slots reserve memory that is
 * never used. A sparse array resolves this problem by never allocating
 * space for unused elements.
 *
 * As of writing xsparse uses a dictionary array to store the sparse array.
 * This results in O(n) rather than O(1) access times. To avoid iterating
 * through the entire array including unused slots the caller should use 
 * \a xsparse_iterate() which will ensure that only those slots with values 
 * are iterated over.
 *
 * Items can never be removed from a sparse array, hence if the caller 
 * requires a container which allows deletion then a list would be a better
 * option.
 *
 * \sa xsparse_new(), xsparse_set(), xsparse_del(), xsparse_get(), 
 * xsparse_last(), xsparse_count()
 *
 * xsparse is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 *
 */

#include <stdlib.h>
#include <string.h>

#include "xvector/xvector.h"
#include "xdict/xdict.h"
#include "xsparse/xsparse.h"

struct xsparse_t {
   xdict_t *dict;
   size_t last_index;
};

/**
 * \brief Create a new sparse array
 *
 * Creates a new sparse array suitable for use with \a xsparse_get() and
 * xsparse_set(). See the notes in \a xsparse.c for more information.
 * 
 * \sa xsparse_set(), xsparse_del(), xsparse_get(), 
 * xsparse_last(), xsparse_count(), xsparse_iterate()
 *
 * @return On success a new xsparse array is returned. On failure NULL
 * is returned.
 *
 * \a Example 
 * \verbatim
int main (void)
{
   xsparse_t *bigarray = xsparse_new ();
   // Insert a few items into the sparse array
   for (size_t i=0; i<100; i++) {
      if (i%4) continue;
      static char tmp[255];
      sprintf (tmp, "->'%zu'", i);
      char *ins = xstr_dup (tmp);
      xsparse_set (bigarray, i, ins);
   }
   // Print then delete all the elements that were inserted
   for (size_t i=0; i<100; i++) {
      char *tmp = xsparse_get (bigarray, i);
      printf ("%zu : %s\n", i, tmp);
      free (tmp);
   }

   xsparse_del (bigarray);
   return EXIT_SUCCESS;
}
  \endverbatim
 */
xsparse_t *xsparse_new (void)
{
   xsparse_t *ret = malloc (sizeof *ret);
   if (!ret) return NULL;
   ret->last_index = 0;
   ret->dict = xdict_new (NULL, NULL, NULL);
   if (!ret->dict) {
      free (ret);
      return NULL;
   }
   return ret;
}

/**
 * \brief Delete a sparse array
 *
 * Deletes a sparse array, freeing all resources associated with it. Note that
 * the elements of the sparse array are pointers, and thus must be freed by
 * the caller.
 * 
 * \sa xsparse_set(), xsparse_new(), xsparse_get(), 
 * xsparse_last(), xsparse_count(), xsparse_iterate()
 *
 * @param[in] sa The sparse array to delete
 *
 * @return Nothing.
 *
 * \a Example 
 * \verbatim
int main (void)
{
   xsparse_t *bigarray = xsparse_new ();
   // Insert a few items into the sparse array
   for (size_t i=0; i<100; i++) {
      if (i%4) continue;
      static char tmp[255];
      sprintf (tmp, "->'%zu'", i);
      char *ins = xstr_dup (tmp);
      xsparse_set (bigarray, i, ins);
   }
   // Print then delete all the elements that were inserted
   for (size_t i=0; i<100; i++) {
      char *tmp = xsparse_get (bigarray, i);
      printf ("%zu : %s\n", i, tmp);
      free (tmp);
   }

   xsparse_del (bigarray);
   return EXIT_SUCCESS;
}
  \endverbatim
 */
void xsparse_del (xsparse_t *sa)
{
   if (sa) {
      xdict_del (sa->dict);
      free (sa);
   }
}

/**
 * \brief Store an element into a sparse array
 *
 * Stores the element \a value into the sparse array \a sa at index \a index.
 * The storage replaces any element that was already stored at that location.
 * 
 * \sa xsparse_del(), xsparse_new(), xsparse_get(), 
 * xsparse_last(), xsparse_count(), xsparse_iterate()
 *
 * @param[in] sa The sparse array to modify
 * @param[in] index The index at which the new value \a value will be stored
 * @param[in] value The value to store at \a index
 *
 * @return On success the value that was stored is returned. On failure \a NULL
 * is returned.
 *
 * \a Example 
 * \verbatim
int main (void)
{
   xsparse_t *bigarray = xsparse_new ();
   // Insert a few items into the sparse array
   for (size_t i=0; i<100; i++) {
      if (i%4) continue;
      static char tmp[255];
      sprintf (tmp, "->'%zu'", i);
      char *ins = xstr_dup (tmp);
      xsparse_set (bigarray, i, ins);
   }
   // Print then delete all the elements that were inserted
   for (size_t i=0; i<100; i++) {
      char *tmp = xsparse_get (bigarray, i);
      printf ("%zu : %s\n", i, tmp);
      free (tmp);
   }

   xsparse_del (bigarray);
   return EXIT_SUCCESS;
}
  \endverbatim
 */
void *xsparse_set (xsparse_t *sa, size_t index, void *value)
{
   if (!sa) return NULL;
   sa->last_index = index > sa->last_index ? index : sa->last_index;
   return xdict_set (sa->dict, (void *)index, value);
}

/**
 * \brief Retrieve an element from a sparse array
 *
 * Retrieves the element at position \a index in sparse array \a sa.
 * 
 * \sa xsparse_del(), xsparse_new(), xsparse_set(), 
 * xsparse_last(), xsparse_count(), xsparse_iterate()
 *
 * @param[in] sa The sparse array to examine
 * @param[in] index The position from which to retrieve the value.
 *
 * @return On success the value at position \a index in sparse array \a sa
 * is returned. On failure or if no element was stored at position \a index 
 * then
 * \a NULL is returned.
 *
 * \a Example 
 * \verbatim
int main (void)
{
   xsparse_t *bigarray = xsparse_new ();
   // Insert a few items into the sparse array
   for (size_t i=0; i<100; i++) {
      if (i%4) continue;
      static char tmp[255];
      sprintf (tmp, "->'%zu'", i);
      char *ins = xstr_dup (tmp);
      xsparse_set (bigarray, i, ins);
   }
   // Print then delete all the elements that were inserted
   for (size_t i=0; i<100; i++) {
      char *tmp = xsparse_get (bigarray, i);
      printf ("%zu : %s\n", i, tmp);
      free (tmp);
   }

   xsparse_del (bigarray);
   return EXIT_SUCCESS;
}
  \endverbatim
 */
void *xsparse_get (xsparse_t *sa, size_t index)
{
   if (!sa) return NULL;
   return index > sa->last_index ? NULL : xdict_get (sa->dict, (void *)index);
}

/**
 * \brief Iterate over a sparse array
 *
 * Iterates over the sparse array \a sa, and for each element calls the
 * function \a fptr. The function \a fptr receives two arguments, the first
 * is the index of the element within the sparse array \a sa and the second
 * is the value at that index. The function \a fptr is only called for 
 * used slots within the sparse array.
 *
 * The order of the invocation of each of the values in each position within
 * the array is unspecified.
 * 
 * \sa xsparse_del(), xsparse_new(), xsparse_set(), xsparse_get()
 * xsparse_last(), xsparse_count()
 *
 * @param[in] sa The sparse array to examine
 * @param[in] fptr The function to call for every element in the sparse array
 *
 * @return Nothing
 *
 * \a Example 
 * \verbatim
static void print_elem (size_t index, void *element)
{
   char *el = element;
   printf ("index %zu = \"%s\"\n", index, el);
}

int main (void)
{
   xsparse_t *bigarray = xsparse_new ();
   // Insert a few items into the sparse array
   for (size_t i=0; i<100; i++) {
      if (i%4) continue;
      static char tmp[255];
      sprintf (tmp, "->'%zu'", i);
      char *ins = xstr_dup (tmp);
      xsparse_set (bigarray, i, ins);
   }
   // Iterate only on the used slots of the array
   xsparse_iterate (bigarray, print_elem);
   
   // Delete all the elements that were inserted
   for (size_t i=0; i<100; i++) {
      char *tmp = xsparse_get (bigarray, i);
      free (tmp);
   }

   xsparse_del (bigarray);
   return EXIT_SUCCESS;
}
  \endverbatim
 */
void xsparse_iterate (xsparse_t *sa, void (*fptr) (size_t, void *))
{
   xdict_iterate (sa->dict, (void (*) (void *, void *))fptr);
}

#if 0 // There is not efficient way to do this
static xvector_t *xp_indices;
static bool xp_indices_err;
static void xp_get_indices (void *n, void *v)
{
   v = v;
   xvector_t *tmp = xvector_ins_head (xp_indices, n);
   if (!tmp) {
      xp_indices_err = true;
   } else {
      xp_indices = tmp;
   }
}

size_t *xsparse_indexlist (xsparse_t *sa)
{
   xp_indices_err = false;
   if (xp_indices) xvector_free (xp_indices);
   xp_indices = NULL;
   xp_indices = xvector_ins_head (xp_indices, NULL);
   xdict_iterate (sa->dict, xp_get_indices);
   if (xp_indices_err) return NULL;
   size_t *ret =  xvector_native (xp_indices);
   xvector_free (xp_indices); xp_indices = NULL;
}
#endif

/**
 * \brief Return the index of the last element in the sparse array
 *
 * Returns the index of the last element in the array \a sa. The returned
 * number is gauranteed to be the highest index that is used in the sparse
 * array. This is useful in loops to determine how high to iterate:
 * \verbatim
for (size_t i=0; i<xsparse_last(sa); i++) {
   ...
}
 \endverbatim
 * 
 * Note that a sparse array has neither a practical upper limit nor any 
 * requirement to contain much data, so using a loop to iterate over a sparse
 * array might be computationally wasteful and inefficient. For example,
 * in a sparse array with the last element at position 4000003, there may
 * be only three other elements in the array. Using a loop to process all
 * four elements of the array would result in checking all 4 million unused 
 * positions in the array.
 *
 * A better solution is to use \a xsparse_iterate() with a callback function
 * that will process only those positions in the sparse array that have data.
 *
 * \sa xsparse_del(), xsparse_new(), xsparse_set(), xsparse_get()
 * xsparse_count(), xsparse_iterate()
 *
 * @param[in] sa The sparse array to examine for the last element
 *
 * @return The last position within the sparse array \a sa that has data.
 *
 * \a Example 
 * \verbatim
int main (void)
{
   xsparse_t *bigarray = xsparse_new ();
   // Insert a few items into the sparse array
   for (size_t i=0; i<100; i++) {
      if (i%4) continue;
      static char tmp[255];
      sprintf (tmp, "->'%zu'", i);
      char *ins = xstr_dup (tmp);
      xsparse_set (bigarray, i, ins);
   }
   // Print a count of the number of items in the array
   printf ("Inserted %zu items from 0-100\n", xsparse_count (bigarray));
   // Print the last index of the array
   printf ("Last index of the array = %zu\n", xsparse_last (bigarray));

   // Print then delete elements that were inserted
   for (size_t i=0; i<100; i++) {
      char *tmp = xsparse_get (bigarray, i);
      printf ("%zu : %s\n", i, tmp);
      free (tmp);
   }

   xsparse_del (bigarray);
   return EXIT_SUCCESS;
}
  \endverbatim
 */
size_t xsparse_last (xsparse_t *sa)
{
   return sa->last_index;
}

static size_t xp_count_items;
static void xp_count (void *n, void *v)
{
   n = n; v = v;
   xp_count_items++;
}

/**
 * \brief Count the number of used positions in a sparse array
 *
 * Returns the number of used positions within the sparse array \a sa.
 * Note that the number of used positions within a sparse array may be
 * less than the position of the last element in the array.
 *
 * For example, in a sparse array which has the last element in position
 * 100 and with only four of the positions from 0 to 99 in use, 
 * \a xsparse_count() will return 4 while \a xsparse_last() will return 
 * 100.
 *
 * \sa xsparse_del(), xsparse_new(), xsparse_set(), xsparse_get()
 * xsparse_last(), xsparse_iterate()
 *
 * @param[in] sa The sparse array to examine for the last element
 *
 * @return The last position within the sparse array \a sa that has data.
 *
 * \a Example 
 * \verbatim
int main (void)
{
   xsparse_t *bigarray = xsparse_new ();
   // Insert a few items into the sparse array
   for (size_t i=0; i<100; i++) {
      if (i%4) continue;
      static char tmp[255];
      sprintf (tmp, "->'%zu'", i);
      char *ins = xstr_dup (tmp);
      xsparse_set (bigarray, i, ins);
   }
   // Print a count of the number of items in the array
   printf ("Inserted %zu items from 0-100\n", xsparse_count (bigarray));
   // Print the last index of the array
   printf ("Last index of the array = %zu\n", xsparse_last (bigarray));

   // Print then delete elements that were inserted
   for (size_t i=0; i<100; i++) {
      char *tmp = xsparse_get (bigarray, i);
      printf ("%zu : %s\n", i, tmp);
      free (tmp);
   }

   xsparse_del (bigarray);
   return EXIT_SUCCESS;
}
  \endverbatim
 */
size_t xsparse_count (xsparse_t *sa)
{
   xp_count_items = 0;
   xdict_iterate (sa->dict, xp_count);
   return xp_count_items;
}

