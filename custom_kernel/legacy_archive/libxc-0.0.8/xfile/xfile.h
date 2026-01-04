#ifndef H_XFILE
#define H_XFILE    ("1.0.0")

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <time.h>
#include <inttypes.h>

#include "xvector/xvector.h"

#define  STRINGIFY(x)            #x
#define  STRINGIFY_VALUE(x)      STRINGIFY(x)
#define  DIRSEP                  STRINGIFY_VALUE(DS)


enum xfile_type_t {
   stat_REG = 1,
   stat_DIR,
   stat_OTHER,
};
typedef enum xfile_type_t xfile_type_t;

struct xfile_stat_t {
   xfile_type_t type;
   uint64_t size;
   size_t link_count;
   time_t last_access;
   time_t last_modified;
   char *name;
};

typedef struct xfile_stat_t xfile_stat_t;

#ifdef __cplusplus
extern "C" {
#endif

   char *xfile_tmpname (const char *dirname);
   
   FILE *xfile_atomic (const char *dirname, const char *filename);

   uint64_t xfile_copy (const char *source, const char *dest, 
                        size_t bufsize, int (*progress) (uint64_t));
   
   FILE *xfile_open_with_backup (const char *filename, const char *ext);

   // char **xfile_list (const char *dirname);

   xfile_stat_t *xfile_stat (const char *pathname);

   bool xfile_readable (const char *filename);
   bool xfile_writable (const char *filename);

   bool xfile_mkdir (char *path, char *flags);

   size_t xfile_rm (char *pathname, char *flags);

   xvector_t *xfile_find (const char *pathname,
                          const char *xregex,
                          const char *flags);

#ifdef __cplusplus
};
#endif  


#endif 
