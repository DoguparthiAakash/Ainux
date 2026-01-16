#ifndef _ASSERT_H
#define _ASSERT_H

extern void kprint(const char *msg);
extern void term_clear(void);

#ifdef NDEBUG
#define assert(ignore) ((void)0)
#else
#define assert(expression) \
    if (!(expression)) { \
        kprint("\n\nASSERTION FAILED: " #expression "\n"); \
        kprint("File: " __FILE__ "\n"); \
        kprint("Line: "); \
        /* print_num macro/func not avail here directly without recursion or stdio dependency hell */ \
        /* Just hanging for now */ \
        kprint("\nSystem Halted.\n"); \
        for(;;); \
    }
#endif

#endif
