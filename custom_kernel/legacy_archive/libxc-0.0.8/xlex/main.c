
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <regex.h>

#include "xerror/xerror.h"
#include "xlex/xlex.h"

#define SYMBOL       "[A-Za-z][A-Za-z0-9]*"
//#define ALL          "[A-Za-z0-9&~!@#$%^\\*()-_+{}|\\\\;':,.?/ \t]*"
#define ALL          "[A-Za-z0-9 \t]*"
//#define ALL          "[A-Za-z0-9 \t]*"
#define NVPAIR       "([[:space:]]*" SYMBOL "=\"" ALL "\"[[:space:]]*)"
#define NSTRING       "(\"" ALL "\")"
#define ESTRING      "(\"" ALL "\\\"" ALL "\\\"" ALL "\")"
#define STRING       NSTRING "|" ESTRING

static bool in_attrs = false;
static bool in_content = false;

static void newtree_sa (char *match, void *ea, size_t line, size_t offset)
{
   line = line; offset = offset;
   printf ("newtree_sa" "(%s ...", &match[1]);
   in_attrs = true;
}

static void newtree_sc (char *match, void *ea, size_t line, size_t offset)
{
   line = line; offset = offset; in_content = true;
   char *tmp = strrchr (match, '>');
   if (tmp) *tmp = 0;
   printf ("newtree_sc" "2(%s ...", &match[1]);
}

static void endtree (char *match, void *ea, size_t line, size_t offset)
{
   line = line; offset = offset; match = match;
   in_attrs = false; in_content = false;
   printf ("endtree" "3)\n");
}

static void findstring (char *match, void *ea, size_t line, size_t offset)
{
   printf ("findstring" "5--%s--\n", match);
}

static void everything (char *match, void *ea, size_t line, size_t offset)
{
   line = line; offset = offset; match = match;
   printf ("everything" "4--%s--\n", match);
}

static void add_attrs (char *match, void *ea, size_t line, size_t offset)
{
   //if (!in_attrs) return;
   line = line; offset = offset;
   char *name = match, *value = strchr (match, '=');
   if (value) {
      *value++ = 0; value++;
      char *tmp = strchr (value, '"');
      if (tmp) *tmp = 0;
   } else {
      value = "";
   }
   printf ("add_attrs" "5:%s '%s' ", name, value);
}

xlex_t *nvs;
static void tagmatch (char *match, void *ea, size_t line, size_t offset)
{
   line = line; offset = offset; match = match;
   printf ("tagmatch" "10(%s ", &match[1]);
}

static void nvmatch (char *match, void *ea, size_t line, size_t offset)
{
   line = line; offset = offset; match = match;
   char *name = match, *value = strchr (match, '=');
   if (value) {
      *value++ = 0; value++;
      char *tmp = strchr (value, '"');
      if (tmp) *tmp = 0;
   } else {
      value = "";
   }
   printf ("nvmatch" "5:%s '%s' ", name, value);
}

static void endmatch (char *match, void *ea, size_t line, size_t offset)
{
   line = line; offset = offset; match = match;
   in_attrs = false; in_content = false;
   printf ("endmatch" "3)\n");
}


static void match (char *match, void *ea, size_t line, size_t offset)
{
   line = line; offset = offset; match = match;
   xlex_exec (nvs, match, line, NULL);
}

int main (int argc, char **argv)
{
      size_t line_num = 1;
      // Create a new lexical scanner
      xlex_t *ls = xlex_new ("main lexical scanner");
      nvs = xlex_new ("nvpair scanner");
      xlex_add (nvs, "<" SYMBOL, tagmatch);
      xlex_add (nvs, NVPAIR, nvmatch);
      xlex_add (nvs, ">", endmatch);
      
#if 1
      printf ("%s\n", NVPAIR);
      // Match opening tag
      xlex_add (ls, "<" SYMBOL NVPAIR "*>", match);
      // Match closing tag
      xlex_add (ls, "</" SYMBOL ">", endmatch);
      xlex_add (ls, STRING, findstring);
      // Match everything else
       xlex_add (ls, ALL, everything);
#endif

      // Execute the scanner on our input
      while (!feof (stdin) && !ferror (stdin)) {
         static char input[4096];
         if (!fgets (input, sizeof input -1, stdin)) break;
         xlex_exec (ls, input, line_num++, NULL);
      }

      xlex_del (ls);
      xlex_del (nvs);
#if 0
   xlex_t *xltest = xlex_new ("testing");
   if (!xltest) {
      XERROR ("xltest could not be created\n");
   } else {
      XLOG ("xltest created: %p\n", xltest);
   }

   xlex_add (xltest, "\".*\"", mstring);
   xlex_add (xltest, "[0-9]+\\.[0-9]+", mfloat);
   xlex_add (xltest, "[0-9]+", mint);

   xlex_exec (xltest, "0.9 1234 \"testing a long string to match\"", 1);

   xlex_del (xltest);

   printf ("Starting dynamic lexical scanning tests (v%s)\n", H_XLEX);
   regex_t re;
   int re_error;
   
   if ((re_error = regcomp (&re, "\".+\"", REG_EXTENDED))!=0) {
      static char errmsg[4096];
      regerror (re_error, &re, errmsg, sizeof errmsg);
      printf ("Error %i: %s\n", re_error, errmsg);
      regfree (&re);
      return EXIT_FAILURE;
   }
   while (!feof (stdin) && !ferror (stdin)) {
      regmatch_t marray[200];
      static char line[4096];
      memset (line, 0, sizeof line);
      if (fgets (line, sizeof line -1, stdin)==NULL) {
         fprintf (stderr, "Error encountered %m, aborting\n");
         break;
      }
      if (regexec (&re, line, sizeof marray/sizeof marray[0], marray, 0)) {
         fprintf (stderr, "Not matched %m\n");
         break;
      }
      for (size_t i=0; i<sizeof marray/sizeof marray[0]; i++) {
         static char match[4096];
         if (marray[i].rm_so>=0) {
            memset (match, 0, sizeof match);
            memcpy (match, &line[marray[i].rm_so],
                           marray[i].rm_eo - marray[i].rm_so);
            printf ("matched: %s : %i/%i\n", match,
                     marray[i].rm_so, marray[i].rm_eo);
         }
      }
   }
   regfree (&re);
#endif
   return EXIT_SUCCESS;
}
