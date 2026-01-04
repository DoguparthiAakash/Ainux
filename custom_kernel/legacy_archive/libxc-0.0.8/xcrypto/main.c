#include <stdio.h>
#include <stdlib.h>


#include "xcrypto/xcrypto.h"

int main (void)
{
   printf ("Testing xcrypto version %s\n", H_XCRYPTO);
   char *msg = "Hello World";
   char *hash = xcrypto_hash ((unsigned char *)msg, strlen (msg));
   printf ("%s\n%s\n", msg, hash);
   memset (hash, 0, 65);
   xcrypto_random ((unsigned char *)hash, 65);
   for (size_t i=0; i<65; i++) {
      printf ("%zu: %02x : %c\n",i, (uint8_t)(hash[i]), (uint8_t)(hash[i]));
   }
   printf ("\n");
   free (hash);
   return EXIT_SUCCESS;
}

