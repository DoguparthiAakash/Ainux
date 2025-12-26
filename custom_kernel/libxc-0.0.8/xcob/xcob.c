/**
 * \file 
 *
 * \brief Named-Object functionality for C
 *
 * \a xcob is a facility for providing limited OO functionality within the
 * constraints of the C language. Object creation, naming, setting of
 * member values, getting of member values and composition of more
 * objects are supported. The object model includes facilities for both
 * HAS-A relationships and IS-A relationships. This is discussed in more
 * detail below.
 *
 *
 * IS-A RELATIONSHIPS
 *
 * Multiple inheritance is supported via a precedence-determining 
 * method for resolving redundant member names from parent objects
 * (see \a xcob_inherit() below for more information). All data members
 * as well as the member names are null-terminated strings.
 *
 * \a xcob follows a prototype object model (much like javascript) rather
 * than the usual imperative language class-based model (C++, Java, etc).
 * Objects are created with no members (unless inherited) and name/value
 * pairs representing the member name and member data are then added to the
 * object. The object can then be used as is or as a parent class for
 * child objects which inherit from it.
 *
 * Object inheritance is performed via duplicating the parent object(s)
 * using a deep-copying mechanism; in this way it is safe to modify
 * any members present in children objects as every object's members
 * are separate and distinct from the parent object.
 *
 *
 * HAS-A RELATIONSHIPS
 *
 * Objects may contain other objects; this is not implemented via 
 * deep-copying and it is recommended that a duplicate of the object to be
 * included into another object be first created with
 * \a xcob_dup before the caller invokes \a xcob_append. Member objects which
 * are included in other objects using \a xcob_append will be free()ed
 * automatically by the holding object.
 *
 *
 * MEMBER FUNCTIONS
 *
 * At time of writing \a xcob does not include any facilities for 
 * creating data members, although this option is being explored for 
 * future releases.
 *
 *
 * \sa xcob_new(), xcob_set(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_serialise(), xcob_del(), xcob_list_members(),
 * xcob_list_members(), xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * \a xcob is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 *
 */

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#include "xstring/xstring.h"
#include "xvector/xvector.h"
#include "xdict/xdict.h"
#include "xerror/xerror.h"
#include "xcob/xcob.h"

struct xcob_t {
   char *name;
   xdict_t *pods;
   xvector_t *objs;
};

/**
 * \brief Create a new named object.
 *
 * Creates a new empty named object, \as xcob_t, with name \a name. Object
 * that is created contains no data members nor other objects.
 * Caller is responsible for freeing the returned object using \a xcob_del().
 *
 * \sa xcob_set(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_serialise(), xcob_del(), xcob_list_objects(),
 * xcob_list_members(), xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] name The name to give this object. The library does not
 *    track names therefore this name is only of interest to the caller
 *    and thus does not have to be unique.
 *
 * @return On success, a freshly allocated named object with the name \a name
 * is returned. On failure \a NULL is returned. The caller must free 
 * the returned object using \a xcob_del().
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a data member
      xcob_set (cob1, "dataMember1", "Value-1");
      // Get a data member 
      printf ("Member value is %s\n", xcob_get (cob1, "dataMember1"));
      // Print a list of all data members
      char **memberlist = xcob_list_members (cob1);
      for (size_t i=0; memberlist[i]; i++) {
         printf ("Found member %s\n", memberlist[i]);
         free (memberlist[i]);
      }
      free (memberlist);
      xcob_del (cob1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
xcob_t *xcob_new (const char *name)
{
   xcob_t *ret = malloc (sizeof *ret);
   if (!ret) {
      XLOG ("Out of memory\n");
      goto errorexit;
   }
   memset (ret, 0, sizeof *ret);
   ret->name = xstr_dup (name);
   if (!ret->name) {
      XLOG ("Out of memory error\n");
      goto errorexit;
   }

   ret->pods = xdict_new ((int (*) (void *, void *))strcmp,
                          (void *(*) (void *)) xstr_dup,
                          (void (*) (void *))free);
   if (!ret->pods) {
      XLOG ("Out of error\n");
      goto errorexit;
   }
   ret->objs = NULL;

   return ret;
errorexit:
   if (ret) xcob_del (ret);
   return NULL;
}

/**
 * \brief Retrieves the name of the named object.
 *
 * Retrieves and returns the name of the named object \a cob. Should cob
 * be NULL, then NULL is returned. The returned value should not be modified
 * in any way.
 *
 * \sa xcob_set(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_serialise(), xcob_del(), xcob_list_objects(),
 * xcob_list_members(), xcob_list_data(), 
 * xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] cob The named object.
 *
 * @return On success a pointer to a const string is returned. The caller
 * must not free or modify this return value in any way.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a name
      xcob_set_name (cob1, "object-1");
      // Get the name 
      printf ("Name of object is %s\n", xcob_get_name (cob1));
      xcob_del (cob1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
const char *xcob_get_name (xcob_t *cob)
{
   return cob ? cob->name : NULL;
}

/**
 * \brief Sets the name of a named object
 *
 * Sets the name of the named object \a cob to \a name. A copy of the parameter
 * \a name is made so the caller may, after this function returns, free or
 * modify the argument to this function. The named object \a cob will have
 * a copy of the argument, so it is safe for the caller to destroy the argument
 * after the function returns.
 *
 * \sa xcob_set(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_serialise(), xcob_del(), xcob_list_objects(),
 * xcob_list_members(), xcob_list_data(), 
 * xcob_get_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] cob The named object.
 *
 * @return On success a pointer to a const string is returned. The caller
 * must not free or modify this return value in any way.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a name
      xcob_set_name (cob1, "object-1");
      // Get the name 
      printf ("Name of object is %s\n", xcob_get_name (cob1));
      xcob_del (cob1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
bool xcob_set_name (xcob_t *cob, char *name)
{
   char *tmp = NULL;
   if (!cob) return false;
   tmp = xstr_dup (name);
   if (!tmp) return false;
   if (cob->name)
      free (cob->name);
   cob->name = tmp;
   tmp = NULL;
   return true;
}

static void free_pods (void *n, void *v)
{
   n = n;
   free (v);
}

/**
 * \brief Delete a previously created named object.
 *
 * Deletes an object created with \a xcob_new(). All resources associated
 * with the object are released.
 *
 * \sa xcob_new(), xcob_set(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_serialise(), xcob_list_members(), xcob_list_objects(),
 * xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] obj The object to delete.
 *
 * @return Nothing
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a data member
      xcob_set (cob1, "dataMember1", "Value-1");
      // Get a data member 
      printf ("Member value is %s\n", xcob_get (cob1, "dataMember1"));
      // Print a list of all data members
      char **memberlist = xcob_list_members (cob1);
      for (size_t i=0; memberlist[i]; i++) {
         printf ("Found member %s\n", memberlist[i]);
         free (memberlist[i]);
      }
      free (memberlist);
      xcob_del (cob1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
void xcob_del (xcob_t *obj)
{
   if (obj) {
      if (obj->name) {
         free (obj->name);
         obj->name = NULL;
      }
      if (obj->pods) {
         xdict_iterate (obj->pods, free_pods);
         xdict_del (obj->pods);
         obj->pods = NULL;
      }
      if (obj->objs) {
         xvector_iterate (obj->objs, (void (*) (void *))xcob_del);
         xvector_free (obj->objs);
      }
      free (obj);
   }
}

/**
 * \brief Set a value for a data member.
 *
 * Stores the given value \a value in the member variable \a name within
 * the given object, \a cob. If the value does not exist it will be created.
 * If the value does exist it will be silently overwritten.
 *
 * \sa xcob_new(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_serialise(), xcob_del(), xcob_list_members(),
 * xcob_list_objects(), xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] cob The object to modify.
 * @param[in] name The name of the data member to assign \a value to.
 * @param[in] value The value to assign to data member \a name.
 *
 * @return On success a boolean true value is returned. On failure a 
 * boolean false value is returned.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a data member
      if (!xcob_set (cob1, "dataMember1", "Value-1")) {
         printf ("Unable to set member \n");
         return EXIT_FAILURE;
      }
      // Get a data member 
      printf ("Member value is %s\n", xcob_get (cob1, "dataMember1"));
      // Print a list of all data members
      char **memberlist = xcob_list_members (cob1);
      for (size_t i=0; memberlist[i]; i++) {
         printf ("Found member %s\n", memberlist[i]);
         free (memberlist[i]);
      }
      free (memberlist);
      xcob_del (cob1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
bool xcob_set (xcob_t *cob, char *name, char *value)
{
   if (!cob) return false;
   char *vvalue = xstr_dup (value);
   if (!vvalue) {
      XLOG ("Out of memory\n");
      return false;
   }
   char *tmp = xdict_set (cob->pods, name, vvalue);
   if (strcmp (tmp, value)==0) {
      return true;
   } else {
      XLOG ("Incorrect value stored: '%s' vs '%s'\n", tmp, value);
      return false;
   }
}

/**
 * \brief Return the value for a specific data member.
 *
 * Queries the given object \a cob and returns the value of the data member
 * \a name. If neither the object \a cob nor the data member \a name exists
 * then NULL is returned.
 *
 * \sa xcob_set(), xcob_new(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_serialise(), xcob_del(), xcob_list_members(),
 * xcob_list_objects(), xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] cob The object to query.
 * @param[in] name The data member to read from. 
 *
 *
 * @return On success a pointer to the data member's value is returned. The
 * caller must not free this pointer and any changes to this pointer is 
 * reflected in the object. On failure NULL is returned and the object remains
 * unchanged.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a data member
      xcob_set (cob1, "dataMember1", "Value-1");
      // Get a data member 
      printf ("Member value is %s\n", xcob_get (cob1, "dataMember1"));
      // Print a list of all data members
      char **memberlist = xcob_list_members (cob1);
      for (size_t i=0; memberlist[i]; i++) {
         printf ("Found member %s\n", memberlist[i]);
         free (memberlist[i]);
      }
      free (memberlist);
      xcob_del (cob1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
char *xcob_get (xcob_t *cob, char *name)
{
   return xdict_get (cob->pods, name);
}

static void *pods_pred (void *n, void *v)
{
   return xstr_cat (n, "=", v, "\n", NULL);
}

/**
 * \brief Creates a string representation of the specified object.
 *
 * Creates a NULL-string representation of the given object \a cob. The caller
 * is responsible for freeing this string. On failure a NULL is returned.
 *
 * \sa xcob_set(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_new(), xcob_del(), xcob_list_members(),
 * xcob_list_objects(), xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] cob The object to serialise.
 * @param[in] level The level of indentation for this object. This makes it
 * more more human-readable but can be set to '0' by the caller.
 *
 * @return On success, a freshly allocated NULL-terminated string is returned.
 * The caller is responsible for freeing this string.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a data member
      xcob_set (cob1, "dataMember1", "Value-1");
      // Get a data member 
      printf ("Member value is %s\n", xcob_get (cob1, "dataMember1"));
      char *string = xcob_serialise (cob1, 0);
      if (!string) {
         printf ("Error occurred when trying to serialise the object\n");
         return EXIT_FAILURE;
      }
      printf ("%s\n", string);
      free (string);
      // Print a list of all data members
      char **memberlist = xcob_list_members (cob1);
      for (size_t i=0; memberlist[i]; i++) {
         printf ("Found member %s\n", memberlist[i]);
         free (memberlist[i]);
      }
      free (memberlist);
      xcob_del (cob1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
char *xcob_serialise (xcob_t *cob, size_t level)
{
   char *ret = NULL;
   char *tmp;
   char indent[4196];
   memset (indent, ' ', sizeof indent-1);
   indent [sizeof indent - 1] = 0;
   if (level>=sizeof indent -1) level = sizeof indent -1;
   indent [level] = 0;
   if (cob) {
      ret = xstr_cat (indent, cob->name, "\n", NULL);
      if (!ret) goto errorexit;
   } else {
      goto errorexit;
   }

   if (cob->pods) {
      char **results = xdict_map (cob->pods, pods_pred);
      char *all_pods = xstr_join (results, 0);
      tmp = xstr_cat (ret, all_pods, NULL);
      if (!tmp) {
         XLOG ("Out of memory, deep copy error\n");
         goto errorexit;
      }
      free (ret); ret = tmp;
      xstr_delarray (results);
      free (all_pods);
   }

   if (cob->objs) {
      for (size_t i=0; i<XVECT_LENGTH(cob->objs); i++) {
         xcob_t *lcob = XVECT_INDEX (cob->objs, i);
         char *childobj = xcob_serialise (lcob, ++level);
         tmp = xstr_cat (ret, "----\n", childobj, NULL);
         if (childobj) free (childobj);
         if (!tmp) goto errorexit;
         free (ret); ret = tmp;
      }
   }

   return ret;
errorexit:
   if (ret) free (ret);
   return NULL;

}

/**
 * \brief Creates a composited object
 *
 * Creates a composited object, in which the member object, \a member, is a
 * member 
 * of another, the \a holding. The member object is directly linked as a member
 * of the holding object so that any operation on the member object via the
 * holding object is visible on the member object and any direct operation on
 * the member object will be seen by the holding object.
 *
 * When freeing the objects, it is enough to simply free the holding object.
 * Objects will free all resources they use, including any member objects.
 *
 * \sa xcob_set(), xcob_get(), xcob_serialise(), xcob_inherit(),
 * xcob_dup(), xcob_new(), xcob_del(), xcob_list_members(),
 * xcob_list_objects(), xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] parent The parent object.
 * @param[in] child The child object.
 *
 * @return On success a pointer to the member object is returned.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      xcob_t *cob2 = xcob_new ("Obj-2");
      if (!cob1 || !cob2) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      if (!xcob_append (cob1, cob2)) {
         printf ("Unable to set cob2 as a member of cob1\n");
         return EXIT_FAILURE;
      }
      char *string = xcob_serialise (cob1, 0);
      if (!string) {
         printf ("Error occurred when trying to serialise the object\n");
         return EXIT_FAILURE;
      }
      printf ("%s\n", string); // Prints out Obj-2 as a member of Obj-1
      free (string);
      xcob_del (cob1); // Member object gets deleted along with holding object

      return EXIT_SUCCESS;
   }
   \endverbatim
 */
xcob_t *xcob_append (xcob_t *holding, xcob_t *member)
{
   xvector_t *tmp;
   if (!holding || !member) {
      XLOG ("Cowardly refusing to deal with NULL holding or "
            "NULL member or both\n");
      return NULL;
   }
   tmp = xvector_ins_tail (holding->objs, member);
   if (!tmp) {
      XLOG ("Unable to append member to parent\n");
      return NULL;
   }
   holding->objs = tmp;
   return member;
}

/**
 * \brief Creates an inherited object
 *
 * Creates a new object with all the data members and member objects
 * of all the given parents. The parent objects are all the arguments after
 * the \a name argument, terminated with a NULL. Where data member names 
 * collide, precedence is given to the rightmost parent in the list of
 * arguments.
 *
 * In other words, should more than one parent have a data member
 * of the same name, then the child object would receive the data member
 * from the last parent specified in the argument list that has that data
 * member.
 *
 * The new object contains only copies of the parent object data members.
 * When the new object is freed the parent objects are still available and
 * vice versa. When the new object is modified, the parents are all unaffected
 * and vice versa.
 *
 *
 * \sa xcob_set(), xcob_get(), xcob_append(), xcob_serialise(),
 * xcob_dup(), xcob_new(), xcob_del(), xcob_list_members(),
 * xcob_list_objects(), xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] name The name of the new object.
 * @param[in] ...  All the parent objects, terminated with a NULL pointer.
 *
 * @return On success a freshly allocated object of name \a name containing
 * all the data members in all of the parent objects, in terms of the 
 * precedence rules explained above.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a data member
      xcob_set (cob1, "dataMember1", "Value-1");
      // Create a new object
      xcob_t *cob2 = xcob_inherit ("Obj-2", cob1, NULL);
      // cob2 now contains all the data members of cob1
      printf ("Member value is %s\n", xcob_get (cob2, "dataMember1"));
      xcob_del (cob1);
      xcob_del (cob2);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
xcob_t *xcob_inherit (char *name, ...)
{
   // TODO: Fix inheritance to also deep-copy the child objects.
   xcob_t *ret = xcob_new (name);
   if (!ret) {
      XLOG ("Unable to create new object\n");
      goto errorexit;
   }
   va_list ap;
   va_start (ap, name);
   xcob_t *tmp = va_arg (ap, xcob_t *);
   while (tmp) {
      char **indices = xcob_list_data (tmp);
      for (size_t i=0; indices && indices[i]; i++) {
         char *value = xcob_get (tmp, indices[i]);
         xcob_set (ret, indices[i], value);
         free (indices[i]);
      }
      for (size_t i=0; i<XVECT_LENGTH (tmp->objs); i++) {
         xcob_t *tobj = XVECT_INDEX (tmp->objs, i);
         xcob_t *nobj = xcob_dup (tobj, tobj->name);
         if (!nobj) {
            XLOG ("Error appending member object of parent\n");
            goto errorexit;
         }
         if (xcob_append (ret, nobj)!=nobj) {
            XLOG ("Something went wrong, unknown error\n");
            goto errorexit;
         }
      }
      free (indices);
      tmp = va_arg (ap, xcob_t *);
   }
   va_end (ap);
   return ret;

errorexit:
   if (ret) {
      xcob_del (ret);
   }
   return NULL;
}
static void *pods_pred_name (void *n, void *v)
{
   v = v;
   return xstr_dup (n);
}

/**
 * \brief Clones an object
 *
 * Creates a new object that is an exact clone of a specified object. The
 * new object will have the specified name, but the same data members with
 * the same values and the same member objects as the source object \a src.
 * A deep-copy is
 * performed, therefore the newly created object is not linked in any way
 * to the source object \a src.
 *
 * \sa xcob_set(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_serialise(), xcob_new(), xcob_del(), xcob_list_members(),
 * xcob_list_objects(), xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] cob The object to duplicate.
 * @param[in] name The name to give the newly created object.
 *
 * @return On success a freshly created object is returned. The caller is
 * responsible for freeing this object using \a xcob_del().
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a data member
      xcob_set (cob1, "dataMember1", "Value-1");
      xcob_t *cob2 = xcob_dup (cob1, "Obj-2");
      // cob2 is now an exact clone of cob1
      xcob_del (cob1);
      xcob_del (cob2);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
xcob_t *xcob_dup (xcob_t *src, const char *name)
{
   if (!src) goto errorexit;
   const char *nname = name ? name : src->name;
   xcob_t *ret = xcob_new (nname);
   char **names = xdict_map (src->pods, pods_pred_name);
   if (names) {
      for (size_t i=0; names[i]; i++) {
         char *value = xcob_get (src, names[i]);
         if (!value) continue;
         if (!xcob_set (ret, names[i], value)) {
            XLOG ("Failure to set name in deep copy\n");
            goto errorexit;
         }
      }
      xstr_delarray (names);
   }
   for (size_t i=0; i<XVECT_LENGTH (src->objs); i++) {
      xcob_t *obj = XVECT_INDEX (src->objs, i);
      xcob_t *newobj = xcob_dup (obj, NULL);
      if (!newobj) {
         XLOG ("Failure in recursive deep copy of objects\n");
         goto errorexit;
      }
      if (!xcob_append (ret, newobj)) {
         XLOG ("Failure to append new object in deep copy\n");
         goto errorexit;
      }
   }

   return ret;

errorexit:
   if (ret) {
      xcob_del (ret); ret = NULL;
   }
   return NULL;
}

/**
 * \brief Returns a list of all the members of an object
 *
 * Returns a NULL-terminated array of NULL-terminated strings for object
 * \a cob that contains the name of every data member and every object member
 * held in the object * \a cob.
 *
 * Each of the elements of the array contain the name of a data/object member
 * and must be free by the caller. The array itself must be freed by the
 * caller once all the elements have been free.
 *
 * \sa xcob_set(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_new(), xcob_del(), xcob_serialise(), xcob_list_objects(),
 * xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] cob The object to examine.
 *
 * @return On success a freshly allocated NULL-terminated array of 
 * NULL-terminated strings is returned. The caller is responsible for
 * freeing each element of the array and for freeing the array itself.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a data member
      xcob_set (cob1, "dataMember1", "Value-1");
      // Print a list of all data members
      char **memberlist = xcob_list_members (cob1);
      for (size_t i=0; memberlist[i]; i++) {
         printf ("Found member %s\n", memberlist[i]);
         free (memberlist[i]);
      }
      free (memberlist);
      xcob_del (cob1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
char **xcob_list_members (xcob_t *cob)
{
   char **pods = xdict_list_indices (cob->pods);
   char **objs = malloc (sizeof *objs * (XVECT_LENGTH (cob->objs)+1));
   if (!objs) {
      XLOG ("Out of memory error\n");
      goto errorexit;
   }
   size_t i;
   for (i=0; i<XVECT_LENGTH (cob->objs); i++) {
      xcob_t *tmp = XVECT_INDEX (cob->objs, i);
      objs[i] = xstr_dup (tmp->name);
      if (!objs[i]) {
         XLOG ("Out of memory error\n");
         goto errorexit;
      }
      objs[i+1] = NULL;
   }
   for (size_t j=0; pods[j]; i++, j++) ;
   char **ret = malloc (sizeof *ret * (i + 1));
   if (!ret) {
      XLOG ("Out of memory error\n");
      goto errorexit;
   }

   xstr_delarray (pods);
   xstr_delarray (objs);
   return ret;

errorexit:
   if (pods) {
      xstr_delarray (pods);
   }
   if (objs) {
      xstr_delarray (objs);
   }
   if (ret) {
      xstr_delarray (ret);
   }
   return NULL;
}

/**
 * \brief Returns a list of only the data members of an object
 *
 * Returns a NULL-terminated array of NULL-terminated strings for object
 * \a cob that contains the name of every data member held in the object
 * \a cob.
 *
 * Each of the elements of the array contain the name of a data member
 * and must be free by the caller. The array itself must be freed by the
 * caller once all the elements have been free.
 *
 * \sa xcob_set(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_new(), xcob_del(), xcob_serialise(), xcob_list_data(),
 * xcob_list_objects(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] cob The object to examine.
 *
 * @return On success a freshly allocated NULL-terminated array of 
 * NULL-terminated strings is returned. The caller is responsible for
 * freeing each element of the array and for freeing the array itself.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      xcob_t *cob1 = xcob_new ("Obj-1");
      if (!cob1) {
         printf ("Error creating object\n");
         return EXIT_FAILURE;
      }
      // Set a data member
      xcob_set (cob1, "dataMember1", "Value-1");
      // Print a list of all data members
      char **memberlist = xcob_list_data (cob1);
      for (size_t i=0; memberlist[i]; i++) {
         printf ("Found member %s\n", memberlist[i]);
         free (memberlist[i]);
      }
      free (memberlist);
      xcob_del (cob1);
      return EXIT_SUCCESS;
   }
   \endverbatim
 */
char **xcob_list_data (xcob_t *cob)
{
   return xdict_list_indices (cob->pods);
}

/**
 * \brief Returns a list of only the member objects of an object
 *
 * Returns a NULL-terminated array of NULL-terminated strings for object
 * \a cob that contains the name of every member object held in the object
 * \a cob.
 *
 * Each of the elements of the array contain the name of a data member
 * and must be free by the caller. The array itself must be freed by the
 * caller once all the elements have been free.
 *
 * \sa xcob_set(), xcob_get(), xcob_append(), xcob_inherit(),
 * xcob_dup(), xcob_new(), xcob_del(), xcob_serialise(), xcob_list_objects(),
 * xcob_list_data(),
 * xcob_get_name(), xcob_set_name(), xcobmgmt_startup(), 
 * xcobmgmt_has_started(), xcobmgmt_install(), xcobmgmt_uninstall(),
 * xcobmgmt_find(), xcobmgmt_shutdown()
 *
 * @param[in] cob The object to examine.
 *
 * @return On success a freshly allocated NULL-terminated array of 
 * NULL-terminated strings is returned. The caller is responsible for
 * freeing each element of the array and for freeing the array itself.
 *
 */
char **xcob_list_objects (xcob_t *cob)
{
   char **ret = malloc (sizeof *ret * (XVECT_LENGTH (cob->objs)+1));
   if (!ret) {
      XLOG ("Could not retrieve member object names\n");
      goto errorexit;
   }
   for (size_t i=0; i<XVECT_LENGTH (cob->objs); i++) {
      xcob_t *tmp = XVECT_INDEX (cob->objs, i);
      ret[i] = xstr_dup (tmp->name);
      ret[i+1] = NULL;
   }
   return ret;

errorexit:
   if (ret) {
      xstr_delarray (ret);
   }
   return NULL;
}
