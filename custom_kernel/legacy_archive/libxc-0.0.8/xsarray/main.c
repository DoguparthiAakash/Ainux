
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "xsarray/xsarray.h"


int main (void)
{
   char *elements[] = {
      "the", "quick", "brown", "fox", "jumped",
      "over", "the", "lazy", "dog",
   };

   xsarray_t *array = xsarray_new ((int (*)(void *, void *))strcmp);

   for (size_t i=0; i<(sizeof elements/sizeof elements[0] * 2); i++) {
      char *string =
         xsarray_insert (array, 
            elements[i % (sizeof elements/sizeof elements[0])]);
      printf ("Inserted %p : %s\n", string, string);
   }

   for (size_t i=0; i<xsarray_length(array); i++) {
      char *string = (char *)xsarray_index (array, i);
      printf ("Pos %zu: %p : %s\n", i, string, string);
   }
   char *kword = xsarray_find (array, "the");
   printf ("Found %p : %s\n", kword, kword);
   kword = xsarray_remove (array, kword);
   printf ("Removed %p : %s\n", kword, kword);
   printf ("===========================\n");
   for (size_t i=0; i<xsarray_length(array); i++) {
      char *string = (char *)xsarray_index (array, i);
      printf ("Pos %zu: %p : %s\n", i, string, string);
   }
   kword = xsarray_purge (array, kword);
   printf ("Purged %p : %s\n", kword, kword);
   printf ("===========================\n");
   for (size_t i=0; i<xsarray_length(array); i++) {
      char *string = (char *)xsarray_index (array, i);
      printf ("Pos %zu: %p : %s\n", i, string, string);
   }
   
   xsarray_free (array);
   return EXIT_SUCCESS;
}
