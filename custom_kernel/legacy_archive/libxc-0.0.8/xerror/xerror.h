
#ifndef H_XERROR
#define H_XERROR "0.0.1"

#include <stdarg.h>

/**
 * \file xerror.h
 *
 * \brief A set of error reporting macros.
 *
 * It is unwise to use the actual functions in xerror.c such as
 * xerror_diagnostic() or xerror_log() Instead, the caller is urged to use
 * the macros XERROR() and XLOG() respectively instead.
 *
 * xerror is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 */

#define XERROR(...)     xerror_diagnostic (__FILE__, __func__, __LINE__, \
                                           __VA_ARGS__)

/**
 *
 * \def XERROR(formatstring, ...)
 *
 * \brief Outputs given formatted string \a formatstring to \a stderr, along
 * with the Source filename, function name and line number
 *
 * \a XERROR() is a macro that uses the given formatted string \a formatstring
 * along with all of the elements that \a formatstring refers to, to produce
 * output on stderr which prefixed with the source filename, function name and
 * line number that XERROR was invoked on.
 *
 * The format of \a formatstring is the same as for printf, and the 
 * requirements for all the arguments after \a formatstring are also the same
 * as printf. See the printf manpage and/or documentation for an in-depth
 * explanation of the format specifiers.
 *
 * @param[in] formatstring The format string that controls the output, 
 * consisting of literal output interspersed with format specifiers.
 * @param[in] ... All the subsequent arguments as required by the format
 * specifiers in \a formatstring.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      char *sa = " String Arg ";
      int ia = 42;
      float fa = 54.149;

      XERROR ("Trying a macro with %s, check if %i works\nAnd %f\n", 
         sa, ia, fa);

      return EXIT_SUCCESS;
   }
  \endverbatim
 * Output of above program:
 * \verbatim
   main.c:main():14:Trying a macro with  String Arg , check if 42 works
   And 54.148998
  \endverbatim
 *
 *
 * @return Nothing. Permanent side-effects on the \a stderr file stream.
 */

#define XLOG(...)     xerror_log (__FILE__, __func__, __LINE__, \
                                  __VA_ARGS__)

/**
 *
 * \def XLOG(formatstring, ...)
 *
 * \brief Outputs given formatted string \a formatstring to \a the logfile,
 * along
 * with the Source filename, function name and line number
 *
 * \a XLOG() is a macro that uses the given formatted string \a formatstring
 * along with all of the elements that \a formatstring refers to, to produce
 * output on logfile which prefixed with the source filename, function name 
 * and line number that XLOG was invoked on. If a logfile name was not
 * specified with \a xerror_set_logfile() then all log messages go to
 * stderr.
 *
 * The format of \a formatstring is the same as for printf, and the 
 * requirements for all the arguments after \a formatstring are also the same
 * as printf. See the printf manpage and/or documentation for an in-depth
 * explanation of the format specifiers.
 *
 * \sa XERROR(), xerror_set_logfile()
 *
 * @param[in] formatstring The format string that controls the output, 
 * consisting of literal output interspersed with format specifiers.
 * @param[in] ... All the subsequent arguments as required by the format
 * specifiers in \a formatstring.
 *
 * \a Example:
 * \verbatim
   int main (void)
   {
      char *sa = " String Arg ";
      int ia = 42;
      float fa = 54.149;

      XLOG ("Trying a macro with %s, check if %i works\nAnd %f\n", 
         sa, ia, fa);

      return EXIT_SUCCESS;
   }
  \endverbatim
 * Output of above program:
 * \verbatim
   main.c:main():14:Trying a macro with  String Arg , check if 42 works
   And 54.148998
  \endverbatim
 *
 *
 * @return Nothing. Permanent side-effects on the \a logfile file stream
 * that was previously opened with \a xerror_set_logfile().
 */
#ifdef __cplusplus
extern "C" {
#endif

   void xerror_diagnostic (char const *filename, char const *func,
                           size_t line, char *fmt, ...);
   int xerror_set_logfile (const char *filename);
   void xerror_log (char const *filename, char const *func, size_t line, 
                  char *fmt, ...);
   void xerror_flush (void);

#ifdef __cplusplus
};
#endif




#endif
