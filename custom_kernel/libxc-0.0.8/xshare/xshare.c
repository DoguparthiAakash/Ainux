/**
 * \file 
 *
 * \brief Routines to load dynamic libraries at runtime
 *
 * A set of functions to load dynamic libraries and then locate
 * symbols within those libraries at runtime. Xshare is simply a 
 * cross-platform way to load runtime libraries and has been tested on
 * unix variants (Linux, FreeBSD) and Microsoft Windows.
 * 
 * \sa xshare_open(), xshare_symbol(), xshare_close(), xshare_close(),
 *    xshare_error()
 *
 * xshare is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 *
 */
#include <stdlib.h>
#include <string.h>

#include "xshare/xshare.h"

#define MAX_ERROR_MSG            (255)

/**
 * \brief Open a dynamic library
 *
 * Opens the given dynamic library \a libname for further use with
 * xshare_symbol(). Note that \a libname is only the filename, not the
 * extension. The library that will be searched for depends on the platform.
 * On unix variants the library that will be searched for is \a libname.so and
 * on Microsoft Windows the library that will be searched for is 
 * \a libname.dll.
 *
 * \sa xshare_symbol(), xshare_close(), xshare_close(), xshare_error()
 *
 * @param[in] libname The library to search for
 *
 * @return On success, a handle to the library is returned. The handle
 * is used for further calls to xshare_symbol() and can be passed to
 * xshare_close() to explicitly unload the library, although the system
 * will unload the library at program exit. On failure
 * (void *)NULL is returned. 
 *
 * \a Example:
 * \verbatim
  int main (void)
   {
      xshare_library_t ext_lib = xshare_open ("libc");
      if (ext_lib==NULL) {
         printf ("Could not load the libc library\b");
         return EXIT_FAILURE;
      }
      int (*ext_func) (const char *) = xshare_symbol (ext_lib, "puts");
      if (!ext_func) {
         xshare_close (ext_lib);
         printf ("Could not locate the puts function in libc\n");
         return EXIT_FAILURE;
      }
      ext_func ("Hello World\n");
      xshare_close (ext_lib);
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
xshare_library_t xshare_open (char *libname)
{
   char *fullname = malloc (strlen (libname) + strlen (SUFFIX) +1);
   if (!fullname) {
      return NULL;
   }
   strcpy (fullname, libname);
   strcat (fullname, SUFFIX);
   xshare_library_t ret = 0;
#ifdef DLL_LIB
   ret = LoadLibrary (fullname);
#endif
#ifdef SO_LIB
   ret = dlopen (fullname, RTLD_NOW);
#endif
   free (fullname);
   return ret;
}

/**
 * \brief Locate a symbol within a dynamic library
 *
 * Locates the symbol \a symbol within the dynamic library \a lib. Note that
 * \a lib is a handle returned by xshare_open(). The caller is required to
 * ensure that the return value of \a xshare_symbol() is cast to the correct
 * type before using it.
 *
 * \sa xshare_open(), xshare_close(), xshare_close(), xshare_error()
 *
 * @param[in] lib The handle to the library (previously opened with 
 * xshare_open())
 * @param[in] symbol The symbol to locate within the library \a lib
 *
 * @return On success, a pointer to the symbol \a symbol within the library
 *    \a lib is returned.  On failure (void *)NULL is returned. 
 *
 * \a Example:
 * \verbatim
  int main (void)
   {
      xshare_library_t ext_lib = xshare_open ("libc");
      if (ext_lib==NULL) {
         printf ("Could not load the libc library\b");
         return EXIT_FAILURE;
      }
      int (*ext_func) (const char *) = xshare_symbol (ext_lib, "puts");
      if (!ext_func) {
         xshare_close (ext_lib);
         printf ("Could not locate the puts function in libc\n");
         return EXIT_FAILURE;
      }
      ext_func ("Hello World\n");
      xshare_close (ext_lib);
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
xshare_symbol_t xshare_symbol (xshare_library_t lib, char *symbol)
{
   xshare_symbol_t ret;
#ifdef DLL_LIB
   ret = GetProcAddress (lib, symbol);
#endif
#ifdef SO_LIB
   ret = dlsym (lib, symbol);
#endif
   return ret;
}

/**
 * \brief Close a previously opened dynamic library
 *
 * Closes a previously opened dynamic library that was opened with
 * \a xshare_open().
 *
 * \sa xshare_open(), xshare_symbol(), xshare_close(), xshare_error()
 *
 * @return Nothing.
 *
 * \a Example:
 * \verbatim
  int main (void)
   {
      xshare_library_t ext_lib = xshare_open ("libc");
      if (ext_lib==NULL) {
         printf ("Could not load the libc library\b");
         return EXIT_FAILURE;
      }
      int (*ext_func) (const char *) = xshare_symbol (ext_lib, "puts");
      if (!ext_func) {
         xshare_close (ext_lib);
         printf ("Could not locate the puts function in libc\n");
         return EXIT_FAILURE;
      }
      ext_func ("Hello World\n");
      xshare_close (ext_lib);
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
void xshare_close (xshare_library_t lib)
{
#ifdef DLL_LIB
   FreeLibrary (lib);
#endif
#ifdef SO_LIB
   dlclose (lib);
#endif
}

/**
 * \brief Returns an integer describing the last error
 *
 * Returns an integer describing the last error that occurred. This value
 * differs from platform to platform and is thus almost totally useless
 * in a program that assumes no knowledge of the underlying platform.
 * Use \a xshare_errmsg() to get a human-readable description of the
 * error.
 *
 * \sa xshare_open(), xshare_symbol(), xshare_close(), xshare_close(),
 *    xshare_errmsg()
 *
 * @return A platform-defined integer describing the last error that
 *    occurred.
 *
 * \a Example:
 * \verbatim
  int main (void)
   {
      xshare_library_t ext_lib = xshare_open ("libc");
      if (ext_lib==NULL) {
         printf ("Could not load the libc library - %i\b", xshare_error());
         return EXIT_FAILURE;
      }
      int (*ext_func) (const char *) = xshare_symbol (ext_lib, "puts");
      if (!ext_func) {
         printf ("Locate of 'puts' in libc failed - %i\n", xshare_error());
         xshare_close (ext_lib);
         return EXIT_FAILURE;
      }
      ext_func ("Hello World\n");
      xshare_close (ext_lib);
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
int xshare_error  (void)
{
#ifdef DLL_LIB
   return GetLastError ();
#endif
#ifdef SO_LIB
   return dlerror () == NULL ? 0 : 1;
#endif
}

/**
 * \brief Returns a human-readable string describing the last error
 *
 * Returns a human-readable string describing the last error that occurred. 
 * The actual string will differ from platform to platform but can be used
 * unchanged by the caller to present the user of the program with an error
 * message.
 *
 * This function is not reentrant and thus multiple calls to any xshare
 * function will overwrite this message. The caller should make a copy
 * of the return value if the value is needed in between calls to any
 * xshare function. The string that is returned will have a maximum
 * length of 256 characters, including the NULL-terminator.
 *
 * \sa xshare_open(), xshare_symbol(), xshare_close(), xshare_close(),
 *    xshare_error()
 *
 * @return A platform-defined human-readable string describing the last
 * error that occurred.
 *
 * \a Example:
 * \verbatim
  int main (void)
   {
      xshare_library_t ext_lib = xshare_open ("libc");
      if (ext_lib==NULL) {
         printf ("Could not load the libc library - %s\b", xshare_errmsg());
         return EXIT_FAILURE;
      }
      int (*ext_func) (const char *) = xshare_symbol (ext_lib, "puts");
      if (!ext_func) {
         printf ("Locate of 'puts' in libc failed - %s\n", xshare_errmsg());
         xshare_close (ext_lib);
         return EXIT_FAILURE;
      }
      ext_func ("Hello World\n");
      xshare_close (ext_lib);
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
char *xshare_errmsg (void)
{
   static char ret[MAX_ERROR_MSG];
   memset (ret, 0, sizeof ret);
#ifdef DLL_LIB
   LPVOID lpMsgBuf;
    
   FormatMessage ( 
            FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM,
            NULL,
            GetLastError(),
            MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
            (LPTSTR) &lpMsgBuf,
            0,
            NULL);

   strncpy (ret, lpMsgBuf, sizeof ret -1);
   LocalFree (lpMsgBuf);
#endif
#ifdef SO_LIB
   char *tmp = dlerror ();
   if (tmp) {
      strncpy (ret, tmp, sizeof ret -1);
   }
#endif
   return ret;
}

