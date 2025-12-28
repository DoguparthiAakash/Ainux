#ifndef SCHED_H
#define SCHED_H

#include <stdint.h>

// Process State
#define TASK_RUNNING 0
#define TASK_READY   1
#define TASK_BLOCKED 2

struct task_struct {
    uint64_t rsp;          // Saved Stack Pointer
    uint64_t id;           // Process ID
    struct task_struct *next; // Linked list
    int state;
};

void sched_init(void);
struct task_struct* sched_create_task(void (*entry_point)(void));
void schedule_handler(void); // Called from assembly stub
uint64_t timer_isr_handler_c(uint64_t rsp); // Called from assembly stub

#endif
