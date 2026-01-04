
#include <stdio.h>
#include <stdlib.h>

#include "xtree/xtree.h"


static void s_print_tree (void *number, void *level)
{
   int num = (int)number;
   int lev = (int)level;
   printf ("%*s = %i\n", lev * 3, " ", num);
}

static void *s_double_tree (void *number, void *extra)
{
   int num = (int)number;
   extra = extra;
   return (void *)(num * 2);
}

int main (void)
{
   xtree_t *tree1 = xtree_add_child (NULL, (char *)-3007);

   if (!tree1) {
      fprintf (stderr, "Unable to add root node, aborting\n");
      return EXIT_FAILURE;
   }

   for (size_t i=0; i<3; i++) {
      xtree_t *tmp = xtree_add_child (tree1, (char *)i);
      if (!tmp) {
         fprintf (stderr, "Unable to add layer-1, %i\n", i);
         goto error;
      }
   }
   for (size_t i=0; i<3; i++) {
      xtree_t *child_tree = xtree_get_child (tree1, i);
      if (!child_tree) {
         fprintf (stderr, "Unable to find child number %i\n", i);
         goto error;
      }
      for (size_t j=10; j<13; j++) {
         xtree_t *tmp = xtree_add_child (child_tree, (char *)j);
         if (!tmp) {
            fprintf (stderr, "Unable to add to child tree - %i, %i\n", i, j);
            goto error;
         }
      }
   }
   xtree_apply (tree1, s_print_tree, 0);
   xtree_t *tree2 = xtree_map (tree1, s_double_tree, NULL);
   xtree_apply (tree2, s_print_tree, 0);
   printf ("Toplevel result of second tree = %i\n", 
            (int)xtree_get_payload (tree2));
   xtree_del (tree1);
   xtree_del (tree2);
   return EXIT_SUCCESS;
error:
   if (tree1) {
      xtree_del (tree1);
   }
   return EXIT_FAILURE;
}
