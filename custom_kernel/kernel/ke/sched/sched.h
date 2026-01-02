#ifndef SCHED_H
#define SCHED_H

#include <stdint.h>

/* Forward declarations */
struct file_descriptor;
struct vmm_address_space;
struct signal_pending;

/* Process State */
#define TASK_RUNNING  0
#define TASK_READY    1
#define TASK_BLOCKED  2
#define TASK_STOPPED  3
#define TASK_ZOMBIE   4
#define TASK_KERNEL   0x00
#define TASK_USER     0x01
#define TASK_MAX_FDS  64

/* Process Control Block */
struct task_struct {
    /* Scheduler state (must be first for context switch) */
    uint64_t rsp;                  /* Saved kernel stack pointer */
    uint64_t rsp0;                 /* Ring 0 stack pointer for TSS */
    
    /* Process identification */
    uint64_t id;                   /* Process ID (PID) */
    struct task_struct *parent;    /* Parent Task Pointer (Updated type) */
    uint64_t parent_id;            /* Parent PID (Keep for now) */
    char name[32];                 /* Process name */
    
    /* Scheduling */
    struct task_struct *next;      /* Linked list for scheduler */
    volatile int state;            /* Current state */
    int flags;                     /* TASK_KERNEL or TASK_USER */
    int priority;                  /* Scheduling priority */
    uint64_t time_slice;           /* Remaining time slice */
    
    /* Memory */
    struct vmm_address_space *address_space; /* Virtual address space */
    uint64_t brk;                  /* Program break for heap */
    
    /* User-mode context */
    uint64_t user_rsp;             /* User stack pointer */
    uint64_t user_rip;             /* User instruction pointer */
    
    /* File descriptors */
    struct file_descriptor *fd_table[TASK_MAX_FDS];  /* File descriptor table */
    
    /* Signal handling */
    struct signal_pending *signals; /* Signal state */
    
    /* Exit status */
    /* Exit status */
    int exit_code;
    
    /* Windowing Support (Virtual Layer) */
    void *output_window; /* (Window*) - void* to avoid circular dep */
};

/* Global task pointers */
extern struct task_struct *current_task;
extern struct task_struct *task_head;

/* Initialization */
void sched_init(void);

/* Task creation */
struct task_struct *sched_create_task(void (*entry_point)(void));
struct task_struct *sched_create_user_task(uint64_t entry, uint64_t stack, 
                                            struct vmm_address_space *space);

/* Task management */
void sched_yield(void);
void sched_exit(int code);
void sched_block(void);
void sched_unblock(struct task_struct *task);
int sched_gc(void);

/* Get current task */
struct task_struct *sched_get_current(void);

/* Get task by PID */
struct task_struct *sched_get_by_pid(uint64_t pid);

/* Switch to user mode */
void sched_switch_to_user(uint64_t entry, uint64_t stack);

/* ISR handlers */
void schedule_handler(void);
uint64_t timer_isr_handler_c(uint64_t rsp);

#endif
