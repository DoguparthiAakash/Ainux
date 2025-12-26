#ifndef H_XMALLOC
#define H_XMALLOC    ("1.0.2")

#include <stdlib.h>
#include <string.h>

/* xmalloc can dump its internal structures to stderr. On detection of an
 * error, the dump will occur automatically (see below).
 * The following are the only recognisable flags that can be specified
 * in a dump to stderr.
 */
// Dump aggregate statistics only
#define XMALLOC_DUMP_STATS       (0x01)            
// Dump the blocklist in delimited form
#define XMALLOC_DUMP_BLOCKS      (0x01 << 0x01)
// Dump statistics about xmalloc itself
#define XMALLOC_DUMP_SELF        (0x01 << 0x02)
// Dump everything
#define XMALLOC_DUMP_ALL         (XMALLOC_DUMP_STATS | \
                                  XMALLOC_DUMP_BLOCKS | \
                                  XMALLOC_DUMP_SELF)

/* xmalloc behaviour can be fine-tuned by setting values to certain
 * knobs. These are the only recognisable knobs.
 */
// The default flags that gets used in dumping when an error is detected
#define XMALLOC_KNOB_FLAGS       (1)
// The amount to increment each reallocation by
#define XMALLOC_KNOB_INCREMENT   (2)
// This knob can be used to turn off data collection. xmalloc will then
// function as a thin wrapper to malloc.
#define XMALLOC_KNOB_ENABLE     (3)

// These macros shold be used with the functions - they ensure that
// the point of call (file, function and line_number) are recorded.
#define XMALLOC(s)               \
   xmalloc (__FILE__, __func__, __LINE__, s)
#define XMALLOC_REALLOC(p,s)               \
   xmalloc_realloc (__FILE__, __func__, __LINE__, p, s)
#define XMALLOC_FREE(p)          \
   xmalloc_free (p);
#define XMALLOC_DUMP(flags)      \
   xmalloc_dump (flags)

#ifdef __cplusplus
extern "C" {
#endif

   /* These three should only be used via the macros above */
   void *xmalloc (const char *file, const char *func, size_t line, size_t size);
   void xmalloc_free (void *ptr);
   void *xmalloc_realloc (const char *file, const char *func, 
                          size_t line, void *ptr, size_t newsize);
   /* Dump certain statistics (specified with 'flags') to stderr */
   void xmalloc_dump (unsigned char flags);
   /* Fine-tune the behaviour of xmalloc */
   void xmalloc_set_knob (int knob, size_t value);
   /* Search for an allocated pointer */
   void *xmalloc_find (void *);
   /* Reset xmalloc, freeing all resources used by xmalloc internally.
    * The memory allocated to the caller is still available.
    */
   void xmalloc_reset (void);

#ifdef __cplusplus
};
#endif      /* end of function prototypes */


#endif      /* end of header              */
