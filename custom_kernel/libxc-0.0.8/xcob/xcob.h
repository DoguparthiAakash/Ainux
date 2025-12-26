
#ifndef H_XCOB
#define H_XCOB ("1.0.0")

#include <stdarg.h>
#include <stdbool.h>
#include <stdlib.h>

typedef struct xcob_t xcob_t;

#ifdef __cplusplus
extern "C" {
#endif

   xcob_t *xcob_new (const char *name);
   const char *xcob_get_name (xcob_t *cob);
   bool xcob_set_name (xcob_t *cob, char *name);
   bool xcob_set (xcob_t *cob, char *name, char *value);
   char *xcob_get (xcob_t *cob, char *name);
   xcob_t *xcob_append (xcob_t *holding, xcob_t *member);
   xcob_t *xcob_inherit (char *name, ...);
   xcob_t *xcob_dup (xcob_t *src, const char *name);
   char *xcob_serialise (xcob_t *cob, size_t level);
   void xcob_del (xcob_t *cob);
   void xcob_del_real (xcob_t *obj);
   char **xcob_list_members (xcob_t *cob);
   char **xcob_list_data (xcob_t *cob);
   char **xcob_list_objects (xcob_t *cob);

#ifdef __cplusplus
};
#endif

#endif

