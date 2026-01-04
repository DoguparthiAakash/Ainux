#include <stdio.h>
#include <stdlib.h>

#include "xshare/xshare.h"

static char *function (char * t, char * v)
{
   xshare_library_t handle = xshare_open ("./libdynamic");
   if (!handle) {
      printf ("dl1: %s\n", xshare_errmsg ());
      return NULL;
   }
   int (*myputs) (char *, char *) = (int (*) (char *, char *))
      xshare_symbol (handle, "testing");
   printf ("after %p\n", (void *)myputs);
   if (!myputs) {
      printf ("dl2: %s\n", xshare_errmsg ());
      return NULL;
   }
   printf (" in func '%s', with ptr %p\n", __PRETTY_FUNCTION__, (void *)myputs);
   int ret = myputs (t, v);
   printf ("returned: %i\n", ret);
   xshare_close (handle);
   return "done";
}

int main (int argc, char **argv)
{
   for (int i=0; i<argc; i++) {
      puts (argv[i]);
   }
   puts ("---------------------------------------------------------");
   char * t = "test-t";
   char * v = "test-v";
   puts ("---------------------------------------------------------");
   printf ("returned - '%s'\n", function (t, v));
   printf ("Hello World - cpp\n");
   return EXIT_SUCCESS;
}
