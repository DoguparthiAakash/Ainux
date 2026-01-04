#ifndef H_XCFG
#define H_XCFG    ("1.0.0")

#include <stdlib.h>
#include <string.h>

typedef struct xcfg_t xcfg_t;
struct xcfg_t {
   char *section;
   char *name;
   char *value;
   char *from;
   char *doc;
   char *defval;
};


#ifdef __cplusplus
extern "C" {
#endif

   int xcfg_configure (const char *section, const char *name, const char *doc, const char *defval);
   const char *xcfg_set (const char *section, const char *name, const char *value, const char *from);
   const char *xcfg_get (const char *section, const char *name);
   int xcfg_get_i (const char *section, const char *name);
   size_t xcfg_get_u (const char *section, const char *name);
   float xcfg_get_f (const char *section, const char *name);
   const char *xcfg_from_env (const char *name);
   xcfg_t **xcfg_get_all (void);
   void xcfg_shutdown (void);

   size_t xcfg_from_file (const char *filename);
   size_t xcfg_from_array (const char *section, const char **argv, const char *from);

   int xcfg_save_config (char *filename);


#ifdef __cplusplus
};
#endif      /* end of function prototypes */


#endif      /* end of header              */
