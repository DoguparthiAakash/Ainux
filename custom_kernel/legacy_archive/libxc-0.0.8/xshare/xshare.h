#ifndef H_XSHARE
#define H_XSHARE  ("1.0")

#ifndef DLL_LIB
#ifndef SO_LIB
#error No library type defined - please define either DLL_LIB or SO_LIB
#endif
#endif

#ifdef PLATFORM_DEFINITIONS_COMPLETE
#undef PLATFORM_DEFINITIONS_COMPLETE
#endif

/* All the platform specific definitions for windows */
#ifdef DLL_LIB
#include <windows.h>
#include <winbase.h>
typedef HMODULE xshare_library_t;
typedef FARPROC xshare_symbol_t;
#define SLIB_INIT_FUNC           BOOL WINAPI DllEntryPoint
#define SLIB_INIT_FUNC_ARGS      HINSTANCE hinst, DWORD reason, LPVOID res
#define SUFFIX       (".dll")
#define PLATFORM_DEFINITIONS_COMPLETE     (1)
#endif

/* All the platform specific definitions for unix */
#ifdef SO_LIB
#include <dlfcn.h>
typedef void *xshare_library_t;
typedef void *xshare_symbol_t;
#define SLIB_INIT_FUNC           void _init
#define SLIB_INIT_FUNC_ARGS      void
#define PLATFORM_DEFINITIONS_COMPLETE     (1)
#define SUFFIX       (".so")
#endif

#ifndef PLATFORM_DEFINITIONS_COMPLETE
#error \
   "Not all definitions for specified platform have been made.\n" \
   "Please contact the maintainer of this package if your \n"\
   "platform was documented as 'working' for this package.\n" 
#endif

#undef PLATFORM_DEFINITIONS_COMPLETE
#undef HELP_MSG


#ifdef __cplusplus
extern "C" {
#endif

   xshare_library_t xshare_open   (char *libname);
   xshare_symbol_t xshare_symbol (xshare_library_t lib, char *symbol);
   void xshare_close  (xshare_library_t lib);
   int xshare_error  (void);
   char *xshare_errmsg (void);


#ifdef __cplusplus
};
#endif




#endif
