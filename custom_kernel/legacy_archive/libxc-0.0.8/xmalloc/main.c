#include <stdio.h>
#include <stdlib.h>
#include <time.h>


#include "xmalloc/xmalloc.h"

#define NUM_ALLOCS         (1024 * 32)

#define PRINT_SEPARATOR          \
   fprintf (stderr, "=============================================\n");
/* Note: the %m format is gnu-specific. Remove it for compatibility
 * with a different compiler.
 */

int test (size_t inc_amount) 
{
   time_t start = time (NULL);
   static int *values[NUM_ALLOCS];
   xmalloc_set_knob (XMALLOC_KNOB_INCREMENT, inc_amount);
   if (start==(time_t)-1) {
      fprintf (stderr, "Unable to get clock value, aborting\n");
      fprintf (stderr, "%m\n");
      return EXIT_FAILURE;
   }
   fprintf (stderr, "Starting test with %zu at time %zu\n",
         inc_amount,
         (size_t)start);

   start = time (NULL);
   {
      for (size_t i=0; i<sizeof values / sizeof values[0]; i++) {
         values[i] = XMALLOC (sizeof *values[i]);
         if (!values[i]) {
            fprintf (stderr, "Allocation failed, aborting\n");
            return EXIT_FAILURE;
         }
      }
   }
   time_t end = time (NULL);
   PRINT_SEPARATOR;
   fprintf (stderr, "Allocation time: %zus\n", (size_t) (end - start));
   PRINT_SEPARATOR;


   start = time (NULL);
   {
      for (size_t i=0; i<sizeof values / sizeof values[0]; i++) {
         *(values[i]) = i;
      }
   }
   end = time (NULL);
   PRINT_SEPARATOR;
   fprintf (stderr, "Access time: %zus\n", (size_t) (end - start));
   PRINT_SEPARATOR;

   // The reason that searching is important is because xmalloc searches
   // for an empty slot in its list of allocations before expanding the
   // array used to store the blocklist.
   start = time (NULL);
   {
      for (size_t i=0; i<sizeof values / sizeof values[0]; i++) {
         void *tmp = XMALLOC_REALLOC (values[i], sizeof (int) * 2);
         if (!tmp) {
            fprintf (stderr, "Unable to realloc %zu element\n", i);
            return EXIT_FAILURE;
         }
         values[i] = tmp;
      }
   }
   end = time (NULL);
   PRINT_SEPARATOR;
   fprintf (stderr, "Search time: %zus\n", (size_t) (end - start));
   PRINT_SEPARATOR;

   xmalloc_dump (XMALLOC_DUMP_STATS);
   PRINT_SEPARATOR;
   xmalloc_dump (XMALLOC_DUMP_SELF);
   start = time (NULL);
   {
      for (size_t i=0; i<sizeof values / sizeof values[0]; i++) {
         XMALLOC_FREE (values[i]);
      }
   }
   end = time (NULL);
   PRINT_SEPARATOR;
   fprintf (stderr, "Free time: %zus\n", (size_t) (end - start));
   PRINT_SEPARATOR;
   xmalloc_reset ();
   return EXIT_SUCCESS;
}

int main (void)
{
   return test (4096);
}

