
/**
 * \file 
 *
 * \brief Extra string routines for C
 *
 * xstring is a library of routines to provide easy string manipulation
 * in C. xstring is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 *
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <ctype.h>

#include "xvector/xvector.h"
#include "xstring/xstring.h"

/**
 * \brief Duplicate the given null-terminated string 
 *
 * Duplicates the given string by allocating space using malloc and copying
 * the given string into the newly allocated space. The caller is responsible
 * for freeing the return value.
 *
 * @param[in] src The source string to be duplicated.
 *
 * @return On success, a newly allocated block of memory which contains the
 * \a src string. On failure \a NULL is returned. The caller must free 
 * the returned pointer on succcess.
 */
char *xstr_dup (char const *src)
{
   if (!src) return NULL;
   size_t len = strlen (src) + 1;
   char *ret = malloc (len);
   if (!ret) return NULL;
   strcpy (ret, src);
   return ret;
}

/**
 * \brief Safe string copy
 *
 * Safely copy the \a src string to the location pointed to by \a dst, copying
 * at most \a nchars \a - \a 1 characters. The copied string is properly
 * null-terminated.
 *
 * 
 * @param[out] dst The destination to copy \a src to. The caller must ensure
 * that at least \a nchars characters will fit into \a dst.
 * @param[in] src The source string to be copied.
 * @param[in] nchars The number of characters that will fit into \a dst.
 *
 * @return On success \a dst is returned and \a dst will contain a copy of
 * \a src of up to \a nchars \a - \a 1 characters. On failure \a NULL is
 * returned.
 * 
 * \a Example:
 *
 * \verbatim
   int main (void)
   {
      char t1[] = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
      char *t2 = "Hello World!";
      char *result = xstr_ncpy (t1, t2, sizeof t1);
      // result now contains the string "Hello World!"
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
char *xstr_ncpy (char *dst, char const *src, size_t nchars)
{
   if (!src || !dst) return NULL;
   memset (dst, 0, nchars);
   strncpy (dst, src, nchars - 1);
   return dst;
}

/**
 * \brief Multiple string concatenation
 *
 * Concatenates all the given strings in the order that they are presented
 * in. The argument list must end with a \a NULL pointer. The return value
 * is malloc'ed by the function so the caller is responsible for freeing
 * the result.
 *
 * @param[in] first The mandatory first string in the list of strings to 
 * concatenate. The list of arguments to this function must end with a
 * \a NULL pointer.
 * 
 * @return On success, a newly allocated block of memory which contains all
 * the given strings concatenated into a single large string. On failure 
 * \a NULL is returned. The caller must free the returned pointer on succcess.
 *
 *
 * \verbatim
   int main (void)
   {
      char t1[] = "Hello ";
      char *t2 = "World, ";
      char *result = xstr_cat (t1, t2, "everyone ", "\n", NULL);
      // result now contains the string "Hello World, everyone \n"
      printf ("%s", result);
      // Caller responsible for freeing the return from xstr_cat()
      free (result);
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
char *xstr_cat (char const *first, ...)
{
   if (!first) return NULL;
   /* First, work out the length of the final string */
   size_t final_len = strlen (first) + 1;
   va_list ap;
   va_start (ap, first);
   char *as = va_arg (ap, char *);
   while (as) {
      final_len += strlen (as) + 1;
      as = va_arg (ap, char *);
   }
   va_end (ap);
   /* Allocate space for the final string (return value) */
   char *ret = malloc (final_len + 1);
   if (!ret) return NULL;
   /* Copy all the arguments into the return value */
   strcpy (ret, first);
   va_start (ap, first);
   as = va_arg (ap, char *);
   while (as) {
      strcat (ret, as);
      as = va_arg (ap, char *);
   }
   va_end (ap);
   return ret;
}

/**
 * \brief Multiple string concatenation (array of strings)
 *
 * Concatenates all the given strings in the array \a array in the order
 * that they are found in the array \a array, optionally separated with 
 * delimiter \a delim. The final element of the array \a array must be 
 * a \a NULL pointer because xstr_join() doesn't receive the length of the
 * the array. The final \a NULL pointer in the array signifies the end of
 * the array to xstr_join(), thus xstr_join() will stop concatenation upon
 * reaching the first element in the array that evaluates to NULL.
 *
 * The return value is malloc'ed by the function so the caller is
 * responsible for freeing the result.
 *
 * @param[in] array An array of char pointers, representing the array of
 * strings that are to be concatenated. The last element of this array must
 * be a NULL pointer.
 * @param[in] delim Optional delimiter for the resulting string. If non-zero,
 *  this character is used to separate each of the strings from \a array
 * in the result.
 * 
 * @return On success, a newly allocated block of memory which contains all
 * the given strings concatenated into a single large string, optionally 
 * separated by \a delim. On failure \a NULL is returned. The caller must
 * free the returned pointer on succcess.
 *
 *
 * \verbatim
   int main (void)
   {
      char *a_of_s[] = {"One ", "Two ", "Three ", "Four ", "Five ", NULL};
      char *result = xstr_join (a_of_s, 0);
      printf ("a_of_s = '%s'\n", result);
      // outputs:    a_of_s = 'One Two Three Four Five'
      free (result);

      result = xstr_join (a_of_s, ':');
      printf ("a_of_s = '%s'\n", result);
      // outputs:    a_of_s = 'One :Two :Three :Four :Five'
      free (result);

      return EXIT_SUCCESS;
   }
  \endverbatim
 */
char *xstr_join (char *const array[], char delim)
{
   size_t ret_size = 1;
   size_t num_strings = 0;
   for (size_t i=0; array[i]; i++) {
      ret_size += strlen (array[i]) + 1; // extra byte for delimiter
      num_strings++;
   }
   ret_size++;
   char *ret = malloc (ret_size);
   if (!ret) return NULL;
   char *dst = ret;
   for (size_t i=0; array[i]; i++) {
      size_t l = strlen (array[i]);
      memcpy (dst, array[i], l);
      dst += l;
      num_strings--;
      if (delim && num_strings) *dst++ = delim;
   }
   *dst = 0;
   return ret;
}

/**
 * \brief Split a string into multiple substrings
 *
 * Splits the string \a full_string into multiple substrings using \a delim
 * as the set of characters that delimit the substrings in the original string.
 * Any delimiter character that appears in the string that is not a delimiter
 * should be escaped.
 *
 * For example, given \a full_string = "this,is,a,string:that.is.delimited" 
 * and the set of delimiters as \a delim = ":,." xstr_split() will return
 * the set of strings "this", "is", "a", "string", "that", "is" and 
 * "delimited". If \a full_string ends with a delimiter before the terminating
 * NULL, then the final empty field is omitted.
 *
 * The caller is responsible for freeing the result (See section "RETURNS"
 * and the example below for more information).
 *
 * @param[in] full_string A null-terminated C string
 * @param[in] delim The set of delimiters to use, in the form of a 
 * null-terminated C string
 * 
 * @return On success, a newly allocated array which contains all
 * the substrings found, with the final element of the array being \a NULL. 
 * Each substring in the array is also newly allocated. The caller is
 * responsible for freeing each substring and for freeing the array itself.
 * On failure, NULL is returned.
 * 
 *
 * \verbatim
   int main (void)
   {
      char *mf_string = ".this is.a:string\\,that?is'delimited..";
      char *delims = ".:,?'";

      char **result = xstr_split (mf_string, delims);
      // Print out each element of result, and then free it
      for (size_t i=0; result && result[i]; i++) {
         printf ("substr='%s'\n", result[i]);
         free (result[i]);
      }
      // Free the entire array
      if (result) free (result);
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
char **xstr_split (char const *full_string, char const *delim)
{
#define DELIMITER       ((char)0xff)
   if (!full_string || !*full_string) return NULL;
   char *tmpstr = xstr_dup (full_string);
   size_t num_ret = 1;
   if (!tmpstr || *tmpstr==0) return NULL;
   size_t len = strlen (tmpstr);
   // The first character, if a delimiter, cannot be escaped.
   if (strchr (delim, tmpstr[0])) {
      num_ret++;
      tmpstr[0] = DELIMITER;
   }
   // For all the other characters, determine the boundaries on
   // the string
   for (size_t i=1; i<len; i++) {
      if (tmpstr[i]=='\\') {
         i++;
         continue;
      }
      char *tc = strchr (delim, tmpstr[i]);
      if (tc) {
         tmpstr[i] = DELIMITER;
         num_ret++;
      }
   }

   // Construct the return string, with extra space for final NULL pointer
   char **ret = malloc (sizeof *ret * (num_ret + 1));
   if (!ret) goto error;
   memset (ret, 0, (sizeof *ret * num_ret) + 1);
   char *start, *end;
   start = end = tmpstr;
   size_t index = 0;
   while (*start && *end) {
      while (*end && *end!=DELIMITER)
         end++;
      char *subs = malloc (end - start + 1);
      if (!subs) goto error;
      memcpy (subs, start, end - start);
      subs[end - start] = 0;
#if 0
      // This code was turned on during testing to ensure that no memory
      // leaks resulted in the even of an error in the inner loop
      if (index==3) {
         free (subs);
         goto error;
      }
#endif
      ret[index++] = subs;
      if (!*end) break;
      end++; start = end;
   }
   ret[index] = 0;
   free (tmpstr);
   return ret;
error:
   if (tmpstr) free (tmpstr);
   if (ret) {
      for (size_t i=0; ret[i]; i++) {
         free (ret[i]);
      }
      free (ret);
   }
   return NULL;
}

/**
 * \brief Match a string to a list of parameter strings
 *
 * Given a string, \a needle, and a list of strings to examine, 
 * \a xstr_match_first() will determine which of the strings in \a (...)
 * match \a needle, and return the position of that string in \a (...)
 *
 * For example the caller can use \a xstr_match_first() in a switch statement
 * that decides based on strings which decision path to take (see example
 * below). The argument list \a (...) must be terminated with a NULL pointer.
 *
 * Note that xstr_match_first() uses strcmp to match \a needle and thus
 * will incur much overhead. It is recommended that xstr_match_first() not
 * be used in loops with many iterations.
 *
 * \sa xstr_match_first_a()
 *
 * @param[in] needle The string to match all the others against
 * @param[in] ... The set of strings to search for \a needle, terminated
 * with a NULL
 *
 * @return On success the index (starting from 0) of the string in \a (...)
 * that matches \a needle is returned. On failure or failure to match,
 * (size_t)-1 is returned.
 *
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      char input[25];
      memset (input, 0, sizeof input);
      printf ("Enter command to execute: \n");
      fgets (input, sizeof input - 1, stdin);
      switch (xstr_match_first (input, "play", 
                                       "pause", 
                                       "restart", 
                                       "stop", 
                                       NULL))
      {
         case 0: // take action for "play";     break
         case 1: // take action for "pause";    break
         case 2: // take action for "restart";  break
         case 3: // take action for "stop";     break
         case (size_t)-1:
         default:
            // Unknown input
      }
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
size_t xstr_match_first (const char *needle, ...)
{
   va_list ap;
   if (!needle) return (size_t)-1;
   va_start (ap, needle);
   const char *haystack = va_arg (ap, const char *);
   size_t index = 0;
   while (haystack) {
      if (strcmp (needle, haystack)==0) return index;
      index++;
      haystack = va_arg (ap, char *);
   }
   va_end (ap);
   return (size_t)-1;
}


/**
 * \brief Find a string in an array of strings
 *
 * Given a string, \a needle, and array \a haystack of strings to examine, 
 * \a xstr_match_first() will determine which of the strings in \a haystack
 * match \a needle, and return the position of that string in \a haystack.
 *
 * For example the caller can use \a xstr_match_first_a() in a switch statement
 * that decides based on strings which decision path to take (see example
 * below). The array \a haystack must be terminated with a NULL pointer.
 *
 * Note that xstr_match_first_a() uses strcmp to match \a needle and thus
 * will incur much overhead. It is recommended that xstr_match_first_a() not
 * be used in loops with many iterations.
 *
 * \sa xstr_match_first()
 *
 * @param[in] needle The string to match all the others against
 * @param[in] haystack The array of strings to search for \a needle,
 * terminated with a NULL pointer
 *
 * @return On success the index (starting from 0) of the string in
 * \a haystack that matches \a needle is returned. On failure or failure to
 * match, (size_t)-1 is returned.
 *
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      char *commands[] = { "play", "pause", "restart", "stop", NULL, };
      char input[25];
      memset (input, 0, sizeof input);
      printf ("Enter command to execute: \n");
      fgets (input, sizeof input - 1, stdin);
      switch (xstr_match_first_a (input, commands))
      {
         case 0: // take action for "play";     break;
         case 1: // take action for "pause";    break;
         case 2: // take action for "restart";  break;
         case 3: // take action for "stop";     break;
         case (size_t)-1:
         default:
            // Unknown input
            break;
      }
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
size_t xstr_match_first_a (const char *needle, char *const *haystack)
{
   for (size_t i=0; haystack[i]; i++) {
      if (strcmp (needle, haystack[i])==0) return i;
   }
   return (size_t)-1;
}

/**
 * \brief Removes all trailing whitespace from a string
 *
 * Removes all trailing whitespace from the given string \a str. The string
 * \a str is modified in place and no copies are made. The string \a str
 * is not reallocated.
 *
 * When \a str is empty the string \a str is returned unchanged. When \a str
 * is \a NULL, \a NULL is returned. Should no trailing whitespace be found
 * then the string \a str is returned unchanged. Should the string \a str be
 * nothing but whitespace, then \a str is made into an empty string and
 * returned.
 *
 * \sa xstr_ltrim(), xstr_trim()
 *
 * @param[in,out] str The string to trim off the trailing whitespace from.
 *
 * @return The \a str that was passed into the function is returned, with
 * any trailing whitespace found in it removed.
 *
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      char *test_string = xstr_dup ("testing \t \n ");
      xstr_rtrim (test_string);
      printf ("--%s--\n", test_string); // Outputs '--testing--'
      free (test_string);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
char *xstr_rtrim (char *str)
{
   if (!str) return NULL;
   char *tmp = &str[strlen(str)-1];
   while (tmp && tmp > str && isspace (*tmp)) tmp--;
   if (tmp==str && isspace (*tmp)) *tmp = 0;
   *(tmp+1) = 0;
   return str;
}

/**
 * \brief Removes all leading whitespace from a string
 *
 * Removes all leading whitespace from the given string \a str. The string
 * \a str is modified in place and no copies are made. The string \a str
 * is not reallocated.
 *
 * When \a str is empty the string \a str is returned unchanged. When \a str
 * is \a NULL, \a NULL is returned. Should no leading whitespace be found
 * then the string \a str is returned unchanged. Should the string \a str be
 * nothing but whitespace, then \a str is made into an empty string and
 * returned.
 *
 * \sa xstr_rtrim(), xstr_trim()
 *
 * @param[in,out] str The string to trim off the leading whitespace from.
 *
 * @return The \a str that was passed into the function is returned, with
 * any leading whitespace found in it removed.
 *
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      char *test_string = xstr_dup (" \t \n testing");
      xstr_ltrim (test_string);
      printf ("--%s--\n", test_string); // Outputs '--testing--'
      free (test_string);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
char *xstr_ltrim (char *str)
{
   if (!str) return NULL;
   char *tmp = str;
   while (tmp && isspace (*tmp)) tmp++;
   memmove (str, tmp, strlen (tmp)+1);
   return str;
}

/**
 * \brief Removes all leading and trailing whitespace from a string
 *
 * Removes all leading and trailing whitespace from the given string \a str.
 * The string
 * \a str is modified in place and no copies are made. The string \a str
 * is not reallocated.
 *
 * When \a str is empty the string \a str is returned unchanged. When \a str
 * is \a NULL, \a NULL is returned. Should neither leading whitespace nor
 * trailing whitespace be found
 * then the string \a str is returned unchanged. Should the string \a str be
 * nothing but whitespace, then \a str is made into an empty string and
 * returned.
 *
 * \sa xstr_rtrim(), xstr_trim()
 *
 * @param[in,out] str The string to trim off the leading and trailing whitespace 
 * from.
 *
 * @return The \a str that was passed into the function is returned, with
 * any leading and trailing whitespace found in it removed.
 *
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      char *test_string = xstr_dup (" \t \n testing \t \n ");
      xstr_trim (test_string);
      printf ("--%s--\n", test_string); // Outputs '--testing--'
      free (test_string);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
char *xstr_trim (char *str)
{
   return xstr_rtrim (xstr_ltrim (str));
}

/**
 * \brief Copies an array of strings (such as argv)
 *
 * Returns a copy of \a array, which must be an array of strings terminated
 * with a \a NULL pointer. The array \a array must be of the same form as
 * \verbatim
      char *array[] = {"one", "two", ..., NULL};
   \endverbatim
 *
 * The returned array is malloced (and thus must be freed by the caller)
 * and each element in the returned array is also malloced (and thus must
 * be freed by the caller). The caller should use xstr_delarray() to free
 * the returned value.
 *
 * \sa xstr_delarray()
 *
 * @param[in] array The array to make a copy of.
 *
 * @return On success a copy of the array \a array is returned. 
 * The return value is malloced and
 * must be freed by the caller. Each element in the returned array is also
 * malloced and must be freed by the caller. The caller should use 
 * xstr_delarray() to free the returned array. On any failure, NULL is 
 * returned.
 *
 * \a Example: Copy the command line arguments and print the copy.
 * \verbatim
   int main (int argc, char **argv)
   {
      char **argv_cpy = xstr_cpyarray (argv);
      argc = argc;
      for (size_t i=0; argv_cpy[i]; i++) {
         printf ("%zu: '%s'\n", i, argv_cpy[i]);
      }
      xstr_delarray (argv_cpy);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
char **xstr_cpyarray (char **array)
{
   size_t argc;
   for (argc=0; array[argc]; argc++) ;
   char **ret = malloc (sizeof *ret * (argc + 1));
   if (!ret) return NULL;
   memset (ret, 0, sizeof *ret * (argc + 1));
   for (size_t i=0; array[i]; i++) {
      ret[i] = xstr_dup (array[i]);
      // We have to return on the first NULL, even if not an error.
      // This is because other routines will stop processing this 
      // array when they encounter the first NULL, so our array
      // cannot have NULL's in it at all. This would cause a memory leak.
      if (!ret[i]) {
         xstr_delarray (ret);
         return NULL;
      }
   }
   return ret;
}

/**
 * \brief Deletes all resources used by a dynamically allocated array of 
 * strings
 *
 * Deletes the array \a array, which must be an array of strings. The array 
 * must be terminated with a NULL pointer. 
 *
 * The array \a array is deleted by first iterating through all the elements
 * and freeing each one until the terminating NULL element is found, after
 * which the array \a array itself is freed.
 * 
 * For example, making a copy of the command line arguments in \a argv
 * would result in the structure that xstr_delarray() will delete and free.
 *
 * \sa xstr_cpyarray()
 *
 * @param[in] array The array to delete
 *
 * @return Nothing
 *
 * \a Example: Copy the command line arguments and print the copy.
 * \verbatim
   int main (int argc, char **argv)
   {
      char **argv_cpy = xstr_cpyarray (argv);
      argc = argc;
      for (size_t i=0; argv_cpy[i]; i++) {
         printf ("%zu: '%s'\n", i, argv_cpy[i]);
      }
      xstr_delarray (argv_cpy);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
void xstr_delarray (char **array)
{
   if (!array) return;
   for (size_t i=0; array[i]; i++) {
      if (array[i]) free (array[i]);
   }
   free (array);
}

/**
 * \brief Reads and returns the given file as a single C-style string
 *
 * Reads the file \a filename and returns the entire file contents as a 
 * single C-style NULL-terminated string. The caller must ensure that
 * the returned NULL-terminated string is freed.
 *
 * If the file specified by \a filename doesn't exist or is otherwise
 * unreadable (permissions might not allow, for example), then NULL
 * is returned.
 *
 * \sa xfile_readable()
 *
 * @param[in] filename The filename to read and return as a single string
 *
 * @return A single string, terminated with a \a NULL character, that
 * contains the contents of the file \a filename
 *
 * \a Example: Program that prints its own source
 * \verbatim
   int main (void)
   {
      char *myself = xstr_readfile ("main.c");
      if (myself) {
         printf ("Read in %zu bytes in main.c\n", strlen (myself));
         printf ("%s\n", myself);
         free (myself);
      }
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
char *xstr_readfile (const char *filename)
{
   if (!filename) return NULL;
   FILE *inf = fopen (filename, "r");
   if (!inf) return NULL;
   rewind (inf);
   long startpos = ftell (inf);
   fseek (inf, 0, SEEK_END);
   long endpos = ftell (inf);
   size_t filesize = endpos - startpos;
   char *ret = malloc (filesize + 1);
   if (!ret) {
      fclose (inf);
      return NULL;
   }
   rewind (inf);
   size_t bytes_read = fread (ret, 1, filesize, inf);
   if (!feof (inf) && bytes_read!=filesize) {
      free (ret);
      fclose (inf);
      return NULL;
   }
   fclose (inf);
   ret[filesize] = 0;
   return ret;
}

/**
 * \brief Searches string for first occurrence of any of a set of characters
 *
 * Searches the string \a haystack for any character in \a needles and 
 * returns a pointer to the first match in \a haystack.
 *
 * \sa xstr_rchr()
 *
 * @param[in] haystack The string to search in
 * @param[in] needles The set of characters to search for within \a haystack
 *
 * @return A pointer to the first occurence of any of the characters from
 *    \a needles that occurr within \a haystack. The return value should
 *    not be freed by the caller as it is an address within \a haystack.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      // Will find the first occurrence of any the characters in ":,;"
      // in the string of fields given
      char *second_field = xstr_chr ("Field 1: field 2, field 3",
                                     ":,;");
      // second_field = " field 2, field 3";
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
const char *xstr_chr (const char *haystack, const char *needles)
{
   while (*haystack) {
      if (strchr (needles, *haystack)) {
         return haystack;
      }
      haystack++;
   }
   return NULL;
}

/**
 * \brief Searches string for last occurrence of any of a set of characters
 *
 * Searches the string \a haystack for any character in \a needles and 
 * returns a pointer to the last match in \a haystack.
 *
 * \sa xstr_chr()
 *
 * @param[in] haystack The string to search in
 * @param[in] needles The set of characters to search for within \a haystack
 *
 * @return A pointer to the last occurence of any of the characters from
 *    \a needles that occurr within \a haystack. The return value should
 *    not be freed by the caller as it is an address within \a haystack.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      // Will find the last occurrence of any the characters in ":,;"
      // in the string of fields given
      char *last_field = xstr_chr ("Field 1: field 2, field 3",
                                     ":,;");
      // lastecond_field = "  field 3";
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
const char *xstr_rchr (const char *haystack, const char *needles)
{
   const char *tmp = &haystack[strlen(haystack)];
   while (tmp!=haystack) {
      if (strchr (needles, *tmp)) {
         return tmp;
      }
      tmp--;
   }
   return NULL;
}

/**
 * \brief Formats a paragraph into specified width
 *
 * Formats the given paragraph \a input into lines of not more than
 * \a width characters wide, with the actual width of each line being the 
 * larger of the specified \a width and the maximum word length that occurs
 * in the input string. 
 *
 * Existing linebreaks in \a input are removed. To force a linebreak use
 * the two character code "\n", represented as a C-string "\\n".
 *
 * @param[in] input The input text to format
 * @param[in] width The maximum width for each line in the resulting
 * paragraph. The actual maximum is the larger of the specified maximum
 * width and the largest word encountered in the input \a input.
 *
 * @return A NULL-terminated string containing the entire paragraph, with
 * linebreaks at every width character or less. The caller has to free the
 * return value.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      // Format the HELP_MSG #defined string into a paragraph that
      // fits in most terminals
      char *help_msg = xstr_fmt (HELP_MSG, 62);
      fprintf (stderr, "%s\n", help_msg);
      free (help_msg);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
char *xstr_fmt (const char *input, size_t width)
{
   char *ret = xstr_dup (input);
   if (!ret) return NULL;
   // Determine what width we should actually be using
   char *tok = strtok (ret, " -=");
   while (tok) {
      size_t tmpwidth = strlen (tok);
      if (tmpwidth>width) {
         width = tmpwidth;
      }
      tok = strtok (NULL, " -=");
   }
   free (ret); ret = xstr_dup (input);
   if (!ret) return NULL;
   // replace all the '\\n' with special code 0x01
   char *tmp = ret;
   char *nnl;
   while ((nnl = strstr (tmp, "\\n"))) {
      if (nnl>(tmp + 1) && *(nnl-1)=='\n') {
         nnl[0] = nnl[1] = ' ';
         tmp = nnl;
         continue;
      }
      if (nnl[2] && nnl[2]=='\n') {
         nnl[0] = nnl[1] = ' ';
         continue;
      }
      nnl[0] = '\n';
      nnl[1] = 0x01;
      tmp = &nnl[1];
   }
   // replace all the linebreaks with spaces
   char *lb = strchr (ret, '\n');
   while (lb) {
      *lb = ' ';
      lb = strchr (ret, '\n');
   }
   char *replace_pos = ret;
   char *last_line = ret;
   while (*replace_pos) {
      char *tmp;
      if (*replace_pos=='\n') *replace_pos = ' ';
      while (replace_pos && replace_pos - last_line <= (long)width) {
         tmp = (char *)xstr_chr (replace_pos+1, " -=");
         if (!tmp) {
            if (replace_pos - last_line >= (long)width) *replace_pos = '\n';
            replace_pos = NULL;
            break;
         }
         replace_pos = tmp;
      }
      if (!replace_pos) break;
      replace_pos = tmp;
      replace_pos--;
      if (replace_pos<ret) break;
      while (replace_pos >= ret
               && *replace_pos != ' '
               && *replace_pos != '-'
               && *replace_pos != '=')
         replace_pos--;
      *replace_pos++ = '\n';
      last_line = replace_pos;
   }

   // Replace all 0x01 with newlines
   tmp = ret;
   while (tmp && *tmp) {
      *tmp = *tmp==0x01 ? '\n' : *tmp;
      tmp++;
   }
   return ret;
}

/**
 * \brief Escapes the supplied C-string
 *
 * Escapes the supplied string \a text by prefixing each occurrence of any
 * character in \a esc_chars with the character \a esc. The returned
 * string must be freed by the caller.
 *
 * \sa xstr_unescape()
 *
 * @param[in] text The input text to escape
 * @param[in] esc The character to use to escape
 * @param[in] esc_chars The characters that must be escaped
 *
 * @return A NULL-terminated string containing the input \a text, with
 * all occurrences of the characters in \a esc_char escaped using the
 * char \a esc. The caller must free the return value.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      char *query = 
         xstr_escape ("Select * from bobby'; drop tables;", '\\',
                      "\"'\;");
      // ... Send query string to the database
      char *org_query = xstr_unescape (query, '\\', "\"'\;");
      printf ("Query: %s\n", org_query);
      free (query);
      free (org_query);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
char *xstr_escape (const char *text, const char esc, const char *esc_chars)
{
   size_t len = (strlen (text) * 2) + 1;
   char *ret = malloc (len);
   if (!len) return NULL;
   char *tmp = ret;
   while (*text) {
      if (strchr (esc_chars, *text)) *tmp++ = esc;
      *tmp++ = *text++;
   }
   *tmp = 0;
   return ret;
}

/**
 * \brief Undoes escaped characters in the supplied C-string
 *
 * Undoes any escaping of \a text that was performed by \a xstr_escape().
 * The character used to detect an escape sequence is \a esc. Only those
 * characters found in \a esc_chars are unescaped. An escape sequence which
 * does not contain the characters in \a esc_chars are ignored.
 *
 * The caller must free the returned string.
 *
 * \sa xstr_escape()
 *
 * @param[in] text The input text to escape
 * @param[in] esc The character that was used as the escape character
 * @param[in] esc_chars The characters that were escaped. Characters other
 * than those in this set will be ignored even if they otherwise form valid
 * escape sequences.
 *
 * @return A NULL-terminated string containing the input \a text, with
 * all occurrences of escape sequences using \a esc as the escape character
 * and \a esc_chars as the set of characters that can be escaped removed.
 * The caller must free the return value.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      char *query = 
         xstr_escape ("Select * from bobby'; drop tables;", '\\',
                      "\"'\;");
      // ... Send query string to the database
      char *org_query = xstr_unescape (query, '\\', "\"'\;");
      printf ("Query: %s\n", org_query);
      free (query);
      free (org_query);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
char *xstr_unescape (const char *text, const char esc, const char *esc_chars)
{
   char *ret = malloc (strlen (text) + 1);
   char *tmp = ret;
   while (*text) {
      if (*text==esc && strchr (esc_chars, *(text+1))) {
         text++; 
         continue;
      }
      *tmp++ = *text++;
   }
   *tmp = 0;
   return ret;
}

/**
 * \brief Parse a line of text into name/value pair
 *
 * Given a string of the form:
 * \verbatim
   name = value
   \endverbatim
 * \a xstr_parse_nv() parses the string \a line to retrieve the name and 
 * the value
 * and return both as an array (of length two or less) of null-terminated 
 * C strings. The parameter \a delim can be used to specify a delimiter 
 * other than '=', or set to 0 to use '=' as the delimiter.
 *
 * If there is neither a delimiter nor a value, then the value is set to 
 * NULL. All text after the comment character is ignored. If the comment
 * character is set to 0, then no comments are allowed and all text will
 * be parsed.
 *
 * The caller must free the returned array of strings, either manually or
 * using \a xstr_delarray() on the return value. 
 * 
 * The caller may set a delimiter using the argument \a delim. If \a delim
 * is 0 then the default '=' is used as a delimiter.
 *
 * \sa xstr_delarray()
 *
 * @param[in] line The line of text to parse. If string is empty or NULL
 *    or only contains whitespace it is ignored and NULL is returned.
 * @param[in] delim The (optional) delimiter. If set to 0 then '=' will be
 *    used.
 * @param[in] comment The (optional) comment. If set to 0 then '=' will be
 *    used.
 *
 * @return On success a NULL-terminated array of strings containing the name
 * as element
 * 0 and the value as element 1 is returned. If no value is present or 
 * no delimiter
 * is found then the value is set to NULL. On error NULL is returned.
 * The caller must free the return value, either manually or by using
 * \a xstr_delarray().
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      char *line = " name1 = value1    # This is a comment";
      char **nvpair = xstr_parse_nv (line, 0);
      printf ("name: %s\nvalue: %s", nvpair[0], nvpair[1]);
      xstr_delarray (nvpair);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */ 
char **xstr_parse_nv (const char *line, char delim, char comment)
{
   // Make a working copy of the string
   if (!line) return NULL;
   char *lline = xstr_dup (line);
   if (!lline) return NULL;

   // Remove any comments if necessary
   char *scomment = strchr (lline, comment);
   if (scomment) *scomment = 0;

   // Find the value
   char ldelim = delim ? delim : '=';
   char *boundary = strchr (lline, ldelim);
   if (boundary) {
      *boundary = 0;
      boundary++;
   }

   // Prep the return array
   char **ret = malloc (sizeof *ret * 3);
   if (!ret) {
      free (lline);
      return NULL;
   }

   // Copy the values to the return array
   ret[0] = xstr_dup (lline);
   ret[1] = xstr_dup (boundary);
   ret[2] = NULL;
   free (lline);

   // Trim whitespace on returned strings
   ret[0] = xstr_trim (ret[0]);
   ret[1] = xstr_trim (ret[1]);

   // Done
   return ret;
}


































