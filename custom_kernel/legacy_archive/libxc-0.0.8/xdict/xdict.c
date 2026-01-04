/**
 * \file 
 *
 * \brief Implementation of dictionary (associative arrays)
 *
 * Implementation of a dictionary, also known as an associative array, 
 * container for C. The caller has to set the hash copy, delete and 
 * compare functions when creating the dictionary. If the width of the hash
 * used is less than a pointer width, then the caller can simply set those
 * values to \a NULL, in which case the copy function is bitwise assignment,
 * the delete function is ignored and the compare function is bitwise
 * comparison. 
 *
 * See the examples for each of the functions in this module for more
 * detailed explanation.
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


struct xdict_t {
   xvector_t *array;
   int (*cmp) (void *, void *);
   void *(*cpy) (void *);
   void (*del) (void *);
};

/* Maintain name/value pairs */
typedef struct xp_pair_t xp_pair_t;
struct xp_pair_t {
   void *name;
   void *value;
};

static xp_pair_t *xp_new_pair (void *name, void *value, void *(*cpy) (void *))
{
   xp_pair_t *ret = malloc (sizeof *ret);
   if (!ret) return NULL;
   ret->name = cpy ? cpy (name) : name;
   ret->value = value;
   return ret;
}

static void xp_del_pair (xp_pair_t *pair, void (*del) (void *))
{
   if (pair) {
      if (pair->name) {
         if (del) del (pair->name);
      }
      free (pair);
   }
}

static int xp_cmp (void *a1, void *a2)
{
   if (a1<a2) return -1;
   if (a1>a2) return 1;
   return 0;
}

static void *xp_cpy (void *e)
{
   return e;
}

static void xp_del (void *e)
{
   e = e;
}
/**
 * \brief Create a new dictionary
 *
 * Creates a new dictionary. An array of name/value pairs are stored as
 * the dictionary. The three functions \a cmp, \a cpy and \a del are
 * the functions that the dictionary will use to compare, copy and delete
 * the names in the array. The values to be stored are caller-allocated
 * pointers.
 *
 * For names that are of a type that is less than or equal to a pointer width, 
 * the caller can simply cast each name on insert to \a void \a * and
 * use \a NULL for all the \a xdict_new() arguments. If this is done then
 * the caller must cast back when retrieving the name (see example).
 *
 * For values that are of a type that is less than or equal to a pointer
 * width, the caller can simply cast each value to a \a void \a * on
 * insert. If this is done then the caller must cast back to the original
 * type when retrieving the value (see example).
 *
 * @param[in] cmp The comparison function that the dictionary will use
 * to compare the hash/names of any entry in the dictionary.
 * @param[in] cpy The copying function that the dictionary will use
 * when copying the hash/names of any entry in the dictionary.
 * @param[in] del The deletion function that the dictionary will use
 * when removing the hash/names of any entry in the dictionary.
 *
 * @return On success an \a xdict_t \a * is returned for subsequent use
 * with \a xdict_set() and \a xdict_get. The caller must free this value
 * at a later time using \a xdict_del().
 * On failure \a NULL is returned.
 *
 * \a Example 
 * \verbatim
   // The following three static functions are for example two below

   static int dict_string_cmp (void *a1, void *a2)
   {
      return strcmp ((char *)a1, (char *)a2);
   }

   static void *dict_string_cpy (void *s)
   {
      return xstr_dup (s); // duplicate the string
   }

   static void dict_string_del (void *s)
   {
      if (s) free (s);
   }

   int main (void)
   {
      // First example: using hash entries with each name less 
      // than a pointer width
      // All NULL's indicating that datatype for hash is less
      // than a pointer width
      xdict_t *hashmap = xdict_new (NULL, NULL, NULL);
   
      // Our hashmap will use ints for name and chars for values
      int v1[] = {10, 20, 30, 40, 50, 60, 70, 80, 90, 100};
      char v2[] = "1234567890";

      // Set all the dictionary entries
      for (size_t i=0; i < sizeof v1/sizeof v1[0]; i++) {
         xdict_set (hashmap, (void *)v1[i], (void *)v2[i]);
      }
      // Check that they are all set properly
      for (size_t i=sizeof v1/sizeof v1[0]; i>0; i--) {
         int index = v1[i-1];
         char value = (char)xdict_get (hashmap, (void *)index);
         printf ("For name=%i, got value '%c'\n", index, value);
      }
      // Free the map and all entries it holds
      xdict_del (hashmap);

      // Second example: Using custom comparison, copy and deletion 
      // functions that uses strings as the hash/name and ints as the
      // value
      hashmap = xdict_new (dict_string_cmp,
                           dict_string_cpy, 
                           dict_string_del);
      char *v4[] = {"one", "two", "three", "four", "five",
                     "six", "seven"};
      // Add in dictionary entries using strings as the hashes/names and
      // ints as the value
      for (size_t i=0; i<sizeof v4/sizeof v4[0]; i++) {
         xdict_set (hashmap, v4[i], (void *)v1[i]);
      }
      // Print it all out to make sure that they got set
      for (size_t i=sizeof v4/sizeof v4[0]; i>0; i--) {
         char *index = v4[i-1];
         int value = (char)xdict_get (hashmap, index);
         printf ("For name=%s, got value '%i'\n", index, value);
      }
      // Delete the hash and all it's entries
      xdict_del (hashmap);

      return EXIT_SUCCESS;
   }
  \endverbatim
 */

xdict_t *xdict_new (int (*cmp) (void *, void *), void *(*cpy) (void *),
                    void (*del) (void *))
{
   xdict_t *ret = malloc (sizeof *ret);
   if (!ret) return NULL;
   ret->array=NULL;
   ret->cmp = cmp ? cmp : xp_cmp;
   ret->cpy = cpy ? cpy : xp_cpy;
   ret->del = del ? del : xp_del;
   return ret;
}

/**
 * \brief Deletes a dictionary and all it's entries
 *
 * Deletes the specified dictionary/associative array, and frees all the
 * resources that it was using. Note that the caller has to free all the
 * values stored before this function is run to avoid leaking memory.
 *
 * See the function \a xdict_new() for more information on how this function
 * deletes a dictionary and examples/
 *
 * @param[in] dict The dictionary to delete
 *
 * @return Nothing.
 */
void xdict_del (xdict_t *dict)
{
   if (dict) {
      for (size_t i=0; i<XVECT_LENGTH (dict->array); i++) {
         xp_pair_t *pair = XVECT_INDEX (dict->array, i);
         xp_del_pair (pair, dict->del);
      }
      xvector_free (dict->array);
      free (dict);
   }
}

/**
 * \brief Set the value for a given name/hash in the dictionary
 *
 * Sets the value of \a index to \a element in the dictionary \a dict. If the
 * element at index \a index already exists, it will be reset to the 
 * \a element given by the caller. A new copy of \a index is stored in the
 * dictionary.
 *
 * For names/indices that are of a type that is less than or equal to a
 * pointer width, the caller can simply cast each name on insert to \a void
 * \a * and
 * use \a NULL for all the \a xdict_new() arguments. If this is done then
 * the caller must cast back when retrieving the name (see example).
 *
 * For values that are of a type that is less than or equal to a pointer
 * width, the caller can simply cast each value to a \a void \a * on
 * insert. If this is done then the caller must cast back to the original
 * type when retrieving the value (see example).
 *
 * @param[in] dict The dictionary to use
 * @param[in] index The index of the entry in the dictionary
 * @param[in] element A pointer to the caller-allocated element to 
 * store for entry \a index in the dictionary \a dict
 *
 * @return On success a pointer to the existing element is returned.
 * On failure \a NULL is returned.
 *
 * \a Example 
 * \verbatim
   // The following three static functions are for example two below

   static int dict_string_cmp (void *a1, void *a2)
   {
      return strcmp ((char *)a1, (char *)a2);
   }

   static void *dict_string_cpy (void *s)
   {
      return xstr_dup (s); // duplicate the string
   }

   static void dict_string_del (void *s)
   {
      if (s) free (s);
   }

   int main (void)
   {
      // First example: using hash entries with each name less 
      // than a pointer width
      // All NULL's indicating that datatype for hash is less
      // than a pointer width
      xdict_t *hashmap = xdict_new (NULL, NULL, NULL);
   
      // Our hashmap will use ints for name and chars for values
      int v1[] = {10, 20, 30, 40, 50, 60, 70, 80, 90, 100};
      char v2[] = "1234567890";

      // Set all the dictionary entries
      for (size_t i=0; i < sizeof v1/sizeof v1[0]; i++) {
         xdict_set (hashmap, (void *)v1[i], (void *)v2[i]);
      }
      // Check that they are all set properly
      for (size_t i=sizeof v1/sizeof v1[0]; i>0; i--) {
         int index = v1[i-1];
         char value = (char)xdict_get (hashmap, (void *)index);
         printf ("For name=%i, got value '%c'\n", index, value);
      }
      // Free the map and all entries it holds
      xdict_del (hashmap);

      // Second example: Using custom comparison, copy and deletion 
      // functions that uses strings as the hash/name and ints as the
      // value
      hashmap = xdict_new (dict_string_cmp,
                           dict_string_cpy, 
                           dict_string_del);
      char *v4[] = {"one", "two", "three", "four", "five",
                     "six", "seven"};
      // Add in dictionary entries using strings as the hashes/names and
      // ints as the value
      for (size_t i=0; i<sizeof v4/sizeof v4[0]; i++) {
         xdict_set (hashmap, v4[i], (void *)v1[i]);
      }
      // Print it all out to make sure that they got set
      for (size_t i=sizeof v4/sizeof v4[0]; i>0; i--) {
         char *index = v4[i-1];
         int value = (char)xdict_get (hashmap, index);
         printf ("For name=%s, got value '%i'\n", index, value);
      }
      // Delete the hash and all it's entries
      xdict_del (hashmap);

      return EXIT_SUCCESS;
   }
  \endverbatim
 */
void *xdict_set (xdict_t *dict, void *index, void *element)
{
   // Create the pair that will go into the array
   xp_pair_t *newpair = xp_new_pair (index, element, dict->cpy);
   if (!newpair) return NULL;

   // Determine if index already exists in array
   void *oldpair = NULL;
   size_t replace_pos = (size_t)-1;
   for (size_t i=0; i<xvector_length (dict->array); i++) {
      xp_pair_t *pair = XVECT_INDEX (dict->array, i);
      if (dict->cmp (index, pair->name)==0) {
         oldpair = pair;
         replace_pos = i;
         break;
      }
   }
   // Insert at the end of array if not found
   if (!oldpair) {
      xvector_t *tmp = xvector_ins_tail (dict->array, newpair);
      if (!tmp) goto error;
      dict->array = tmp;
      return element;
   }
   // Else, replace the existing pair with newpair
   dict->array->data[replace_pos] = newpair;
   // Free the oldpair
   xp_del_pair (oldpair, dict->del);
   return element;
error:
   if (newpair) xp_del_pair (newpair, dict->del);
   return NULL;
}

/**
 * \brief Get the value for a given name/hash in the dictionary
 *
 * Retrieves the value of \a index to \a element in the dictionary \a dict.
 *
 * For names/indices that are of a type that is less than or equal to a
 * pointer width, the caller can simply cast each name on insert to \a void
 * \a * and
 * use \a NULL for all the \a xdict_new() arguments. If this is done then
 * the caller must cast back when retrieving the name (see example).
 *
 * For values that are of a type that is less than or equal to a pointer
 * width, the caller can simply cast each value to a \a void \a * on
 * insert. If this is done then the caller must cast back to the original
 * type when retrieving the value (see example).
 *
 * @param[in] dict The dictionary to use
 * @param[in] index The index identifying the element to return to the caller
 *
 * @return On success a pointer to the element identified by \a index
 * is returned.
 * On failure or if the element is not found \a NULL is returned.
 *
 * \a Example 
 * \verbatim
   // The following three static functions are for example two below

   static int dict_string_cmp (void *a1, void *a2)
   {
      return strcmp ((char *)a1, (char *)a2);
   }

   static void *dict_string_cpy (void *s)
   {
      return xstr_dup (s); // duplicate the string
   }

   static void dict_string_del (void *s)
   {
      if (s) free (s);
   }

   int main (void)
   {
      // First example: using hash entries with each name less 
      // than a pointer width
      // All NULL's indicating that datatype for hash is less
      // than a pointer width
      xdict_t *hashmap = xdict_new (NULL, NULL, NULL);
   
      // Our hashmap will use ints for name and chars for values
      int v1[] = {10, 20, 30, 40, 50, 60, 70, 80, 90, 100};
      char v2[] = "1234567890";

      // Set all the dictionary entries
      for (size_t i=0; i < sizeof v1/sizeof v1[0]; i++) {
         xdict_set (hashmap, (void *)v1[i], (void *)v2[i]);
      }
      // Check that they are all set properly
      for (size_t i=sizeof v1/sizeof v1[0]; i>0; i--) {
         int index = v1[i-1];
         char value = (char)xdict_get (hashmap, (void *)index);
         printf ("For name=%i, got value '%c'\n", index, value);
      }
      // Free the map and all entries it holds
      xdict_del (hashmap);

      // Second example: Using custom comparison, copy and deletion 
      // functions that uses strings as the hash/name and ints as the
      // value
      hashmap = xdict_new (dict_string_cmp,
                           dict_string_cpy, 
                           dict_string_del);
      char *v4[] = {"one", "two", "three", "four", "five",
                     "six", "seven"};
      // Add in dictionary entries using strings as the hashes/names and
      // ints as the value
      for (size_t i=0; i<sizeof v4/sizeof v4[0]; i++) {
         xdict_set (hashmap, v4[i], (void *)v1[i]);
      }
      // Print it all out to make sure that they got set
      for (size_t i=sizeof v4/sizeof v4[0]; i>0; i--) {
         char *index = v4[i-1];
         int value = (char)xdict_get (hashmap, index);
         printf ("For name=%s, got value '%i'\n", index, value);
      }
      // Delete the hash and all it's entries
      xdict_del (hashmap);

      return EXIT_SUCCESS;
   }
  \endverbatim
 */
void *xdict_get (xdict_t *dict, void *index)
{
   for (size_t i=0; i<XVECT_LENGTH (dict->array); i++) {
      xp_pair_t *pair = XVECT_INDEX (dict->array, i);
      if (dict->cmp (pair->name, index)==0) {
         return pair->value;
      }
   }
   return NULL;
}

/**
 * \brief Iterate over all entries in the dictionary
 *
 * Iterates over all entries in the dictionary \a dict, calling the function
 * \a fptr() for each tuple of \a name, \a value in the dictionary.
 *
 * @param[in] dict The dictionary to iterate over
 * @param[in] fptr The function to call on each name/value pair in the
 * dictionary \a dict
 *
 * @return Nothing.
 *
 * \a Example 
 * \verbatim
   // The following three static functions are for example two below

   static int dict_string_cmp (void *a1, void *a2)
   {
      return strcmp ((char *)a1, (char *)a2);
   }

   static void *dict_string_cpy (void *s)
   {
      return xstr_dup (s); // duplicate the string
   }

   static void dict_string_del (void *s)
   {
      if (s) free (s);
   }

   static void dict_string_int_print (void *n, void *v)
   {
      printf ("name='%s', value='%i'\n", (char *)n, (int)v);
   }

   int main (void)
   {
      hashmap = xdict_new (dict_string_cmp,
                           dict_string_cpy, 
                           dict_string_del);
      char *v4[] = {"one", "two", "three", "four", "five",
                     "six", "seven"};
      int v1[] = {10, 20, 30, 40, 50, 60, 70, 80, 90, 100};

      // Add in dictionary entries using strings as the hashes/names and
      // ints as the value
      for (size_t i=0; i<sizeof v4/sizeof v4[0]; i++) {
         xdict_set (hashmap, v4[i], (void *)v1[i]);
      }
      // Print it all out to make sure that they got set
      xdict_iterate (xtmp, dict_string_int_print);

      // Delete the hash and all it's entries
      xdict_del (hashmap);

      return EXIT_SUCCESS;
   }
  \endverbatim
 */
void xdict_iterate (xdict_t *dict, void (*fptr) (void *, void *))
{
   for (size_t i=0; i<XVECT_LENGTH (dict->array); i++) {
      xp_pair_t *pair = XVECT_INDEX (dict->array, i);
      fptr (pair->name, pair->value);
   }
}

/**
 * \brief Map a predicate over all the entries in the dictionary
 *
 * Iterates over all entries in the dictionary \a dict, calling the function
 * \a fptr() for each tuple of \a name, \a value in the dictionary and storing
 * the results in a null-terminated array of \a void \a * which is returned
 * to the caller.
 *
 * @param[in] dict The dictionary to iterate over
 * @param[in] fptr The function to call on each name/value pair in the
 * dictionary \a dict
 *
 * @return An array of \a void \a * containing the results of each 
 * invocation of \a fptr() using the entry's \a name/value pair.
 *
 * \a Example 
 * \verbatim
   // The following three static functions are for example two below

   static int dict_string_cmp (void *a1, void *a2)
   {
      return strcmp ((char *)a1, (char *)a2);
   }

   static void *dict_string_cpy (void *s)
   {
      return xstr_dup (s); // duplicate the string
   }

   static void dict_string_del (void *s)
   {
      if (s) free (s);
   }

   static void *dict_string_int_map (void *n, void *v)
   {
      return xstr_dup ((char *)n);
   }

   int main (void)
   {
      hashmap = xdict_new (dict_string_cmp,
                           dict_string_cpy, 
                           dict_string_del);
      char *v4[] = {"one", "two", "three", "four", "five",
                     "six", "seven"};
      int v1[] = {10, 20, 30, 40, 50, 60, 70, 80, 90, 100};

      // Add in dictionary entries using strings as the hashes/names and
      // ints as the value
      for (size_t i=0; i<sizeof v4/sizeof v4[0]; i++) {
         xdict_set (hashmap, v4[i], (void *)v1[i]);
      }
      // Print all the names in the dictionary 
      char **results = xdict_map (xtmp, dict_string_int_print);
      char **tmp = results;
      while (*tmp) {
         printf ("%s\n", *tmp);
         free (*tmp);
         tmp++;
      }

      // Delete the hash and all it's entries
      xdict_del (hashmap);

      return EXIT_SUCCESS;
   }
  \endverbatim
 */
void *xdict_map (xdict_t *dict, void *(*fptr) (void *, void *))
{
   void **ret = malloc ((XVECT_LENGTH(dict->array)+1) * sizeof *ret);
   if (!ret) return NULL;
   memset (ret, 0, (XVECT_LENGTH (dict->array)+1) * sizeof *ret);
   for (size_t i=0; i<XVECT_LENGTH (dict->array); i++) {
      xp_pair_t *pair = XVECT_INDEX (dict->array, i);
      ret[i] = fptr (pair->name, pair->value);
   }
   return ret;
}

/**
 * \brief Return a list of all the dictionary's indices
 *
 * Returns a null-terminated list of all the dictionary's indices. The caller
 * is responsible for freeing each index and for freeing the returned array
 * itself.
 *
 * @param[in] dict The dictionary to iterate over
 *
 * @return An array of \a void \a * containing the index of each 
 * item stored in the dictionary. The caller is responsible for freeing
 * each item in the array as well as the array itself.
 *
 * \a Example 
 * \verbatim
   // The following three static functions are for example two below

   static int dict_string_cmp (void *a1, void *a2)
   {
      return strcmp ((char *)a1, (char *)a2);
   }

   static void *dict_string_cpy (void *s)
   {
      return xstr_dup (s); // duplicate the string
   }

   static void dict_string_del (void *s)
   {
      if (s) free (s);
   }

   int main (void)
   {
      hashmap = xdict_new (dict_string_cmp,
                           dict_string_cpy, 
                           dict_string_del);
      char *v4[] = {"one", "two", "three", "four", "five",
                     "six", "seven"};
      int v1[] = {10, 20, 30, 40, 50, 60, 70, 80, 90, 100};

      // Add in dictionary entries using strings as the hashes/names and
      // ints as the value
      for (size_t i=0; i<sizeof v4/sizeof v4[0]; i++) {
         xdict_set (hashmap, v4[i], (void *)v1[i]);
      }
      // Print all the names in the dictionary 
      char **results = xdict_get_all (xtmp);
      char **tmp = results;
      while (*tmp) {
         printf ("%s\n", *tmp);
         free (*tmp);
         tmp++;
      }
      free (results);

      // Delete the hash and all it's entries
      xdict_del (hashmap);

      return EXIT_SUCCESS;
   }
  \endverbatim
 */
char **xdict_list_indices (xdict_t *dict)
{
   char **ret = malloc (sizeof *ret * (XVECT_LENGTH (dict->array)+1));
   if (!ret) {
      goto errorexit;
   }
   memset (ret, 0, sizeof *ret * (XVECT_LENGTH (dict->array)+1));
   for (size_t i=0; i<xvector_length (dict->array); i++) {
      xp_pair_t *pair = XVECT_INDEX (dict->array, i);
      ret[i] = dict->cpy ? dict->cpy (pair->name) : pair->name;
      ret[i+1] = NULL;
      if (!ret[i]) {
         goto errorexit;
      }
   }

   return ret;

errorexit:
   if (ret) {
      for (size_t i=0; ret[i]; i++) {
         free (ret[i]);
      }
      free (ret);
   }
   return NULL;
}
