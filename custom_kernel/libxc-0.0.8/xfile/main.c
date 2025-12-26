#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <signal.h>

#define LOCKFILE     ("test.lock")

#include "xfile/xfile.h"

volatile int stop_copy = 0;

void signal_h (int n)
{
   if (n==SIGINT) stop_copy = 1;
}

int progress_cb (uint64_t copied)
{
   printf ("\rBytes copied:  %llu", copied);
   return stop_copy;
}

int main (void)
{
   printf ("Testing xfile version %s\n", H_XFILE);
   printf ("Using directory separator: '%s'\n", DIRSEP);
   
   {
      // xvector_t *files = xfile_find ("..", "c.*\\.[ch]$", "rfde");
      xvector_t *files = xfile_find ("..", "sp.*\\.[ch]$", "rfde");
      printf ("Files = %p\n", files);
      for (size_t i=0; i<XVECT_LENGTH (files); i++) {
         xfile_stat_t *f = XVECT_INDEX (files, i);
         char *s_type = NULL;
         switch (f->type) {
            case stat_DIR:    s_type = "dir";   break;
            case stat_REG:    s_type = "reg";   break;
            case stat_OTHER:  s_type = "other"; break;
         }
         printf ("Found %s: '%s'\n", s_type, f->name);
         free (f->name);
         free (f);
      }
      xvector_free (files);
   }

   {
      size_t d1 = xfile_rm ("one/two/three/3", NULL);
      printf ("Deleted %zu files\n", d1);
      size_t d2 = xfile_rm ("one", "r");
      printf ("Deleted %zu files\n", d2);
   }

   FILE *test = xfile_atomic (".", LOCKFILE);
   if (!test) {
      printf ("Unable to lock\n");
   } else {
      fprintf (test, "Locked file %s\n", LOCKFILE);
      fclose (test);
   }
   printf ("%zu\n", (size_t)-1);

   signal (SIGINT, signal_h);
   xfile_copy ("testfile", "testfile.new", 1024, progress_cb);
   printf ("\n");
   FILE *tmp = xfile_open_with_backup ("testfile.new", "bak");
   if (!tmp) {
      printf ("Could not open testfile.new for writing: %m\n");
   } else {
      fprintf (tmp, "written\n");
      fclose (tmp);
   }
   printf ("Trying xfile_mkdir()\n");
   if (xfile_mkdir ("/tmp/one/two/three", "p")!=true) {
      printf ("Could not create /tmp/one/two/three: %m\n");
   }
   return EXIT_SUCCESS;
}

