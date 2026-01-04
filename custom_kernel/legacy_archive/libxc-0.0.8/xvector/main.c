
#include <stdio.h>
#include <stdlib.h>

#include "xvector/xvector.h"

typedef struct {
   int i;
   float f;
} eg_t;

int pred (void *o, void *e)
{
   eg_t *lhs=o, *rhs=e;
   if (!rhs) return -1;
   if (lhs->i < rhs->i) return 1;
   if (lhs->i > rhs->i) return -1;
   return 0;
}

void *mapfunc (void *array_element, void *arg)
{
   eg_t *ae = array_element, *a = arg, *ret = malloc (sizeof *ret);
   if (!ret) return NULL;
   if (!ae || !a) {
      free (ret);
      return NULL;
   }
   ret->i = ae->i + a->i;
   ret->f = ae->f - a->f;
   printf ("Returning %i // %f :: %p : %i // %f\n", 
         a->i, a->f,
         (void *)ret, ret->i, ret->f);
   return ret;
}

int main (void)
{
#define SIZEOF(array)      (sizeof array / sizeof array[0])
   printf ("testing xvector version %s\n", H_XVECTOR);
   { // Store as float, retrieve as float
      float sfar[] = {
         1.1, 2.2, 3.3, 4.4, 5.5, 6.6, 7.7, 8.8, 9.9,
      };
      xvector_t *tfar = NULL;
      for (size_t i=0; i<SIZEOF(sfar); i++) {
         xvector_t *tmp = xvector_ins_tail (tfar, &sfar[i]);
         if (tmp) tfar = tmp;
         else fprintf (stderr, "Error inserting %zu/%f\n", i, sfar[i]);
      }
      for (size_t i=0; i<XVECT_LENGTH(tfar); i++) {
         fprintf (stderr, "=> %f\n", XVECT_DEREF (tfar, i, float));
      }
      xvector_free (tfar);
   }
   { // Store as struct, retrieve as struct
      eg_t eg[] = {
         {9, 0.1},
         {8, 1.2},
         {7, 2.3},
         {6, 3.4},
         {5, 4.5},
         {4, 5.6},
         {3, 6.7},
         {2, 7.8},
         {1, 8.9},
         {0, 9.0},
      };
      xvector_t *teg = NULL;
      for (size_t i=0; i<SIZEOF(eg); i++) {
         xvector_t *tmp = xvector_ins_tail (teg, &eg[i]);
         if (tmp) teg = tmp;
         else fprintf (stderr, "%zu/%i/%f error\n", i, eg[i].i, eg[i].f);
      }
      for (size_t i=0; i<XVECT_LENGTH(teg); i++) {
         eg_t *tmp = XVECT_INDEX(teg, i);
         fprintf (stderr, "%i // %f\n", tmp->i, tmp->f);
      }
      eg_t ttmp = { 2, 5.0};
      xvector_t *sub = xvector_map (teg, mapfunc, &ttmp);

      // Make a copy of it and see if it works
      void **nativea = xvector_native (teg);
      for (size_t i=0; nativea[i]; i++) {
         eg_t *tmp = nativea[i];
         fprintf (stderr, "native: %i // %f\n", tmp->i, tmp->f);
      }
      free (nativea);

      XVECT_INDEX (teg, 5) = NULL;
      for (size_t i=0; i<XVECT_LENGTH(teg); i++) {
         eg_t *tmp = XVECT_INDEX(teg, i);
         fprintf (stderr, "%p / ", (void *)tmp);
         tmp = XVECT_INDEX(sub, i);
         fprintf (stderr, "%p : ", (void *)tmp);
         fprintf (stderr, "%i // %f\n", tmp->i, tmp->f);
      }
      eg_t tmp = { 6, 3.4 };
      void **m = xvector_find_first (teg, &tmp, pred);
      eg_t *match = *m;
      fprintf (stderr, "Found -> %i/%f\n", match->i, match->f);
      tmp.i++;
      m = xvector_find_last (teg, &tmp, pred);
      match = *m;
      fprintf (stderr, "Found -> %i/%f\n", match->i, match->f);
      xvector_free (teg);
      xvector_iterate (sub, free);
      xvector_free (sub);
   }
   {  // Test the insertion/deletion of elements from the middle of
      // array
      xvector_t *array = NULL;
      float sfar[] = {
         1.1, 2.2, 3.3, 4.4, 5.5, 6.6, 7.7, 8.8, 9.9,
      };
      for (size_t i=0; i<(sizeof sfar / sizeof sfar[0]) * 4; i++) {
         xvector_t *tmp =
            xvector_ins_tail (array, &sfar[i%(sizeof sfar/sizeof sfar[0])]);
         if (tmp) {
            array = tmp;
         } else {
            fprintf (stderr, "Out of memory inserting elements into array\n");
            xvector_free (array); array = NULL;
            break;
         }
      }
      for (size_t i=0; i<XVECT_LENGTH (array); i++) {
         fprintf (stderr, "a-%zu: %f\n", i, *(float *)XVECT_INDEX (array, i));
      }
      for (size_t i=0; i<XVECT_LENGTH (array); i++) {
         if (!(i%3)) {
            float *tmp = xvector_del (array, i);
            if (tmp) {
               fprintf (stderr, "Removed %f\n", *tmp);
            } else {
               fprintf (stderr, "Unable to delete item %zu\n", i);
               free (array); array = NULL;
               break;
            }
            for (size_t j=0; j<XVECT_LENGTH (array); j++) {
               fprintf (stderr, "a-%zu: %f\n", j,
                  *(float *)XVECT_INDEX (array, j));
            }
         }
      }
      fprintf (stderr, "--------------------\n");
      for (size_t i=0; i<sizeof sfar / sizeof sfar[0]; i++) {
         xvector_t *tmp = xvector_insert (array, i*2, &sfar[i]);
         if (tmp) {
            fprintf (stderr, "Inserted %zu:%f\n", i*2, sfar[i]);
            array = tmp;
         } else {
            fprintf (stderr, "Could not insert %f\n", sfar[i]);
            xvector_free (array); array = NULL;
            break;
         }
      }
      for (size_t i=0; i<XVECT_LENGTH (array); i++) {
         fprintf (stderr, "a-%zu: %f\n", i, *(float *)XVECT_INDEX (array, i));
      }
      xvector_free (array);
   }
   {
      // Test xvector_join
      static int sa1[] = {9, 8, 7, 6, 5, 4, 3, 2, 1, 0},
                 sa2[] = {200, 300, 400, 500, 600, 700};
      xvector_t *xa1 = NULL, *xa2 = NULL, *xaboth = NULL;
      for (size_t i=0; i<sizeof sa1/sizeof sa1[0]; i++) {
         xvector_t *tmp = xvector_ins_tail (xa1, &sa1[i]);
         if (!tmp) {
            fprintf (stderr, "Could not create xa1\n");
            return -1;
         }
         xa1 = tmp;
      }
      for (size_t i=0; i<sizeof sa2/sizeof sa2[0]; i++) {
         xvector_t *tmp = xvector_ins_tail (xa2, &sa2[i]);
         if (!tmp) {
            fprintf (stderr, "Could not create xa2\n");
            return -1;
         }
         xa2 = tmp;
      }
      // Join the two arrays
      xaboth = xvector_join (xa1, xa2);
      for (size_t i=0; i<XVECT_LENGTH (xaboth); i++) {
         int value = XVECT_DEREF (xaboth, i, int);
         printf ("joined: %zu/ - %i\n", i, value);
      }
      xvector_free (xa1);
      xvector_free (xa2);
      xvector_free (xaboth);
   }
   return EXIT_SUCCESS;
#undef SIZEOF
}
