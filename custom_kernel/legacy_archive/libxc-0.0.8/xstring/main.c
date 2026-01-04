
#include <stdio.h>
#include <stdlib.h>
#include <string.h>


#include "xerror/xerror.h"
#include "xstring/xstring.h"

#define PARAGRAPH    (\
   "The quick brown fox jumped over the lazy \\ndog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n" \
   "The quick brown fox jumped over the lazy dog.\n")

int main (int argc, char **argv)
{

   printf ("Testing xstring version %s\n", H_XSTRING);
   {
      // Quickly test the copying of an array of strings and deletion
      // of the same.
      char **argv_cpy = xstr_cpyarray (argv);
      argc = argc;
      for (size_t i=0; argv_cpy[i]; i++) {
         printf ("%zu: '%s'\n", i, argv_cpy[i]);
      }
      xstr_delarray (argv_cpy);
   }
   char str1[] = "**************************";
   char *str2 = " Hello World";
   char *str3 = xstr_dup (str2);
   xstr_ncpy (str1, str2, sizeof str1);
   printf ("str1: %s\nstr2: %s\nstr3: %s\n", str1, str2, str3);
   free (str3);

   str3 = xstr_cat (str1, " two ", " three ", " four ", " five ", NULL);
   printf ("strcat: '%s'\n", str3);

   char *str4 = xstr_dup (str3);
   xstr_rtrim (str4);
   printf ("rtrim: '%s'\n", str4);
   free (str4);

   str4 = xstr_dup (str3);
   xstr_ltrim (str4);
   printf ("ltrim: '%s'\n", str4);
   free (str4);
   
   str4 = xstr_dup (str3);
   xstr_trim (str4);
   printf ("trim: '%s'\n", str4);
   free (str4);

   str4 = xstr_dup (str3);
   xstr_trim (xstr_trim (xstr_trim (str4)));
   printf ("rec-trim: '%s'\n", str4);
   free (str4);

   str4 = xstr_dup ("       ");
   xstr_rtrim (str4);
   printf ("only-space-trim: '%s'\n", str4);
   free (str4);

   str4 = xstr_dup ("");
   xstr_trim (str4);
   printf ("empty-trim: '%s'\n", str4);
   free (str4);

   free (str3);

   char *a_of_s[] = {"One ", "Two ", "Three ", "Four ", "Five ", NULL};
   str3 = xstr_join (a_of_s, 0);
   printf ("a_of_s = '%s'\n", str3);
   free (str3);

   str3 = xstr_join (a_of_s, ':');
   printf ("a_of_s = '%s'\n", str3);
   free (str3);

   str3 = xstr_readfile ("xstring.c");
   if (!str3) {
      XERROR ("Unable to read file xstring.c\n%m\n");
   } else {
      printf ("read in %zu bytes\n<--%s-->\n====\n", strlen (str3), str3);
      free (str3);
   }

   char *mf_string = ".this is.a:::::string\\,that?is'delimited..";
   char *delims = ".:,?'";

   char **result = xstr_split (mf_string, delims);
   // Print out each element of result, and then free it
   for (size_t i=0; result && result[i]; i++) {
      printf ("substr='%s'\n", result[i]);
      free (result[i]);
   }
   // Free the entire array
   if (result) free (result);

   char *commands[] = { "play", "pause", "restart", "stop", NULL, };
   char *input[] = {"play", "restart", "pause", "unknown", "stop", };
   for (size_t i=0; i<sizeof input/sizeof input[0]; i++) {
      switch (xstr_match_first_a (input[i], commands))
      {
         case 0: printf ("action for play\n");     break;
         case 1: printf ("action for pause\n");    break;
         case 2: printf ("action for restart\n");  break;
         case 3: printf ("action for stop\n");     break;
         case (size_t)-1:
         default:
            printf ("Unknown Action %s\n", input[i]); break;
            break;
         // Unknown input
      }
   }
   for (size_t i=0; i<sizeof input/sizeof input[0]; i++) {
      switch (xstr_match_first (input[i], "play", 
                                       "pause", 
                                       "restart", 
                                       "stop", 
                                       NULL))
      {
         case 0: printf ("action for play\n");     break;
         case 1: printf ("action for pause\n");    break;
         case 2: printf ("action for restart\n");  break;
         case 3: printf ("action for stop\n");     break;
         case (size_t)-1:
         default:
            printf ("Unknown Action %s\n", input[i]); break;
            break;
         // Unknown input
      }
   }
   char *paragraph = xstr_fmt (PARAGRAPH, 52);
   printf ("%s\n", paragraph);
   free (paragraph);
   char esc_test[] = 
      ",./;'[]\\<>?:\"{}|`1234567890-=~!@#$%^&*()_+\n"
      "This is a test string to escape. More characters follows.\n"
      ",./;'[]\\<>?:\"{}|`1234567890-=~!@#$%^&*()_+\n"
      "The quick brown fox Jumped over the Lazy Dog.\n"
      ",./;'[]\\<>?:\"{}|`1234567890-=~!@#$%^&*()_+\n";
   char *tmp =
      xstr_escape (esc_test, '\\',
                   ",./;'[]\\<>?:\"{}|`1234567890-=~!@#$%^&*()_+");
   printf ("escaping \n%s\n%s\n", esc_test, tmp); free (tmp);
   tmp =
   xstr_unescape (esc_test, '\\',
                  ",./;'[]\\<>?:\"{}|`1234567890-=~!@#$%^&*()_+");
   
   printf ("unescaping\n%s\n%s\n", esc_test, tmp); free (tmp);

   char *nvline = " name 1 = value 1 # This is a comment\n";
   char **nvpair = xstr_parse_nv (nvline, 0, '#');
   if (!nvpair) {
      printf ("nv_parse failed\n");
   } else {
      printf ("name: '%s'\nvalue: '%s'\n", nvpair[0], nvpair[1]);
      xstr_delarray (nvpair);
   }
    
   return EXIT_SUCCESS;
}
