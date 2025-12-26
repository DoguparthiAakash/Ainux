
#ifndef H_XSTRING
#define H_XSTRING ("1.0.0")

#include <stdio.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

   char *xstr_dup (char const *src);
   char *xstr_ncpy (char *dst, char const *src, size_t nchars);
   char *xstr_cat (char const *first, ...);
   char *xstr_join (char *const array[], char delim);

   char **xstr_split (char const *full_string, char const *delim);
   size_t xstr_match_first (const char *needle, ...);
   size_t xstr_match_first_a (const char *needle, char *const *haystack);

   char *xstr_rtrim (char *str);
   char *xstr_ltrim (char *str);
   char *xstr_trim (char *str);

   char **xstr_cpyarray (char **array);
   void xstr_delarray (char **array);
   
   char *xstr_readfile (const char *filename);

   const char *xstr_chr (const char *haystack, const char *needles);
   const char *xstr_rchr (const char *haystack, const char *needles);
   char *xstr_fmt (const char *input, size_t width);

   char *xstr_escape (const char *text,const char esc, const char *esc_chars);
   char *xstr_unescape (const char *text,const char esc, const char *esc_chars);

   char **xstr_parse_nv (const char *line, char delim, char comment);

#ifdef __cplusplus
};
#endif




#endif
