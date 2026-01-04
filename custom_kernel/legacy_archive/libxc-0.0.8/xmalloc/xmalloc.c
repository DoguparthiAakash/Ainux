/**
 * \file 
 *
 * \brief Extended malloc routines
 *
 * A set of functions and libraries to wrap around malloc, that will provide
 * a log of the memory allocated. Which file, function and line-number that
 * allocated and how big each allocation was.
 * 
 * xmalloc is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 *
 */

#include <stdio.h>
#include <signal.h>
#include <stdbool.h>

#include "xmalloc/xmalloc.h"


#define INC_AMOUNT         (1024)

static unsigned char xp_lflags = XMALLOC_DUMP_ALL;
static size_t xp_inc_amount = INC_AMOUNT;
static bool xp_enabled = true;

struct xstruct_t {
   const char *file;
   const char *func;
   size_t line;
   size_t size;
   void *ptr;
};

static struct xstruct_t **xstructs = NULL;

static size_t xstructs_len = 0;
static size_t num_allocations, num_frees, total_allocations;

static void xp_inc_space (void)
{
   struct xstruct_t **tmp = realloc (xstructs, (xstructs_len + xp_inc_amount)
                                             * (sizeof *tmp));
   if (!tmp) {
      return;
   }
   xstructs = tmp;
   for (size_t i=xstructs_len; i<(xstructs_len + xp_inc_amount); i++) {
      xstructs[i] = NULL;
   }
   xstructs_len += xp_inc_amount;
}

/* TODO: This can be speeded up by a factor of almost two simply
 * by checking both ends of the array at once.
 */
struct xstruct_t **xp_find_ptr (void *ptr)
{
   size_t mid = xstructs_len/2;
   if (!xstructs_len) return NULL;
   for (size_t i=0; i<mid; i++) {
      if (xstructs[i]==ptr || 
            (xstructs[i] && xstructs[i]->ptr==ptr)) {
         return &xstructs[i];
      }
      if (xstructs[i+mid]==ptr ||
            (xstructs[i+mid] && xstructs[i+mid]->ptr==ptr)) {
         return &xstructs[i+mid];
      }
   }
   return NULL;
}

/**
 * \brief Allocate memory
 *
 * Allocates the specified amount of memory. Use the macro 
 * XMALLOC (\a size )
 * to automatically have the \a func, \a line and \a file parameter
 * set to the point of invocation.
 *
 * @param[in] size The amount of memory to allocate in bytes
 *
 * @return On success, a newly allocated block of memory of at least
 * \a size bytes long. On failure \a NULL is returned. The caller must free 
 * the returned pointer on succcess.
 */
void *xmalloc (const char *file, const char *func, size_t line, size_t size)
{
   if (!xp_enabled) return malloc (size);
   struct xstruct_t **newxs = xp_find_ptr (NULL);
   if (!newxs) {
      xp_inc_space ();
   }
   newxs = xp_find_ptr (NULL);
   if (!newxs) {
      xmalloc_dump (xp_lflags);
      return NULL;
   }
   (*newxs) = malloc (sizeof **newxs);
   if (!*newxs) {
      xmalloc_dump (xp_lflags);
      return NULL;
   }
   (*newxs)->file = file;
   (*newxs)->func = func;
   (*newxs)->line = line;
   (*newxs)->size = size;
   (*newxs)->ptr = malloc (size);
   if (!(*newxs)->ptr) {
      xmalloc_dump (xp_lflags);
      return NULL;
   }
   num_allocations++;
   total_allocations += size;
   return (*newxs)->ptr;
}

/**
 * \brief Free memory allocated with \a xmalloc()
 *
 * Frees the memory previously allocated with \a xmalloc(). Use the macro 
 * XMALLOC_FREE(\a ptr ) instead of this function directly.
 *
 * @param[in] ptr The pointer to the memory to be freed
 *
 * @return Nothing. A side-effect is caused instead (freeing memory) 
 */
void xmalloc_free (void *ptr)
{
   if (!ptr) return;
   if (!xp_enabled) {
      free (ptr);
      return;
   }
   struct xstruct_t **record = xp_find_ptr (ptr);
   if (!record) {
      fprintf (stderr, "Attempt to free %p when it has not been allocated\n",
               ptr);
   } else {
      total_allocations -= (*record)->size;
      free (ptr);
      free (*record);
      *record = NULL;
      num_frees++;
   }
}

/**
 * \brief Reallocate memory
 *
 * Reallocates the memory block pointed to by \a ptr to be at least
 * \a newsize bytes long. Use the macro XMALLOC_REALLOC (\a ptr, \a newsize)
 * instead of calling this function directly.
 *
 * @param[in] ptr Pointer to the existing block of memory that was allocated
 * with xmalloc()
 *
 * @param[in] newsize The new size of the memory block in bytes
 *
 * @return On success, a reallocated block of memory of at least
 * \a newsize bytes long. On failure \a NULL is returned. The caller must free 
 * the returned pointer on succcess.
 */
void *xmalloc_realloc (const char *file, const char *func, 
                       size_t line, void *ptr, size_t newsize)
{
   if (!xp_enabled) return realloc (ptr, newsize);
   if (!ptr) return xmalloc (file, func, line, newsize);

   struct xstruct_t **tmp = xp_find_ptr (ptr);
   if (!tmp) {
      fprintf (stderr, "Attempt to realloc pointer not yet allocated\n");
      xmalloc_dump (xp_lflags);
      return NULL;
   }
   (*tmp)->ptr = realloc (ptr, newsize);
   if (!(*tmp)->ptr) {
      (*tmp)->ptr = ptr;
      fprintf (stderr, "Out of memory in reallocation\n");
      xmalloc_dump (xp_lflags);
      return NULL;
   }
   (*tmp)->size = newsize;
   (*tmp)->file = file;
   (*tmp)->func = func;
   (*tmp)->line = line;
   return (*tmp)->ptr;
}

/**
 * \brief Dump specified accounting information to stdout
 *
 * Dumps all the accounting and statistical information available
 * about xmalloc and memory usage to stdout.

 * @param[in] flags Flags to control the output, that can be logically
 * OR'ed together to print out any combination of available information:
 *    - \a XMALLOC_DUMP_STATS :  Print only aggregate statistics
 *    - \a XMALLOC_DUMP_BLOCKS :  Print only blocklist
 *    - \a XMALLOC_DUMP_SELF :  Print only statistics about xmalloc itself
 *    - \a XMALLOC_DUMP_ALL : Print all of the above
 *
 *
 * @return Nothing. A side-effect is caused instead (printing information 
 * on the screen).
 */
void xmalloc_dump (unsigned char flags)
{
   if (!xp_enabled) {
      fprintf (stderr, "xmalloc is not enabled\n");
      return;
   }
   if (flags & XMALLOC_DUMP_STATS) {
      fprintf (stderr, "XMALLOC: Statistics about memory usage\n");
      fprintf (stderr, "Number of allocations:  %zu\n",
            num_allocations);
      fprintf (stderr, "Number of free's:       %zu\n",
            num_frees);
      fprintf (stderr, "Current allocation:     %zu bytes\n",
            total_allocations);
   }

   if (flags & XMALLOC_DUMP_BLOCKS) {
      fprintf (stderr, "XMALLOC: block list\n");
      for (size_t i=0; i<xstructs_len; i++) {
         struct xstruct_t *xs = xstructs[i];
         if (xs) {
            fprintf (stderr, "%s:%s:%zu:%zu:%p\n",
                  xs->file, xs->func, xs->line, xs->size, xs->ptr);
         } else {
            fprintf (stderr, "(empty)\n");
         }
      }
   }

   if (flags & XMALLOC_DUMP_SELF) {
      fprintf (stderr, "XMALLOC: Statistics about xmalloc\n");
      fprintf (stderr, "Array length: %zu\n", xstructs_len);
      fprintf (stderr, "Array size: %zu bytes\n",
            xstructs_len * sizeof *xstructs);
   }
}

/**
 * \brief Fine-tune xmalloc behaviour
 *
 * Fine-tune the beahviour of xmalloc by setting or clearing certain
 * "knobs" in the library at runtime. NOTE WELL: The behaviour of
 * xmalloc changes drastically in response to the knobs \a at \a runtime, 
 * so only set or clear an option/knob at safe points in your program.
 *
 * Knobs and values for them are as follows:
 *    - \a XMALLOC_KNOB_FLAGS : The default flags that xmalloc will use to print out information when an exception is detected. Flags  are described in \a xmalloc_dump()
 *    - \a XMALLOC_KNOB_INCREMENT : The amount to increment each allocation * by. The best value is to use what you consider to be the average size * of your allocations throughout the runtime of the program.
 *    - \a XMALLOC_KNOB_ENABLE : 
 *       -# Turn on xmalloc functionality
 *       -# Turn off xmalloc functionality (xmalloc then acts as a thin
 *          wrapper around standard malloc)
 *
 * @param[in] knob The knob to modify (see explanation above)
 * @param[in] value The new value that the knob would take (see explanation
 *    above)
 *
 * @return Nothing. Various side-effects result from the invocation of this
 * function. See above for more information on the side-effects.
 */
void xmalloc_set_knob (int knob, size_t value)
{
   switch (knob) {
      case XMALLOC_KNOB_FLAGS:
         xp_lflags = (unsigned char)value; break;
      case XMALLOC_KNOB_INCREMENT:
         xp_inc_amount = value; break;
      case XMALLOC_KNOB_ENABLE:
         xp_enabled = value ? true : false; break;
      default:
         fprintf (stderr, "%s: unknown knob %i specified\n", __func__, knob);
   }
}

/**
 * \brief Finds the accounting record for pointer \a ptr
 *
 * Locates and returns the accounting record for pointer \a ptr.
 *
 * @param[in] ptr The pointer to search for
 *
 * @return On success, the accounting record for the given pointer is returned.
 * On failure or if the record cannot be found, \a NULL is returned.
 */
void *xmalloc_find (void *ptr)
{
   if (!xp_enabled) {
      fprintf (stderr, "xmalloc is not enabled\n");
      return NULL;
   }
   struct xstruct_t **ret = xp_find_ptr (ptr);
   if (ret) {
      return (*ret)->ptr;
   } else {
      return NULL;
   }
}

/**
 * \brief Resets all the xmalloc internal accounting structures.
 *
 * Resets all the internal xmalloc  accounting and recording structures,
 * and frees all the memory that xmalloc uses to maintain these records.
 * The actual allocated memory is unchanged and unaffected.
 *
 * @return Nothing. A side-effect is caused; all the memory used internally
 * by xmalloc is freed. No other side-effect is implemented.
 * 
 */
void xmalloc_reset (void)
{
   free (xstructs); xstructs = NULL;
   xstructs_len = 0;
   num_allocations = num_frees = total_allocations = 0;
}
