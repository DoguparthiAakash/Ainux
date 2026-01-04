#include <stdio.h>
#include <stdlib.h>


#include "xsock/xsock.h"

int main (int argc, char **argv)
{
   if (argc>1) {
      return xsock_test (true);
   } else {
      return xsock_test (false);
   }
   return EXIT_SUCCESS;
}

