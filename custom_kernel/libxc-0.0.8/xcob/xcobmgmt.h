
#ifndef H_XCOBMGMT
#define H_XCOBMGMT ("1.0.0")

typedef struct xcobmgmt_t xcobmgmt_t;

#ifdef __cplusplus
extern "C" {
#endif

   xcobmgmt_t *xcobmgmt_startup (void);
   size_t xcobmgmt_install (xcobmgmt_t *xcm, xcob_t *obj);
   size_t xcobmgmt_read_string (xcobmgmt_t *xcm, const char *ins);
   size_t xcobmgmt_read_file (xcobmgmt_t *xcm, const char *filename);
   const xcob_t *xcobmgmt_find_by_id (xcobmgmt_t *xcm, size_t uuid);
   const xcob_t *xcobmgmt_find_by_name (xcobmgmt_t *xcm, char *name);
   void xcobmgmt_shutdown (xcobmgmt_t *xcm);


#ifdef __cplusplus
};
#endif

#endif

