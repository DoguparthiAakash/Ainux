
#include <string.h>

#include "xshare/xshare.h"
#include "dynamic.h"

int (*etest) (char *, char *);

int testing (char * one, char * two)
{
   printf ("entered testing, %s:%s\n\n", one, two);
   return strlen (one) + strlen (two);
}

SLIB_INIT_FUNC (SLIB_INIT_FUNC_ARGS);
SLIB_INIT_FUNC (SLIB_INIT_FUNC_ARGS) 
{
   printf ("*************** initing\n");
   printf ("function at %p\n", (void *) testing);
   printf ("pointer to function at %p\n", etest);
   etest = (int (*) (char *, char *)) testing;
   printf ("pointer to function at %p\n", etest);
   printf ("done ******************\n");
}

