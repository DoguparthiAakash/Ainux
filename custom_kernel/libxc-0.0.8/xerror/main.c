
#include <stdio.h>
#include <stdlib.h>

#include "xerror/xerror.h"

int main (void)
{
   char *sa = " String Arg ";
   int ia = 42;
   float fa = 54.149;

   XERROR ("Trying a macro with %s, check if %i works\nAnd %f\n", 
      sa, ia, fa);

   xerror_set_logfile ("output.log");
   XLOG ("1. Checking if output logging works with %s, %i and %f\n",
      sa, ia, fa);

   xerror_set_logfile (NULL);
   XLOG ("2. Checking if output logging works with %s, %i and %f\n",
      sa, ia, fa);

   return EXIT_SUCCESS;
}
