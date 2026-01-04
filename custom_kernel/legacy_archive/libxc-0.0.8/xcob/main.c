
#include <stdio.h>
#include <stdlib.h>
#include <string.h>


#include "xerror/xerror.h"
#include "xstring/xstring.h"
#include "xcob/xcob.h"
#include "xcob/xcobmgmt.h"

static void xcobmgmt_test (void)
{
   xcobmgmt_t *mgr = xcobmgmt_startup ();
   xcob_t *c1 = xcob_new ("c1");
   xcob_t *c2 = xcob_new ("c2");
   xcob_t *c3;

   struct {
      char *name, *value;
   } samples2 [] = {
      {"ONE", "ONE-VALUE"}, 
      {"TWO", "TWO-VALUE"}, 
      {"THREE", "THREE-VALUE"}, 
      {"FOUR", "FOUR-VALUE"}, 
   };
   struct {
      char *name, *value;
   } samples1 [] = {
      {"one", "one-value"}, 
      {"two", "two-value"}, 
      {"three", "three-value"}, 
      {"four", "four-value"}, 
   };

   for (size_t i=0; i<sizeof samples2/sizeof samples2[0]; i++) {
      xcob_set (c2, samples2[i].name, samples2[i].value);
   }

   for (size_t i=0; i<sizeof samples1/sizeof samples1[0]; i++) {
      xcob_set (c1, samples1[i].name, samples1[i].value);
   }

   c3 = xcob_dup (c1, "c3");
   if (!c3) {
      XLOG ("Unable to clone cob1\n");
   }
   if (!xcob_append (c3, c2)) {
      printf ("Error appending c2 to c3\n");
   }

   size_t sc1, sc2, sc3;
   if ((sc3 = xcobmgmt_install (mgr, c3))==(size_t)-1) {
      printf ("Installation c3 failed\n");
   }
   if ((sc2 = xcobmgmt_install (mgr, c2))==(size_t)-1) {
      printf ("Installation c2 failed\n");
   }
   xcob_t *tmp = xcobmgmt_find_by_name (mgr, "c2");
   if (!tmp) {
      printf ("Could not find object name=ONE\n");
   } else {
      char *tmps = xcob_serialise (tmp, 5);
      if (tmps) {
         printf ("Found object below\n%s\n", tmps);
         free (tmps);
      } else {
         printf ("Could not serialise object %p\n", tmp);
      }
   }
   if ((sc1 = xcobmgmt_install (mgr, c1))==(size_t)-1) {
      printf ("Installation c1 failed\n");
   }
   printf ("==================================\n");
   printf ("sc1 = %zu\nsc2 = %zu\nsc3 = %zu\n", sc1, sc2, sc3);

   tmp = xcobmgmt_find_by_id (mgr, 2);
   if (!tmp) {
      printf ("Could not find object uuid=2\n");
   } else {
      char *tmps = xcob_serialise (tmp, 5);
      if (tmps) {
         printf ("3 - Found object below\n%s\n", tmps);
         free (tmps);
      } else {
         printf ("Could not serialise object %p\n", tmp);
      }
   }

   tmp = xcobmgmt_find_by_name (mgr, "c1");
   if (!tmp) {
      printf ("Could not find object name=ONE\n");
   } else {
      char *tmps = xcob_serialise (tmp, 5);
      if (tmps) {
         printf ("Found object below\n%s\n", tmps);
         free (tmps);
      } else {
         printf ("Could not serialise object %p\n", tmp);
      }
   }

   char *string = xcob_serialise (c3, 1);
   if (string) {
      printf ("serialised:\n%s\n", string);
      free (string);
   }

   xcob_del (c1);
   xcob_del (c3);
   if ((size_t)-1!=xcobmgmt_read_file (mgr, "example.cob")) {
      printf ("Could not read c-object file\n");
   }
   xcobmgmt_shutdown (mgr);
}

int main (void)
{
   xcob_t *cob1 = xcob_new ("OBJ-1");
   xcob_t *cob2 = xcob_new ("obj-two\n");
   struct {
      char *name, *value;
   } samples2 [] = {
      {"ONE", "ONE-VALUE"}, 
      {"TWO", "TWO-VALUE"}, 
      {"THREE", "THREE-VALUE"}, 
      {"FOUR", "FOUR-VALUE"}, 
   };
   struct {
      char *name, *value;
   } samples1 [] = {
      {"one", "one-value"}, 
      {"two", "two-value"}, 
      {"three", "three-value"}, 
      {"four", "four-value"}, 
   };

   for (size_t i=0; i<sizeof samples2/sizeof samples2[0]; i++) {
      xcob_set (cob2, samples2[i].name, samples2[i].value);
   }

   for (size_t i=0; i<sizeof samples1/sizeof samples1[0]; i++) {
      xcob_set (cob1, samples1[i].name, samples1[i].value);
   }

   for (size_t i=sizeof samples1/sizeof samples1[0]; i>0; i--) {
      char *tmp = xcob_get (cob1, samples1[i-1].name);
      printf ("Found member '%s' with value '%s'\n", samples1[i-1].name, tmp);
   }

   char *string = xcob_serialise (cob1, 1);
   if (string) {
      printf ("serialised:\n%s\n", string);
      free (string);
   }

   if (!xcob_append (cob1, cob2)) {
      XLOG ("Error appending child to parent\n");
   }

   xcob_t *cob3 = xcob_dup (cob1, "cob3");
   if (!cob3) {
      XLOG ("Unable to clone cob1\n");
   }

   string = xcob_serialise (cob1, 0);
   if (string) {
      printf ("serialised:\n%s\n", string);
      free (string);
   }

   string = xcob_serialise (cob3, 0);
   if (string) {
      printf ("serialised:\n%s\n", string);
      free (string);
   }

   xcob_del (cob3);
   xcob_t *cob4 = xcob_new ("cob4");
   cob3 = xcob_dup (cob4, "cob3-from-4");
   string = xcob_serialise (cob4, 0);
   if (string) {
      printf ("serialised:\n%s\n", string);
      free (string);
   }
   xcob_t *cob5 = xcob_inherit ("cob5", cob4, cob3, cob2, cob1, NULL);
   string = xcob_serialise (cob5, 3);
   if (string) {
      printf ("serialised:\n%s\n", string);
      free (string);
   }
   xcob_t *cob6 = xcob_inherit ("cob6", cob5, NULL);
   string = xcob_serialise (cob6, 4);
   if (string) {
      printf ("serialised:\n%s\n", string);
      free (string);
   }

   xcob_del (cob6);
   xcob_del (cob5);
   xcob_del (cob4);
   xcob_del (cob3);

   // cob2 was appended to cob1, and will get deleted with cob1
   xcob_del (cob1);

   xcobmgmt_test ();
   return EXIT_SUCCESS;
}
