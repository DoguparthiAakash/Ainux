
#include <stdio.h>
#include <stdlib.h>

#include "xcfg/xcfg.h"

int main (int argc, char **argv)
{
   argc = argc;
   int retval;
   printf ("Testing xcfg version %s\n", H_XCFG);
   if ((retval = (xcfg_configure
                ("main", "retries", "Number of retries to attempt", "4"))<0)) {
                   printf ("Error - %i\n", retval);
      return EXIT_FAILURE;
   }
   xcfg_set ("main", "retries", "9", __func__ );
   xcfg_set ("main", "retries", "12", __func__ );
   {
      xcfg_t **all = xcfg_get_all ();
      xcfg_t **tmp = all;
      while (all && *all) {
         printf ("name=%s\n", (*all)->name);
         printf ("value=%s\n", (*all)->value);
         printf ("doc=%s\n", (*all)->doc);
         printf ("from=%s\n", (*all)->from);
         printf ("default=%s\n", (*all)->defval);
         all++;
      }
      free (tmp);
   }
   retval = (xcfg_configure
      ("main", "retries", "Number of retries to attempt", "4"));
   if (retval<0) {
      printf ("Error - %i\n", retval);
      return EXIT_FAILURE;
   }
   if (!xcfg_set ("main", "retries", "2", "From User")) {
      printf ("Error in setting the config value 'retries'\n");
      return EXIT_FAILURE;
   }
   const char *tmp = xcfg_get ("main", "retries");
   if (!tmp) {
      printf ("Unable to get the config value for retries\n");
      return EXIT_FAILURE;
   }
   printf ("***retries=%i\n", xcfg_get_i ("main", "retries"));
   xcfg_from_env ("HOME");
   
   xcfg_from_file ("test.cfg");

   xcfg_from_array ("main", (const char **)argv, "from c/line");

   tmp = xcfg_get ("main", "retries");
   if (!tmp) {
      printf ("Unable to get the config value for retries\n");
      return EXIT_FAILURE;
   }
   printf ("Got value %s for retries\n", tmp);
   {
      xcfg_t **all = xcfg_get_all ();
      xcfg_t **tmp = all;
      while (all && *all) {
         printf ("name=%s\n", (*all)->name);
         printf ("value='%s'\n", (*all)->value);
         printf ("doc=%s\n", (*all)->doc);
         printf ("from=%s\n", (*all)->from);
         printf ("default=%s\n", (*all)->defval);
         printf ("==========================\n");
         all++;
      }
      free (tmp);
   }
   
   xcfg_save_config ("config.out");

   xcfg_shutdown ();
   return EXIT_SUCCESS;
}
