
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "xsparse/xsparse.h"
#include "xstring/xstring.h"

static void print_elem (size_t index, void *element)
{
   char *el = element;
   printf ("index %zu = \"%s\"\n", index, el);
}

int main (void)
{
   printf ("Testing xsparse %s\n", H_XSPARSE);
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

   // Iterate only on the used slots of the array
   xsparse_iterate (bigarray, print_elem);
   
   // Print then delete elements that were inserted
   for (size_t i=0; i<100; i++) {
      char *tmp = xsparse_get (bigarray, i);
      printf ("%zu : %s\n", i, tmp);
      free (tmp);
   }

   xsparse_del (bigarray);
   return EXIT_SUCCESS;
}
