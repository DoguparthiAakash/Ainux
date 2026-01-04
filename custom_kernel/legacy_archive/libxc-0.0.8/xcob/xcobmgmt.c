/**
 * \file 
 *
 * \brief Named-Object Manager
 *
 * \a xcob is a facility for providing limited OO functionality within the
 * constraints of the C language. See \a xcob_new() and related functions for
 * more information.
 *
 * \a Xcobmgmt is a manager for xcob objects. It allows the caller to 
 * register every new object with the manager, provides a unique ID (to the
 * specific instance of an \a xcobmgmt_t object), performs lookups on these
 * objects using either the object name or the unique ID and returns the
 * object being looked up, and, when \a xcobmgmt_shutdown() is called, 
 * destroys all objects registered with a specific instance of a manager.
 *
 * Other functionality provided is the ability to serialise an object to a 
 * plain-text human-readable form as well as to read these serialised forms
 * back into the manager instance from a file, or from C style strings.
 *
 * In addition, \a xcobmgmt provides enhancement to these serialisations
 * over and above the serialisations performed by \a xcob, such as the 
 * ability to include other files and recognition of comments within files.
 *
 * \a xcobmgmt is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 *
 */


#include <stdio.h>
#include <string.h>

#include "xmap/xmap.h"
#include "xerror/xerror.h"
#include "xstring/xstring.h"

#include "xcob/xcob.h"
#include "xcob/xcobmgmt.h"

struct objs_t {
   xcob_t *cob;
   size_t refcount;
};

typedef struct objs_t objs_t;

struct name_id_pair_t {
   const char *name;
   size_t uuid;
};

typedef struct name_id_pair_t name_id_pair_t;

struct xcobmgmt_t {
   xmap_t *objs;
   size_t uuid;
   bool dirty;
   name_id_pair_t **name_id_pair;
};

static xcob_t *cpyfunc (void *cob)
{
   if (!cob) return NULL;
   xcob_t *ret = xcob_dup (cob, xcob_get_name (cob));
   if (!ret) {
      XLOG ("Unable to duplicate xcob %s\n", xcob_get_name (cob));
   }
   return ret;
}

static void freefunc (void *cob)
{
   if (!(xcob_t*)cob) return;
   xcob_del (cob);
}

xcobmgmt_t *xcobmgmt_startup (void)
{
   xcobmgmt_t *ret = malloc (sizeof *ret);
   if (!ret) {
      XLOG ("Out of memory error\n");
      return NULL;
   }
   ret->objs = xmap_new ((void *(*) (void *))cpyfunc, freefunc);
   if (!ret->objs) {
      return NULL;
   }
   ret->uuid = 1;
   ret->dirty = true;
   ret->name_id_pair = NULL;
   return ret;
}

size_t xcobmgmt_install (xcobmgmt_t *xcm, xcob_t *obj)
{
   if (!xmap_insert (xcm->objs, xcm->uuid, obj)) {
      XLOG ("Unable to insert object %s\n", xcob_get_name (obj));
      return (size_t)-1;
   }
   xcm->dirty = true;
   return xcm->uuid++;
}

/* TODO: This function is still outstanding */
size_t xcobmgmt_read_string (xcobmgmt_t *xcm, const char *ins)
{
   if (!ins) goto errorexit;

errorexit:
   return (size_t)-1;
}

static char *local_readfile (const char *filename)
{
   char *input = xstr_readfile (filename);
   if (!input) {
      XLOG ("Could not read file %s\n", filename);
      goto errorexit;
   }
   char *tmp = input;
   while (tmp && *tmp) {
      if (*tmp=='#') {
         while (*tmp && *(tmp+1) && *(tmp+1)!='\n') {
            *tmp++ = ' ';
         }
         *tmp = ' ';
      }
      tmp++;
   }
   printf ("********\n%s\n********\n", input);

   return (input);
errorexit:
   return NULL;
}

size_t xcobmgmt_read_file (xcobmgmt_t *xcm, const char *filename)
{
   size_t ret = 0;
   char *input = local_readfile (filename);
   if (!input) {
      XLOG ("Could not read file %s\n", filename);
      goto errorexit;
   }
   ret = xcobmgmt_read_string (xcm, input);
   free (input);
   return ret;

errorexit:
   return (size_t)-1;
}

const xcob_t *xcobmgmt_find_by_id (xcobmgmt_t *xcm, size_t uuid)
{
   return xmap_find (xcm->objs, uuid);
}

static void *get_name_id (size_t k, void **v)
{
   name_id_pair_t *ret = malloc (sizeof *ret);
   if (!ret) {
      XLOG ("Unable to allocate name/id pair structure for object %s\n",
            (char *)v);
      return NULL;
   }
   xcob_t *tmp = *(xcob_t **)v;
   ret->uuid = k;
   ret->name = xcob_get_name (tmp);
   printf ("cob name = %s\n", ret->name);
   return ret;
}

const xcob_t *xcobmgmt_find_by_name (xcobmgmt_t *xcm, char *name)
{
   if (xcm->dirty) {
      void **names = xmap_map (xcm->objs, get_name_id);
      if (!names) {
         XLOG ("Unable to get list of object names - are any "
               "objects installed?\n");
         return NULL;
      }
      if (xcm->name_id_pair) {
         for (size_t i=0; xcm->name_id_pair[i]!=NULL; i++) {
            free (xcm->name_id_pair[i]);
         }
         free (xcm->name_id_pair);
      }
      xcm->name_id_pair = (name_id_pair_t **)names;
      xcm->dirty = false;
      printf ("Rebuilt index\n");
   }
   for (size_t i=0; xcm->name_id_pair[i]; i++) {
      if (strcmp (name, xcm->name_id_pair[i]->name)==0) {
         return xcobmgmt_find_by_id (xcm, xcm->name_id_pair[i]->uuid);
      }
   }
   return NULL;
}

static void *freeobj (size_t k, void **v)
{
   xcob_del (*(xcob_t **)v);
   *v = NULL;
   return NULL;
}

void xcobmgmt_shutdown (xcobmgmt_t *xcm)
{
   if (xcm) {
      if (xcm->objs) {
         void **tmp = xmap_map (xcm->objs, freeobj);
         if (tmp) free (tmp);
         xmap_del (xcm->objs);
      }
      if (xcm->name_id_pair) {
         for (size_t i=0; xcm->name_id_pair[i]; i++) {
            free (xcm->name_id_pair[i]);
         }
         free (xcm->name_id_pair);
      }
      free (xcm);
   }
}


