#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>

#include "xerror/xerror.h"

/**
 * \file xerror.c
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
/**
 *
 * \brief Foundation for the XERROR() macro
 *
 * Under most circumstances it is not necessary to use this function directly.
 * It should only be used by invoking the XERROR() macro, so please visit
 * the documentation for XERROR(). For completeness
 * this function is documented here anyway, although you are warned against
 * using it directly.
 * 
 * \sa xerror_set_logfile(), XLOG()
 *
 * @param[in] filename The source filename where this function was called
 * @param[in] func The name of the caller that called this function
 * @param[in] line The line number in the source file \a filename where this
 *  function was called
 * @param[in] fmt The format string that controls the output using 
 *    printf style format specifiers
 * @param[in] ... The arguments specified by the format strings format
 * specifiers. See the printf documentation for more information on this.
 *
 * @return Nothing. A side-effect occurrs, by printing out the error message
 *  to stderr
 * 
 * \a Example:
 *
 * \verbatim
   int main (void)
   {
      int x = 42;
      char *y = "string arg";
      float z = 98.718273;
      XERROR ("A Small Error using %i, %s and %f\n", x, y, z);
      // outputs:
      // main.c:10:main():A Small Error using 42, string arg and 98.718273
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
void xerror_diagnostic (char const *filename,
                        char const *func,
                        size_t line,
                        char *fmt, ...)
{
   va_list ap;
   va_start (ap, fmt);
   fprintf (stderr, "%s:%s():%zu:", filename, func, line);
   vfprintf (stderr, fmt, ap);
   va_end (ap);
   
}

static FILE *xp_logfile;
static int xp_handler_registered = 0;
static void xp_close (void)
{
   if (xp_logfile) fclose (xp_logfile);
   xp_logfile = NULL;
}

/**
 *
 * \brief Sets the logfile to use for XLOG() messages
 *
 * Sets the file that will be used for subsequent XLOG() messages. If
 * no file is set, then XLOG() messages are written to stderr. If the
 * filename \a filename is \a NULL, then the current logfile is closed
 * and no new logfile is opened. When the logfile is closed any further 
 * XLOG() messages are displayed on stderr.
 *
 * @param[in] filename The source filename where this function was called
 *
 * @return On success zero is returned and a side-effect occurs;
 * the file \a filename is opened
 *    for writing and all future XLOG() messages will be written to the
 *    file. On failure -1 is returned and logging goes to stderr.
 * 
 * \sa xerror_set_logfile(), XLOG(), XERROR()
 *
 * \a Example:
 *
 * \verbatim
   int main (void)
   {
      int x = 42;
      char *y = "string arg";
      float z = 98.718273;
      xerror_set_log ("logfile.txt");
      XLOG ("A Small Error using %i, %s and %f\n", x, y, z);
      // outputs to the file "logfile.txt" the following:
      // main.c:10:main():A Small Error using 42, string arg and 98.718273
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
int xerror_set_logfile (const char *filename)
{
   if (xp_logfile) fclose (xp_logfile);
   xp_logfile = NULL;
   if (!filename) return 0;

   xp_logfile = fopen (filename, "w");
   if (xp_logfile && !xp_handler_registered) {
      xp_handler_registered++;
      atexit (xp_close);
   }
   return xp_logfile ? 0 : -1;
}

/**
 *
 * \brief Logs a message to the logfile
 *
 * Under most circumstances it is not necessary to use this function directly.
 * It should only be used by invoking the XLOG() macro, so please visit
 * the documentation for XLOG(). For completeness
 * this function is documented here anyway, although you are warned against
 * using it directly.
 * 
 * @param[in] filename The source filename where this function was called
 * @param[in] func The name of the caller that called this function
 * @param[in] line The line number in the source file \a filename where this
 *  function was called
 * @param[in] fmt The format string that controls the output using 
 *    printf style format specifiers
 * @param[in] ... The arguments specified by the format strings format
 * specifiers. See the printf documentation for more information on this.
 *
 * @return Nothing. A side-effect occurrs, by printing out the log message
 *  to the previously specified logfile
 * 
 * \sa xerror_set_logfile(), XLOG(), XERROR()
 *
 * \a Example:
 *
 * \verbatim
   int main (void)
   {
      int x = 42;
      char *y = "string arg";
      float z = 98.718273;
      xerror_set_log ("logfile.txt");
      XLOG ("A Small Error using %i, %s and %f\n", x, y, z);
      // outputs to the file "logfile.txt" the following:
      // main.c:10:main():A Small Error using 42, string arg and 98.718273
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
void xerror_log (char const *filename, char const *func, size_t line, 
                  char *fmt, ...)
{
   FILE *outf = xp_logfile ? xp_logfile : stderr;
   va_list ap;
   va_start (ap, fmt);
   fprintf (outf, "%s:%s():%zu:", filename, func, line);
   vfprintf (outf, fmt, ap);
   va_end (ap);
}

/**
 *
 * \brief Flushes the logfile file
 *
 * \a xerror_flush() flushes all output to the logfile, forcing a write 
 * to the disk. This function is called automatically when the program
 * ends or when \a xerror_set_logfile() is called.
 * 
 * @return Nothing. A side-effect occurrs, by ensuring all log messages
 *  are flushed to the disk.
 * 
 * \sa xerror_set_logfile(), XLOG(), XERROR()
 *
 * \a Example:
 *
 * \verbatim
   int main (void)
   {
      int x = 42;
      char *y = "string arg";
      float z = 98.718273;
      xerror_set_log ("logfile.txt");
      XLOG ("A Small Error using %i, %s and %f\n", x, y, z);
      // outputs to the file "logfile.txt" the following:
      // main.c:10:main():A Small Error using 42, string arg and 98.718273
      xerror_flush ();
      return EXIT_SUCCESS;
   }
  \endverbatim
 */
void xerror_flush (void)
{
   FILE *outf = xp_logfile ? xp_logfile : stderr;
   fflush (outf);
}



