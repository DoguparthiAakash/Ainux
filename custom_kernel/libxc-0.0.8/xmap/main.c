#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "xmap/xmap.h"
#include "xstring/xstring.h"

static void *print1 (size_t k, void **v)
{
   printf ("print1: %zu:%s\n", k, *(char **)v);
   return NULL;
}

static void *print2 (size_t k, void **v)
{
   printf ("print1: %zu:%zu\n", k, **(size_t **)v);
   return NULL;
}

static size_t expired_since (void)
{
   static clock_t mc = 0;
   if (mc==0) mc = clock ();
   return (size_t)(clock() - mc);
}

int main (void)
{
   printf ("Testing xmap version %s:%zu\n", H_XMAP, clock());
   xmap_t *xmap1 = xmap_new ((void *(*) (void *))xstr_dup, free);
   xmap_insert (xmap1, 15, "fifteen");
   xmap_insert (xmap1, 1, "one");
   xmap_insert (xmap1, 2, "two");
   xmap_insert (xmap1, 2, "twod");
   xmap_insert (xmap1, 3, "three");
   xmap_insert (xmap1, 100, "one hundred");
   xmap_insert (xmap1, 200, "two hundred");
   xmap_replace (xmap1, 1, "oned");
   xmap_replace (xmap1, 300, "three hundred");
   char *tmps = xmap_find (xmap1, 15);
   if (tmps) {
      printf ("Found item '%s'\n", tmps);
   } else {
      printf ("Could not find value for 15\n");
   }
   xmap_remove (xmap1, 15);

   void ** tmp = xmap_map (xmap1, print1);
   free (tmp);

   size_t *keys = xmap_getkeys (xmap1);
   for (size_t i=0; keys[i]; i++) {
      printf ("Got value '%s' for key %zu\n",
                                          (char *)xmap_find (xmap1, keys[i]),
                                          keys[i]);
   }

   free (keys);

   for (size_t i=0; i<1024; i++) {
      static char buf[1024];
      sprintf (buf, "string-%zu='%zu'", i, i);
      xmap_insert (xmap1, i, buf);
   }

   xmap_t *xmap2 = xmap_new (NULL, NULL);

   static size_t num_array[1024];
   for (size_t i=0; i<sizeof num_array/sizeof num_array[0]; i++) {
      num_array[i] = 8192 - i;
   }

   printf ("Before Insertion: %zu\n", clock());
   for (size_t i=0; i<1024 * 1; i++) {
      xmap_insert (xmap2, i, &num_array[i % sizeof num_array/sizeof num_array[0]]);
   }
   printf ("After insertion: %zu\n", clock());

   // void **nums = xmap_map (xmap2, print2);
   // free (nums);

   printf ("Before finding: %zu\n", clock());
   printf ("Sought and found %p\n", xmap_find (xmap2, (size_t)-1));
   printf ("After finding: %zu\n", clock());
#if 0
   while (!feof (stdin) && !ferror (stdin)) {
      printf ("Enter a number\n");
      size_t num;
      scanf ("%zu", &num);
      printf ("Found item %zu - %s\n", num, xmap_find (xmap1, num));
   }
#endif
   xmap_del (xmap1);
   xmap_del (xmap2);
   printf ("Before counting: %zu\n", clock());
   //for (size_t i=0; i<1024 * 1024 * 1024 * 1024 * 1024; i++) ;
   printf ("After counting: %zu\n", clock());
   printf ("Program ending\n%zu\n", clock ());
   return EXIT_SUCCESS;
}

