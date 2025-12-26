/**
 * \file 
 *
 * \brief Configuration management routines
 *
 * A set of functions to provide management of config data for C programs. This
 * library provides routines to get config values from configuration files,
 * the command line and environment variables, while also providing sane
 * defaults.
 *
 * Configuration is stored globally for all callers and care needs to be
 * exercised when using these functions from different threads. All 
 * configuration data is stored as a tuple of Name, Value, Default Value,
 * Documentation and SetFrom.
 *
 * Section - The (optional) section of the configuration variable.
 *    (TODO: a temporary workaround is to use "section.name" as the name)
 *
 * Name - The name of the configuration variable.
 *
 * Value - The value that is set for the Named variable.
 *
 * Default Value - The value that will be used if it is not found on the 
 * command-line, nor in a configuration fie, nor in an environment variable.
 * 
 * Documentation - The help message that will be displayed to the user
 * when help is requested for this Named variable.
 *
 * SetFrom - A caller-supplied string describing where this variable got its
 * value from (for example, "Environment Variable", or "User Prompt", or
 * "Command Line Argument", etc). This value can be used to tell the user
 * exactly where a certain value came from (in case a config file overrides
 * their command-line, or vice-versa).
 *
 * 
 * xcfg is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 *
 */


#include <stdio.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <limits.h>
#include <math.h>

#include "xdict/xdict.h"
#include "xstring/xstring.h"
#include "xcfg/xcfg.h"

#define MAX_CFG_LINE_LENGTH      (4096)

static xdict_t *xp_globals;
static void xp_global_init (void)
{
   if (!xp_globals) {
      xp_globals = xdict_new ((int (*) (void *, void *))strcmp,
                              (void *(*) (void *))xstr_dup,
                              (void (*) (void *))free);
   }
}

static void xp_del_xcfg (xcfg_t *cfg)
{
   if (cfg) {
      if (cfg->name) free (cfg->name);
      if (cfg->value) free (cfg->value);
      if (cfg->from) free (cfg->from);
      if (cfg->doc) free (cfg->doc);
      if (cfg->defval) free (cfg->defval);
      free (cfg);
   }
}

static void xp_del_xcfg2 (void *name, void *value)
{
   name = name;
   xp_del_xcfg (value);
}

static xcfg_t *xp_new_xcfg (const char *name,
                            const char *value,
                            const char *from, const char *doc, 
                            const char *defval)
{
   xcfg_t *ret = malloc (sizeof *ret);
   if (!ret) goto error;
   memset (ret, 0, sizeof *ret);
   if (name) if (!(ret->name = xstr_dup (name))) goto error;
   if (value) if (!(ret->value = xstr_dup (value))) goto error;
   if (from) if (!(ret->from = xstr_dup (from))) goto error;
   if (doc) if (!(ret->doc = xstr_dup (doc))) goto error;
   if (defval) if (!(ret->defval = xstr_dup (defval))) goto error;
   return ret;
error:
   xp_del_xcfg (ret);
   return NULL;
}

/**
 * \brief Sets the documentation and default value for variable \a name
 *
 * Sets the documentation string and the default value to be used for
 * the variable \a name. If the variable doesn't exist, it will be created
 * and stored internally with documentation set to \a doc and default value
 * set to \a defval. 
 *
 * @param[in] section \a[RFU]
 * @param[in] name The variable to effect
 * @param[in] doc The documentation string for this variable
 * @param[in] defval The default value for this variable; a value that the
 *    variable gets if the caller never explicitly sets a value.
 *
 * @return On success 0 is returned. On failure a non-zero integer is returned.
 *
 * \a Example 
 * \verbatim
   int main (void)
   {
      xcfg_configure ("retries", 
         "Number of retries to attempt before giving up."
         "4");
      xcfg_configure ("destinaton", 
         "Server IP to connect to.",
         "127.0.0.1");

      // Here we read in the actual values from a file and set them with
      // xcfg_set (name, value, from) calls. If we never find them, the
      // default values above will be returned when xcfg_get() is called

      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
int xcfg_configure (const char *section, const char *name, const char *doc, const char *defval)
{
   section = section;
   xp_global_init ();
   xcfg_t *tmp = xdict_get (xp_globals, (void *)name);
   if (!tmp) {
      tmp = xp_new_xcfg (name, defval, "default", doc, defval);
      if (!tmp) return -1;
      if (xdict_set (xp_globals, (void *)name, (void *)tmp)!=tmp) return -2;
      else return 0;
   } else {
      if (tmp->doc) free (tmp->doc);
      if (tmp->defval) free (tmp->defval);
      tmp->doc = xstr_dup (doc);
      tmp->defval = xstr_dup (defval);
      return 0;
   }
}

/**
 * \brief Sets the value for \a name to \a value 
 *
 * Sets the value for \a name to \a value and the \a from-field for \a name
 * to \a from. The value can later be retrieved with xcfg_get(). The \a from
 * field can be retrieved along with the variables \a doc and \a default-value
 * field.
 *
 * \sa xcfg 
 *
 * @param[in] section \a[RFU]
 * @param[in] name The variable to set
 * @param[in] value The value to assign to the variable \a name
 * @param[in] from A caller-specified description of where this variable
 *    got its value from.
 *
 * @return On success the value that was set is returned. On failure \a NULL is
 * returned.
 *
 * \a Example 
 * \verbatim
   int main (void)
   {
      xcfg_set ("retries", 
         "Number of retries to attempt before giving up."
         "4");

      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
const char *xcfg_set (const char *section, const char *name, const char *value, const char *from)
{
   section = section;
   xp_global_init ();
   xcfg_t *cfg = xdict_get (xp_globals, (void *)name);
   if (!cfg) {
      cfg = xp_new_xcfg (name, value, from, NULL, NULL);
      if (!cfg) return NULL;
      if (xdict_set (xp_globals, (void *)name, (void *)cfg))
         return name;
      else
         return NULL;
   }
   if (cfg->value) free (cfg->value);
   if (cfg->from) free (cfg->from);
   cfg->value = xstr_dup (value);
   cfg->from = xstr_dup (from);
   return name;
}

/**
 * \brief Gets the value for variable \a name
 *
 * Gets the value for \a name. If the \a name was never set then the value 
 * returned is the default value. If a default value does not exist then
 * \a NULL is returned.
 *
 * \sa xcfg_set(), xcfg_get_i(), xcfg_get_u, xcfg_get_f()
 *
 * @param[in] section \a[RFU]
 * @param[in] name The variable to read
 *
 * @return On success the value that was set or the default value if no value
 * was set is returned. On failure \a NULL is returned.
 *
 * \a Example 
 * \verbatim
   int main (void)
   {
      xcfg_set ("retries", 
         "Number of retries to attempt before giving up."
         "4");
      // ...
      printf ("Retries is set to %s\n", xcfg_get ("retries")); 
      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
const char *xcfg_get (const char *section, const char *name)
{
   section = section;
   xcfg_t *tmp = xdict_get (xp_globals, (void *)name);
   if (!tmp) return NULL;
   if (!tmp->value) return tmp->defval;
   else return tmp->value;
}

/**
 * \brief Gets the value for variable \a name as an integer
 *
 * Gets the value for \a name, converted to an integer.
 * If \a name was never set then the value 
 * returned is the default value. If a default value does not exist then
 * \a INT_MAX is returned.
 *
 * \sa xcfg_set(), xcfg_get()
 *
 * @param[in] section \a[RFU]
 * @param[in] name The variable to read
 *
 * @return On success the value that was set or the default value if no value
 * was set is returned. On failure \a INT_MAX is returned.
 *
 * \a Example 
 * \verbatim
   int main (void)
   {
      xcfg_set ("retries", 
         "Number of retries to attempt before giving up."
         "4");
      // ...
      printf ("Retries is set to %i\n", xcfg_get_i ("retries")); 
      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
int xcfg_get_i (const char *section, const char *name)
{
   int retval;
   const char *s = xcfg_get (section, name);
   if (!name || !s) return INT_MAX;
   if (sscanf (s, "%i", &retval)!=1) return INT_MAX;
   else return retval;
}

/**
 * \brief Gets the value for variable \a name as an unsigned integer
 *
 * Gets the value for \a name, converted to an unsigned integer.
 * If \a name was never set then the value 
 * returned is the default value. If a default value does not exist then
 * \a SIZE_MAX is returned.
 *
 * \sa xcfg_set(), xcfg_get(), xcfg_get_i()
 *
 * @param[in] section \a[RFU]
 * @param[in] name The variable to read
 *
 * @return On success the value that was set or the default value if no value
 * was set is returned. On failure \a SIZE_MAX is returned.
 *
 * \a Example 
 * \verbatim
   int main (void)
   {
      xcfg_set ("retries", 
         "Number of retries to attempt before giving up."
         "4");
      // ...
      printf ("Retries is set to %zu\n", xcfg_get_u ("retries")); 
      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
size_t xcfg_get_u (const char *section, const char *name)
{
   size_t retval;
   const char *s = xcfg_get (section, name);
   if (!name) return SIZE_MAX;
   if (sscanf (s, "%zu", &retval)!=1) return SIZE_MAX;
   else return retval;
}


/**
 * \brief Gets the value for variable \a name as a float
 *
 * Gets the value for \a name, converted to a float.
 * If the name was never set then the value 
 * returned is the default value. If a default value does not exist then
 * \a NAN is returned.
 *
 * \sa xcfg_set(), xcfg_get(), xcfg_get_i()
 *
 * @param[in] section \a[RFU]
 * @param[in] name The variable to read
 *
 * @return On success the value that was set or the default value if no value
 * was set is returned. On failure \a NAN is returned.
 *
 * \a Example 
 * \verbatim
   int main (void)
   {
      xcfg_set ("alpha", 
         "Translucency, between 0.0 and 1.0."
         "0.54231");
      // ...
      printf ("Retries is set to %f\n", xcfg_get_f ("alpha")); 
      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
float xcfg_get_f (const char *section, const char *name)
{
   float retval;
   const char *s = xcfg_get (section, name);
   if (!name || !s) return NAN;
   if (sscanf (s, "%f", &retval)!=1) return NAN;
   else return retval;
}

/**
 * \brief Sets the variable \a name using the environment
 *
 * Sets the value of \a name to a value taken out of the environment using
 * the same name for the environment variable. For example, using "HOME" as
 * the argument (the \a name), this function sets the internal variable to 
 * the value that the environment variable "HOME" has.
 *
 *
 * \sa xcfg_set(), xcfg_get()
 *
 * @param[in] name The variable to set
 *
 * @return On success the value that was set is returned. On failure
 * \a NULL is returned.
 *
 * \a Example 
 * \verbatim
   int main (void)
   {
      xcfg_from_env ("PATH");
      // ...
      printf ("Path is set to %s\n", xcfg_get ("PATH")); 
      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
const char *xcfg_from_env (const char *name)
{
   char *value = getenv (name);
   if (!name || !value) return NULL;
   if (!xcfg_set (NULL, name, value, "Environment")) return NULL;
   return value;
}


/**
 * \brief Frees all the variables and the resources they use
 *
 * Frees all the memory associated with all internal variables and values. This
 * should be called before program end or after the caller is certain that none
 * of the variables will be needed again. After this function returns all the
 * variables that were set with xcfg_set() and friends will cease to exist.
 *
 * The caller can always call xcfg_set() and friends to store a new set of
 * internal variables. No parameters are taken.
 *
 * \sa xcfg 
 *
 * @return Nothing.
 *
 * \a Example 
 * \verbatim
   int main (void)
   {
      xcfg_set ("retries", 
         "Number of retries to attempt before giving up."
         "4");

      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
void xcfg_shutdown (void)
{
   if (xp_globals) {
      xdict_iterate (xp_globals, xp_del_xcfg2);
      xdict_del (xp_globals);
   }
   xp_globals = NULL;
}

static void *xp_collect_all (void *n, void *v)
{
   n = n;
   return v;
}

/**
 * \brief Returns a list of all the internal variables
 *
 * Returns a list of all the internal variables that were set by the caller.
 * The caller has to free the returned array, but not the elements in the
 * returned array (see example).
 *
 * No parameters are taken.
 *
 * \sa xcfg 
 *
 * @return On success an array (terminated with a NULL pointer) of 
 * xcfg_t pointers is returned. While the caller has to free the array that 
 * is returned, the caller should not modify the elements of the array.
 *
 * \a Example 
 * \verbatim
   int main (void)
   {
      xcfg_set ("retries", 
         "Number of retries to attempt before giving up."
         "4");
      ...
      // Many more xcfg_set and xcfg_configure calls
      ...
      xcfg_t **all = xcfg_get_all (); // Get all the internal variables
      xcfg_t **tmp = all;  // Keep a pointer to the first one to free it later
      while (all && *all) {
         printf ("name=%s\n", (*all)->name);
         printf ("value=%s\n", (*all)->value);
         printf ("doc=%s\n", (*all)->doc);
         printf ("from=%s\n", (*all)->from);
         printf ("default=%s\n", (*all)->defval);
         all++;
      }
      free (tmp); // Free the returned value (but not each element)

      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
xcfg_t **xcfg_get_all (void)
{
   xcfg_t **all = xdict_map (xp_globals, xp_collect_all);
   return all;
}


/**
 * \brief Sets internal configuration variables from given file
 *
 * Sets internal configuration variables from the file specified by
 * \a filename. The file is read a single line at a time, with a maximum
 * line size of 4096 bytes. Each line is of the form "name = value". The
 * name may be composed of spaces (leading and trailing spaces are ignored)
 * and any character except '='. Values may contain anything except newlines
 ( leading and trailing spaces are ignored).
 * Empty lines are ignored. The value is optional and will revert to an empty
 * string if no value is found after the '='. Lines without the delimiter
 * '=' are ignored.
 *
 * A comment is everything from the first hash character (#) to the end of
 * the line. Hashes are escaped using the two-char sequence "\#". The internal
 * variables that are set (see the main xcfg help page) have their \a SetFrom
 * field set to the filename.
 * 
 *
 *
 * @param[in] section \a[RFU]
 * @param[in] filename The filename to read for name/value pairs.
 *
 * @return On success the number of variables successfully set is returned.
 * On failure (size_t)-1 is returned.
 *
 * \sa xcfg, xcfg_set() xcfg_from_env(), xcfg_from_array()
 *
 *
 * \a Example \a configuration \a file
 * \verbatim
     # Begin small example configuration - Test.cfg
     host retries = 3           # Name = "host retries"
     example-string = a = b = c # Value = "a = b = c"
     check-local=               # Value = ""
     check-remote               # This line gets ignored
     withHash = My \# String    # Value = "My # String"
     # End small example configuration - Test.cfg
  \endverbatim
 *
 * \a Example \a program
 * \verbatim
   int main (void)
   {
      xcfg_from_file ("Test.cfg");
      ...
      xcfg_t **all = xcfg_get_all (); // Get all the internal variables
      xcfg_t **tmp = all;  // Keep a pointer to the first one to free it later
      while (all && *all) {
         printf ("name=%s\n", (*all)->name);
         printf ("value=%s\n", (*all)->value);
         printf ("doc=%s\n", (*all)->doc);
         printf ("from=%s\n", (*all)->from);
         printf ("default=%s\n", (*all)->defval);
         printf ("==========================\n");
         all++;
      }
      free (tmp); // Free the returned value (but not each element)

      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 * 
 * The output from the above is:
 * \verbatim
      name=host retries
      value='3'
      doc=(null)
      from=t2
      default=(null)
      ==========================
      name=example-string
      value='a = b = c'
      doc=(null)
      from=t2
      default=(null)
      ==========================
      name=check-local
      value=''
      doc=(null)
      from=t2
      default=(null)
      ==========================
      name=withHash
      value='My \# String'
      doc=(null)
      from=t2
      default=(null)
      ==========================
  \endverbatim
 */
size_t xcfg_from_file (const char *filename)
{
   FILE *infile = fopen (filename, "r");
   if (!infile) return (size_t)-1;
   static char line[MAX_CFG_LINE_LENGTH * 2];
   int num_variables = 0;
   char *section = NULL;
   while (fgets (line, sizeof line/2, infile)) {
      // Remove everything from hash onwards, unless the hash is escaped
      char *tmp = strchr (line, '#');
      if (tmp==line) continue;
      while (tmp && *(tmp - 1)=='\\') {
         tmp = strchr (tmp+1, '#');
      }
      if (tmp && *tmp=='#' && *(tmp - 1)!='\\') *tmp = 0;
      if (tmp && *(tmp - 1)=='\\') memmove ((tmp - 1), tmp, strlen (tmp));
      // Ignore this line if no equal sign exists
      if (!strchr (line, '=')) continue;
      // Split into fields based on delim
      char **pair = xstr_split (line, "=");
      if (!pair) continue;
      // Free the result and continue if no fields found
      if (!pair[0]) {
         free (pair);
         continue;
      }
      // First field is the name, always
      char *name=xstr_dup (pair[0]);
      free (pair[0]);
      // Second field is all the other fields joined
      char *value=xstr_join (&pair[1], '=');
      for (size_t i=1; pair[i]; i++) {
         free (pair[i]);
      }
      xstr_trim (name);
      xstr_trim (value);
      // Free the results of xstr_split
      free (pair);
      // Store the name/value pair
      if (*name) {
         if (xcfg_set (section, name, value, filename)) num_variables++;
      }
      free (name); free (value);
   }
   fclose (infile);
   return num_variables;
}

/**
 * \brief Sets internal configuration variables from given array of strings
 *
 * Sets internal configuration variables from the array of strings \a argv.
 * The array \a argv must be terminated with a \a NULL pointer, as if it were
 * declared as follows:
 * \verbatim
   char *argv[] = {"--name1=value1", "name2=value2", ..., NULL};
   \endverbatim
 * 
 * All the variables to be set must be of the form "\--name=value". If value
 * is ommitted the variable \a name is assigned an empty non-NULL zero-length
 * value.
 *
 * A "\--" on its' own causes the processing of variables to end. Strings
 * in the array \a argv after the "\--" string are ignored.
 *
 * This function is designed to retrieve values straight out of the \a argv
 * parameter that is passed to the \a %main() function, so it is safe to pass
 * the \a argv parameter to \a %main() directly to xcfg_from_array(). This
 * is the easiest way to retrieve command line arguments passed to the
 * program.
 *
 * @param[in] section \a[RFU]
 * 
 * @param[in] argv The array of strings containing the name/value pairs
 * of each variable's value.
 * @param[in] from The identifier that is used in the \a SetFrom field
 * of the variable. This need not be unique, but does, in a later part of the
 * program, help the caller determine \a where a variable got its' value
 * from.
 *
 * @return On success the number of variables successfully set is returned.
 * On failure (size_t)-1 is returned.
 *
 * \sa xcfg, xcfg_set() xcfg_from_env(), xcfg_from_file()
 *
 *
 * \a Example \a program
 * \verbatim
 int main (void)
   {
      char *options = {
         "--one=1111",
         "--two",
         "--three=3333",
         "--four",
         "--",
         "--five=5555",
         NULL,
      };
      xcfg_from_array ("", options, "Internal")
      ...
      xcfg_t **all = xcfg_get_all (); // Get all the internal variables
      xcfg_t **tmp = all;  // Keep a pointer to the first one to free it later
      while (all && *all) {
         printf ("name=%s\n", (*all)->name);
         printf ("value=%s\n", (*all)->value);
         printf ("doc=%s\n", (*all)->doc);
         printf ("from=%s\n", (*all)->from);
         printf ("default=%s\n", (*all)->defval);
         printf ("==========================\n");
         all++;
      }
      free (tmp); // Free the returned value (but not each element)

      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 * 
 * The output from the above is:
 * \verbatim
      name=one
      value='1111'
      doc=(null)
      from=from c/line
      default=(null)
      ==========================
      name=two
      value=''
      doc=(null)
      from=from c/line
      default=(null)
      ==========================
      name=three
      value='3333'
      doc=(null)
      from=from c/line
      default=(null)
      ==========================
      name=four
      value=''
      doc=(null)
      from=from c/line
      default=(null)
      ==========================
  \endverbatim
 */
size_t xcfg_from_array (const char *section, const char **argv, const char *from)
{
   int num_variables=0;
   for (size_t i=0; argv[i]; i++) {
      if (argv[i][0]=='-' && argv[i][1]=='-') {
         if (argv[i][2]==0) break;
         // Found a name/value pair: --name=value
         char **nvpair = xstr_split (&argv[i][2], "=");
         if (!nvpair[0]) {
            free (nvpair);
            continue;
         }
         char *name = xstr_dup (nvpair[0]);
         free (nvpair[0]);
         char *value=xstr_join (&nvpair[1], '=');
         for (size_t i=1; nvpair[i]; i++) free (nvpair[i]);
         free (nvpair);
         if (name && *name) {
            if (xcfg_set (section, name, value, from)) num_variables++;
         }
         free (name); free (value);
      }
   }
   return num_variables;
}

static FILE *xp_outf;
static void xp_write_cfgvar (void *n, void *v)
{
   char *name = n;
   xcfg_t *value = v;
   if (!xp_outf) return;
   fprintf (xp_outf, "# =======================================\n");
   fprintf (xp_outf, "# Variable %s\n", name);
   fprintf (xp_outf, "# %s\n", value->doc);
   fprintf (xp_outf, "# default = %s\n", value->defval);
   fprintf (xp_outf, "%s = %s \n\n\n", name, value->value);
}

/**
 * \brief Saves the state of all the configuration variables to a
 * config file
 *
 * Saves the state of all the configuration variables that are currently
 * stored in the program to a configuration file that can be read in with
 * xcfg_from_file(). This allows the caller to save program configuration
 * state and reload it later, as well as generate a working configuration
 * file from all the values read in thus far.
 *
 * The variables are saved along with their description and default values
 * (description and default values are saved as comments in the output
 * file).
 *
 * \sa xcfg, xcfg_set() xcfg_from_env(), xcfg_from_file(), xcfg_from_array()
 *
 * @param[in] filename The filename to save the configuration variables to
 *
 * @return On success zero is returned. On failure a negative number is
 * returned.
 *
 * \a Example \a program
 * \verbatim
   int main (void)
   {
      char *options = {
         "--one=1111",
         "--two",
         "--three=3333",
         "--four",
         "--",
         "--five=5555",
         NULL,
      };
      xcfg_from_array (options, "Internal")
      ...
      xcfg_save_config ("config.out");

      xcfg_shutdown (); // Free all resources for the variables
      return EXIT_SUCCESS;
   }
  \endverbatim
 * 
 */
int xcfg_save_config (char *filename)
{
   xp_outf = NULL;
   if (strcmp (filename, "stderr")==0) xp_outf = stderr;
   if (strcmp (filename, "stdout")==0) xp_outf = stdout;
   if (!filename) return -1;
   if (!xp_outf) {
      xp_outf = fopen (filename, "w");
      if (!xp_outf) return -2;
   }
   fprintf (xp_outf, "# \n# This file autogenerated by xconfig.\n# \n\n");
   xdict_iterate (xp_globals, xp_write_cfgvar);
   fclose (xp_outf); xp_outf = NULL;
   return 0;
}

