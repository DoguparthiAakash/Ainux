#include "log.h"
#include <stdint.h>
#include "libc/string.h"

/* Simple external kprint definition to avoid circular dependency if needed, 
   but klog_dump needs to print to screen. */
extern void kprint(const char *msg); 

void kprint_color(int color, const char *msg) {
    const char *ansi_code = "";
    switch (color) {
        case KLOG_COLOR_RED:    ansi_code = "\033[1;31m"; break;
        case KLOG_COLOR_GREEN:  ansi_code = "\033[1;32m"; break;
        case KLOG_COLOR_YELLOW: ansi_code = "\033[1;33m"; break;
        case KLOG_COLOR_BLUE:   ansi_code = "\033[1;34m"; break;
        case KLOG_COLOR_CYAN:   ansi_code = "\033[1;36m"; break;
        default: ansi_code = ""; break;
    }

    if (*ansi_code) kprint(ansi_code);
    kprint(msg);
    if (*ansi_code) kprint("\033[0m");
} 

#define LOG_SIZE 4096
static char log_buffer[LOG_SIZE];
static uint64_t log_head = 0;
static int log_wrapped = 0;

void klog_init(void) {
    memset(log_buffer, 0, LOG_SIZE);
    log_head = 0;
    log_wrapped = 0;
}

void klog_write(const char *msg) {
    while (*msg) {
        // struct task_struct *curr = sched_get_current();
        // if (curr && curr->output_window) {
        //     // wm_console_write((Window*)curr->output_window, msg); // DEBUG: Disabled
        //     // return; /* Redirected to Window, skip Global FB */
        // }
        log_buffer[log_head++] = *msg++;
        if (log_head >= LOG_SIZE) {
            log_head = 0;
            log_wrapped = 1;
        }
    }
}

void klog_dump(void) {
    kprint("\n=== Kernel Message Buffer (dmesg) ===\n");
    
    if (log_wrapped) {
        /* Print from head to end, then 0 to head */
        for (uint64_t i = log_head; i < LOG_SIZE; i++) {
             char c[2] = { log_buffer[i], 0 };
             if (c[0]) kprint(c);
        }
    }
    
    for (uint64_t i = 0; i < log_head; i++) {
        char c[2] = { log_buffer[i], 0 };
        if (c[0]) kprint(c);
    }
    kprint("\n=== End of Log ===\n");
}
