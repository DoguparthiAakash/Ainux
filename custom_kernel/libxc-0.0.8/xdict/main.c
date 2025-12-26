
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#include "xstring/xstring.h"
#include "xdict/xdict.h"

int dict_string_cmp (void *a1, void *a2)
{
   return strcmp ((char *)a1, (char *)a2);
}

void *dict_string_cpy (void *s)
{
   return xstr_dup (s);
}

void dict_string_del (void *s)
{
   if (s) free (s);
}

#define CAST64(x)       ((int)(long long)x)
#define CASTVOID(x)     ((void *)(long long)x)

void dict_string_int_print (void *n, void *v)
{
   printf ("name='%s', value='%i'\n", (char *)n, CAST64(v));
}

void *dict_string_int_map (void *n, void *v)
{
   v = v;
   return xstr_dup ((char *)n);
}

int main (void)
{
   xdict_t *xtmp = xdict_new (NULL, NULL, NULL);

   printf ("Testing xdict\n");

   int v1[] = {10, 20, 30, 40, 50, 60, 70, 80, 90, 100};
   char v2[] = "1234567890";
   char v3[] = "ABCDEFGHIJ";

   printf ("int=%zu, vp=%zu\n", sizeof (int), sizeof (void *));
   for (size_t i=0; i < sizeof v1/sizeof v1[0]; i++) {
      xdict_set (xtmp, CASTVOID (v1[i]), CASTVOID (v2[i]));
   }
   for (size_t i=sizeof v1/sizeof v1[0]; i>0; i--) {
      int index = v1[i-1];
      char value = (char)(long long)xdict_get (xtmp, CASTVOID (index));
      printf ("For name=%i, got value '%c'\n", index, value);
   }
   printf ("==================================\n");
   for (size_t i=0; i<sizeof v1/sizeof v1[0]; i += 2) {
      xdict_set (xtmp, CASTVOID (v1[i]), CASTVOID (v3[i]));
   }
   for (size_t i=sizeof v1/sizeof v1[0]; i>0; i--) {
      int index = v1[i-1];
      char value = (char)(long long)xdict_get (xtmp, CASTVOID (index));
      printf ("For name=%i, got value '%c'\n", index, value);
   }
   printf ("==================================\n");
   xdict_del (xtmp);

   xtmp = xdict_new (dict_string_cmp, dict_string_cpy, dict_string_del);
   char *v4[] = {"one", "two", "three", "four", "five", "six", "seven"};
   for (size_t i=0; i<sizeof v4/sizeof v4[0]; i++) {
      xdict_set (xtmp, v4[i], CASTVOID (v1[i]));
   }

   for (size_t i=sizeof v4/sizeof v4[0]; i>0; i--) {
      char *index = v4[i-1];
      int value = (char)(long long)xdict_get (xtmp, index);
      printf ("For name=%s, got value '%i'\n", index, value);
   }
   printf ("==================================\n");

   for (size_t i=0; i<sizeof v4/sizeof v4[0]; i += 2) {
      xdict_set (xtmp, v4[i], CASTVOID (v3[i]));
   }
   xdict_iterate (xtmp, dict_string_int_print);
   printf ("==================================\n");

   char **results = xdict_map (xtmp, dict_string_int_map);
   char **tmp = results;
   while (*tmp) {
      printf ("%s\n", *tmp);
      free (*tmp);
      tmp++;
   }
   free (results);

   printf ("**************************\n");
   results = xdict_list_indices (xtmp);
   tmp = results;
   while (tmp && *tmp) {
      printf ("%s\n", *tmp);
      free (*tmp);
      tmp++;
   }
   free (results);
   xdict_del (xtmp);
   return EXIT_SUCCESS;
}
