#include "log.h"
#include <stdint.h>
#include "libc/string.h"

/* Simple external kprint definition to avoid circular dependency if needed, 
   but klog_dump needs to print to screen. */
extern void kprint(const char *msg); 

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
