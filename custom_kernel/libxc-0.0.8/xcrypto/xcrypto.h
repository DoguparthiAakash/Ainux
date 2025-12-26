#ifndef H_XCRYPTO
#define H_XCRYPTO    ("1.0.0")

#include <stdlib.h>
#include <string.h>
#include <inttypes.h>


#ifdef __cplusplus
extern "C" {
#endif
   
   size_t xcrypto_random (uint8_t *buffer, size_t num_bytes);
   char *xcrypto_hash (const uint8_t *buffer, size_t num_bytes);

#ifdef __cplusplus
};
#endif      /* end of function prototypes */


#endif      /* end of header              */
