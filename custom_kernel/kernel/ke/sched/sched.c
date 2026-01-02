#include "sched.h"
#include "signal.h"
#include "mm/heap.h"
#include "mm/pmm.h"
#include "mm/vmm.h"
#include "log.h"
#include "drivers/timer.h"
#include "hal/x86_64/gdt.h"
#include "libc/string.h"

/* Global Scheduler State */
struct task_struct *current_task = 0;
struct task_struct *task_head = 0;
static uint64_t next_task_id = 1;

/* Assembly stub defined at the bottom */
extern void timer_isr_stub(void);

void sched_init(void) {
    /* Create task for the running kernel process (PID 0) */
    current_task = (struct task_struct *)kmalloc(sizeof(struct task_struct));
    if (!current_task) {
        kprint("[SCHED] Failed to allocate init task!\n");
        return;
    }
    
    memset(current_task, 0, sizeof(struct task_struct));
    current_task->id = 0;
    current_task->parent_id = 0;
    current_task->state = TASK_RUNNING;
    current_task->flags = TASK_KERNEL;
    current_task->next = current_task; /* Circular list */
    current_task->rsp = 0; /* Will be set on first switch */
    current_task->address_space = vmm_get_kernel_address_space();
    strcpy(current_task->name, "kernel_init");

    task_head = current_task;
    kprint("[SCHED] Initialized. PID 0 created.\n");
}

struct task_struct *sched_get_current(void) {
    return current_task;
}

struct task_struct *sched_get_by_pid(uint64_t pid) {
    struct task_struct *task = task_head;
    do {
        if (task->id == pid) return task;
        task = task->next;
    } while (task != task_head);
    return 0;
}

struct task_struct *sched_create_task(void (*entry_point)(void)) {
    struct task_struct *new_task = (struct task_struct *)kmalloc(sizeof(struct task_struct));
    if (!new_task) return 0;
    
    memset(new_task, 0, sizeof(struct task_struct));
    
    /* Allocate 4KB kernel stack */
    void *stack_base = kmalloc(4096);
    if (!stack_base) {
        kfree(new_task);
        return 0;
    }
    uint64_t *stack_top = (uint64_t *)((uint8_t *)stack_base + 4096);
    uint64_t *stack = stack_top;

    /* Hardware frame (pushed by CPU on interrupt) */
    /* Order: SS, RSP, RFLAGS, CS, RIP */
    stack--; *stack = GDT_KERNEL_DATA;   /* SS */
    stack--; *stack = (uint64_t)stack_top; /* RSP */
    stack--; *stack = 0x202;  /* RFLAGS (Interrupts Enabled) */
    stack--; *stack = GDT_KERNEL_CODE;   /* CS */
    stack--; *stack = (uint64_t)entry_point; /* RIP */

    /* Software frame (GPRs) - 15 registers */
    for (int i = 0; i < 15; i++) {
        stack--;
        *stack = 0;
    }

    new_task->rsp = (uint64_t)stack;
    new_task->rsp0 = (uint64_t)stack_top;
    new_task->id = next_task_id++;
    new_task->parent_id = current_task ? current_task->id : 0;
    new_task->state = TASK_READY;
    new_task->flags = TASK_KERNEL;
    new_task->address_space = vmm_get_kernel_address_space();
    new_task->next = task_head->next;
    strcpy(new_task->name, "kernel_task");
    
    /* Insert into circular list */
    task_head->next = new_task;
    
    return new_task;
}

/* Create a user-mode task */
struct task_struct *sched_create_user_task(uint64_t entry, uint64_t user_stack,
                                            struct vmm_address_space *space) {
    struct task_struct *new_task = (struct task_struct *)kmalloc(sizeof(struct task_struct));
    if (!new_task) return 0;
    
    memset(new_task, 0, sizeof(struct task_struct));
    
    /* Allocate kernel stack for syscalls/interrupts */
    void *kernel_stack_base = kmalloc(8192); /* 8KB kernel stack */
    if (!kernel_stack_base) {
        kfree(new_task);
        return 0;
    }
    uint64_t kernel_stack_top = (uint64_t)kernel_stack_base + 8192;
    uint64_t *stack = (uint64_t *)kernel_stack_top;
    
    /* Set up initial kernel stack for iretq to user mode */
    /* Hardware frame for returning to Ring 3 */
    stack--; *stack = GDT_USER_DATA_RPL3;  /* SS (Ring 3 data) */
    stack--; *stack = user_stack;          /* RSP (User stack) */
    stack--; *stack = 0x202;               /* RFLAGS (IF=1) */
    stack--; *stack = GDT_USER_CODE_RPL3;  /* CS (Ring 3 code) */
    stack--; *stack = entry;               /* RIP (Entry point) */
    
    /* Software frame (GPRs) - 15 registers */
    for (int i = 0; i < 15; i++) {
        stack--;
        *stack = 0;
    }
    
    new_task->rsp = (uint64_t)stack;
    new_task->rsp0 = kernel_stack_top;
    new_task->user_rsp = user_stack;
    new_task->user_rip = entry;
    new_task->id = next_task_id++;
    new_task->parent_id = current_task ? current_task->id : 0;
    new_task->state = TASK_READY;
    new_task->flags = TASK_USER;
    new_task->address_space = space;
    new_task->next = task_head->next;
    strcpy(new_task->name, "user_task");
    
    /* Allocate signal state */
    new_task->signals = signal_alloc();
    
    /* Insert into circular list */
    /* Insert into circular list */
    task_head->next = new_task;
    
    // kprint("[SCHED] Created user task PID "); /* Original commented out or kept? */
    /* Let's keep the PID print logic as it was originally there or restored? */
    /* The original code had PID printing. I modified it to "User Task Inserted". */
    /* I should restore original PID printing logic or just clean up my debugs. */
    /* Restoring original logic is better. */
    
    kprint("[SCHED] Created user task PID ");
    char buf[16];
    int idx = 0;
    uint64_t n = new_task->id;
    if (n == 0) buf[idx++] = '0';
    else {
        while (n > 0) { buf[idx++] = '0' + (n % 10); n /= 10; }
    }
    for (int i = idx - 1; i >= 0; i--) {
        char c[2] = {buf[i], 0};
        kprint(c);
    }
    kprint("\n");
    
    return new_task;
}

void sched_yield(void) {
    /* Trigger a reschedule */
    __asm__ volatile("hlt");
}

void sched_exit(int code) {
    if (!current_task) return;
    
    current_task->exit_code = code;
    current_task->state = TASK_ZOMBIE;
    
    // kprint("[SCHED] Task exited (Zombie). Yielding...\n");
    // kprint("[SCHED] Task exited (Zombie). Yielding... Ptr: ");
    /* Print Pointer Hex */
    // char hex[]="0123456789ABCDEF"; uint64_t v=(uint64_t)current_task; char s[19]; s[0]='0'; s[1]='x';
    // for(int i=0;i<16;i++){ s[17-i] = hex[v&0xF]; v>>=4; } s[18]=0;
    // kprint(s);
    // kprint("\n");
    
    /* Ensure interrupts are enabled so Timer can fire and switch us out */
    __asm__ volatile("sti");
    
    sched_yield();
    
    /* Should not return, but just in case */
    while (1) {
        __asm__ volatile("hlt");
    }
}

void sched_block(void) {
    if (!current_task) return;
    current_task->state = TASK_BLOCKED;
    sched_yield();
}

void sched_unblock(struct task_struct *task) {
    if (!task) return;
    if (task->state == TASK_BLOCKED) {
        task->state = TASK_READY;
    }
}

/* C Handler called by Assembly Stub */
uint64_t timer_isr_handler_c(uint64_t rsp) {
    /* kprint("."); */
    timer_handler_callback();

    if (current_task) {
        current_task->rsp = rsp;
    }

    struct task_struct *next = current_task->next;
    /* Find next runnable */
    while (next->state != TASK_READY && next->state != TASK_RUNNING) {
        if (next == current_task) break; 
        next = next->next;
    }
    
    /* Debug: Trace Switch if changing task */
    if (next != current_task) {
        // kprint("[SCHED] Sw: ");
        /* ... snip ... */
    }
    
    if (current_task->state == TASK_RUNNING) {
        current_task->state = TASK_READY;
    }
    
    current_task = next;
    
    if (current_task->state == TASK_READY) {
        current_task->state = TASK_RUNNING;
    }
    
    if (current_task->rsp0) {
        tss_set_rsp0(current_task->rsp0);
    }
    
    if (current_task->address_space) {
        /* Only switch if different to avoid TLB flush overhead? */
        /* For debug, always switch to be safe? */
        vmm_switch_address_space(current_task->address_space);
    }

    return current_task->rsp;
}

/* Assembly Stub for timer interrupt */
__asm__(
    ".global timer_isr_stub\n"
    "timer_isr_stub:\n"
    "    cli\n"                 /* Ensure interrupts disabled */
    "    push %rax\n"
    "    push %rbx\n"
    "    push %rcx\n"
    "    push %rdx\n"
    "    push %rsi\n"
    "    push %rdi\n"
    "    push %rbp\n"
    "    push %r8\n"
    "    push %r9\n"
    "    push %r10\n"
    "    push %r11\n"
    "    push %r12\n"
    "    push %r13\n"
    "    push %r14\n"
    "    push %r15\n"
    
    "    mov %rsp, %rdi\n"      /* Pass RSP as 1st arg */
    "    call timer_isr_handler_c\n"
    "    mov %rax, %rsp\n"      /* Restore RSP (potentially different) */
    
    "    mov $0x20, %al\n"
    "    outb %al, $0x20\n"     /* Send EOI to PIC (Master) */
    
    "    pop %r15\n"
    "    pop %r14\n"
    "    pop %r13\n"
    "    pop %r12\n"
    "    pop %r11\n"
    "    pop %r10\n"
    "    pop %r9\n"
    "    pop %r8\n"
    "    pop %rbp\n"
    "    pop %rdi\n"
    "    pop %rsi\n"
    "    pop %rdx\n"
    "    pop %rcx\n"
    "    pop %rbx\n"
    "    pop %rax\n"
    
    "    sti\n"
    "    iretq\n"
);

/* Switch to user mode (used for execve-like functionality) */
void sched_switch_to_user(uint64_t entry, uint64_t stack) {
    /* Set up TSS for return from user mode */
    tss_set_rsp0(current_task->rsp0);
    
    /* Switch to user address space */
    if (current_task->address_space) {
        vmm_switch_address_space(current_task->address_space);
    }
    
    /* Jump to user mode using iretq */
    __asm__ volatile(
        "cli\n"
        "mov %0, %%rax\n"       /* User stack */
        "mov %1, %%rbx\n"       /* Entry point */
        "mov %2, %%rcx\n"       /* User data segment */
        "mov %3, %%rdx\n"       /* User code segment */
        
        "push %%rcx\n"          /* SS */
        "push %%rax\n"          /* RSP */
        "pushf\n"               /* RFLAGS */
        "pop %%rax\n"
        "or $0x200, %%rax\n"    /* Enable IF */
        "push %%rax\n"
        "push %%rdx\n"          /* CS */
        "push %%rbx\n"          /* RIP */
        
        "xor %%rax, %%rax\n"    /* Clear registers */
        "xor %%rbx, %%rbx\n"
        "xor %%rcx, %%rcx\n"
        "xor %%rdx, %%rdx\n"
        "xor %%rsi, %%rsi\n"
        "xor %%rdi, %%rdi\n"
        "xor %%rbp, %%rbp\n"
        "xor %%r8, %%r8\n"
        "xor %%r9, %%r9\n"
        "xor %%r10, %%r10\n"
        "xor %%r11, %%r11\n"
        "xor %%r12, %%r12\n"
        "xor %%r13, %%r13\n"
        "xor %%r14, %%r14\n"
        "xor %%r15, %%r15\n"
        
        "iretq\n"
        : 
        : "r"(stack), "r"(entry), 
          "r"((uint64_t)GDT_USER_DATA_RPL3), "r"((uint64_t)GDT_USER_CODE_RPL3)
        : "rax", "rbx", "rcx", "rdx"
    );
    
    /* Should never reach here */
    __builtin_unreachable();
}

/* Garbage Collector */
int sched_gc(void) {
    if (!task_head) return 0;
    
    struct task_struct *prev = NULL;
    int reclaimed = 0;
    
    /* Find tail first to set prev correctly for circular list */
    struct task_struct *scan = task_head;
    while (scan->next != task_head) {
        scan = scan->next;
    }
    prev = scan;
    
    /* Iterate list once */
    /* Caution: If we delete head, we must update task_head */
    
    /* We need to handle the loop carefully because modifying the list */
    /* Simplest approach: Scan linearly, if Zombie, remove and restart loop or carefully step? */
    /* Restarting loop is O(N^2) but safe. Given N < 100 usually, it's fine. */
    
    int found_zombie = 1;
    while (found_zombie) {
        found_zombie = 0;
        
        prev = NULL;
        
        /* Find prev of head again */
        scan = task_head;
        while (scan->next != task_head) {
            scan = scan->next;
        }
        prev = scan;
        
        struct task_struct *initial_head = task_head;
        struct task_struct *iter = task_head;
        
        do {
            if (iter->state == TASK_ZOMBIE) {
                /* REAP */
                struct task_struct *zombie = iter;
                
                /* Unlink */
                prev->next = zombie->next;
                if (zombie == task_head) {
                    task_head = zombie->next;
                }
                
                /* Free Resources */
                if (zombie->address_space && zombie->address_space != vmm_get_kernel_address_space()) {
                    vmm_destroy_address_space(zombie->address_space);
                }
                
                /* Free Stacks */
                /* Stacks stored in rsp0 or calculated? */
                /* User tasks: rsp0 is top of 8KB buffer. Base is rsp0 - 8192 */
                /* Kernel tasks: rsp0 is top of 4KB buffer. Base is rsp0 - 4096 */
                
                /* We allocated with kmalloc. */
                if (zombie->flags & TASK_USER) {
                     kfree((void*)(zombie->rsp0 - 8192));
                } else {
                     /* Kernel task stack */
                     /* Only free if dynamic? IDLE task is static? */
                     /* For now assum created via sched_create_task */
                     if (zombie->rsp0) kfree((void*)(zombie->rsp0 - 4096));
                }
                
                kfree(zombie);
                reclaimed++;
                found_zombie = 1;
                break; /* Restart loop to avoid pointer hazards */
            }
            
            prev = iter;
            iter = iter->next;
        } while (iter != initial_head && iter != task_head);
    }
    
    if (reclaimed > 0) {
        kprint("[GC] Garbage Collection Finished. Reclaimed ");
        /* Print int logic */
        char buf[16];
        int idx = 0;
        if (reclaimed == 0) buf[idx++] = '0';
        else { int n = reclaimed; while (n > 0) { buf[idx++] = '0' + (n % 10); n /= 10; } }
        for (int i = idx - 1; i >= 0; i--) { char c[2]={buf[i],0}; kprint(c); }
        kprint(" tasks.\n");
    }
    
    return reclaimed;
}
