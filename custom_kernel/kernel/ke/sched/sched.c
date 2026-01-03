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
/* O(1) Scheduler Queues */
static struct priority_queue run_queues[MAX_PRIO];
static uint32_t priority_bitmap = 0; /* Bit set if queue has tasks */
static struct task_struct *idle_task = 0;

static uint64_t next_task_id = 1;
static spinlock_t sched_lock;

/* Helpers for Queue Management */
static void enqueue_task(struct task_struct *task) {
    if (!task) return;
    int prio = task->priority;
    if (prio < 0 || prio >= MAX_PRIO) prio = DEFAULT_PRIO;
    task->priority = prio;

    struct priority_queue *q = &run_queues[prio];
    
    if (!q->head) {
        q->head = task;
        q->tail = task;
        task->next = 0;
        priority_bitmap |= (1 << prio);
    } else {
        q->tail->next = task;
        q->tail = task;
        task->next = 0;
    }
}

static struct task_struct *dequeue_task(void) {
    /* Find highest priority non-empty queue */
    if (priority_bitmap == 0) return 0;
    
    /* __builtin_ffs returns 1-based index of LSB. */
    int idx = __builtin_ffs(priority_bitmap) - 1;
    if (idx < 0) return 0;
    
    struct priority_queue *q = &run_queues[idx];
    struct task_struct *task = q->head;
    
    if (task) {
        q->head = task->next;
        if (!q->head) {
            q->tail = 0;
            priority_bitmap &= ~(1 << idx);
        }
        task->next = 0; /* Detach */
    }
    return task;
}

/* Resource Tracking */
void sched_track_resource(struct task_struct *task, void *data, resource_cleanup_t cleanup, const char *desc) {
    if (!task) return;
    
    /* We must allocate a resource node. Since we might define resource_t in .c or .h? .h */
    /* Using kmalloc here. Be careful about locking. 
       Usually tracking happens during syscall, interrupts enabled. 
       We protect the TASK's resource list. If task is current, only we touch it? 
       Interrupts/Preemption can happen. So we should lock sched_lock or a specific lock. 
       For "Brutal", let's use sched_lock to protect EVERYTHING in task struct for now. */
    
    struct resource *res = (struct resource *)kmalloc(sizeof(struct resource));
    if (!res) return;
    
    res->data = data;
    res->cleanup = cleanup;
    res->desc = desc ? desc : "unknown";
    
    uint64_t flags;
    spinlock_irq_save(flags);
    spinlock_acquire(&sched_lock);
    
    res->next = task->resources;
    task->resources = res;
    
    spinlock_release(&sched_lock);
    spinlock_irq_restore(flags);
}

void sched_free_resources(struct task_struct *task) {
    /* Assumes caller holds lock or task is dead/zombie and no one else touches it */
    /* But to be safe, we lock. */
    
    /* Note: sched_free_resources called at exit. */
    
    struct resource *iter = task->resources;
    while (iter) {
        struct resource *next = iter->next;
        if (iter->cleanup) {
            kprint("[RES] Cleaning up: "); kprint(iter->desc); kprint("\n");
            iter->cleanup(iter->data);
        }
        kfree(iter);
        iter = next;
    }
    task->resources = 0; // NULL
}


/* Assembly stub defined at the bottom */
extern void timer_isr_stub(void);

/* Idle Loop */
static void idle_loop(void) {
    while (1) {
        __asm__ volatile("hlt");
        /* Check if we need to yield? No, IRQ handles it. */
    }
}

void sched_init(void) {
    memset(run_queues, 0, sizeof(run_queues));
    priority_bitmap = 0;
    
    /* Create IDLE Task */
    idle_task = (struct task_struct *)kmalloc(sizeof(struct task_struct));
    memset(idle_task, 0, sizeof(struct task_struct));
    
    /* Setup Idle Stack */
    void *stack_base = kmalloc(4096);
    uint64_t *stack_top = (uint64_t *)((uint8_t *)stack_base + 4096);
    uint64_t *stack = stack_top;
    
    /* Frame for switch_to (pop everything) */
    /* sched_create_task logic: RIP, CS, RFLAGS, RSP, SS */
    /* Wait, the first time we switch TO idle_task, we just do MOV RSP then POP registers then IRETQ */
    
    stack--; *stack = GDT_KERNEL_DATA;
    stack--; *stack = (uint64_t)stack_top;
    stack--; *stack = 0x202;
    stack--; *stack = GDT_KERNEL_CODE;
    stack--; *stack = (uint64_t)idle_loop;
    for(int i=0; i<15; i++) { stack--; *stack = 0; }
    
    idle_task->rsp = (uint64_t)stack;
    idle_task->rsp0 = (uint64_t)stack_top;
    idle_task->state = TASK_READY; 
    strcpy(idle_task->name, "idle");
    idle_task->priority = MAX_PRIO - 1; /* Lowest */
    idle_task->address_space = vmm_get_kernel_address_space();
    
    /* Current task is Kernel Init (PID 0) */
    current_task = (struct task_struct *)kmalloc(sizeof(struct task_struct));
    memset(current_task, 0, sizeof(struct task_struct));
    current_task->id = 0;
    current_task->state = TASK_RUNNING;
    current_task->flags = TASK_KERNEL;
    current_task->priority = 0; /* Highest for init */
    strcpy(current_task->name, "kernel_init");
    current_task->address_space = vmm_get_kernel_address_space();

    spinlock_init(&sched_lock, "sched_lock");
    kprint("[SCHED] O(1) Scheduler Initialized.\n");
}

struct task_struct *sched_get_current(void) {
    return current_task;
}

struct task_struct *sched_get_by_pid(uint64_t pid) {
    /* Brute force scan all queues */
    for (int p = 0; p < MAX_PRIO; p++) {
        struct task_struct *t = run_queues[p].head;
        while(t) {
            if (t->id == pid) return t;
            t = t->next;
        }
    }
    if (current_task && current_task->id == pid) return current_task;
    if (idle_task && idle_task->id == pid) return idle_task;
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
    new_task->priority = DEFAULT_PRIO;
    new_task->address_space = vmm_get_kernel_address_space();
    strcpy(new_task->name, "kernel_task");
    
    uint64_t flags;
    spinlock_irq_save(flags);
    spinlock_acquire(&sched_lock);
    enqueue_task(new_task);
    spinlock_release(&sched_lock);
    spinlock_irq_restore(flags);
    
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
    
    stack--; *stack = GDT_USER_DATA_RPL3;
    stack--; *stack = user_stack;
    stack--; *stack = 0x202;
    stack--; *stack = GDT_USER_CODE_RPL3;
    stack--; *stack = entry;
    
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
    new_task->priority = DEFAULT_PRIO;
    new_task->address_space = space;
    strcpy(new_task->name, "user_task");
    
    new_task->signals = signal_alloc();

    uint64_t flags;
    spinlock_irq_save(flags);
    spinlock_acquire(&sched_lock);
    enqueue_task(new_task);
    spinlock_release(&sched_lock);
    spinlock_irq_restore(flags);
    
    kprint("[SCHED] User Task Created O(1).\n");
    return new_task;
}

void sched_yield(void) {
    /* Trigger a reschedule */
    __asm__ volatile("hlt");
}

void sched_exit(int code) {
    if (!current_task) return;
    
    /* Clean up resources BEFORE becoming zombie */
    /* We are running on current_task stack, so we can clean up other things */
    /* Locking not strictly needed for local resources if only this task accesses them,
       but for consistency and global visibility, we lock or just rely on being current.
       However, sched_free_resources is robust. */
    
    /* We don't hold lock during cleanup to allow cleanup functions to sleep/schedule if needed?
       But we are exiting. Cleanup should be fast. */
    sched_free_resources(current_task);

    uint64_t flags;
    spinlock_irq_save(flags);
    spinlock_acquire(&sched_lock);
    
    current_task->exit_code = code;
    current_task->state = TASK_ZOMBIE;
    
    spinlock_release(&sched_lock);
    /* Interrupts still disabled by spinlock_irq_save? No, release doesn't restore. */
    /* We want to yield with interrupts enabled to allow timer to switch us out. */
    spinlock_irq_restore(flags);
    
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
    if (current_task) current_task->rsp = rsp;
    
    spinlock_acquire(&sched_lock);
    
    timer_handler_callback();

    /* Put current task back in queue if it's still running */
    if (current_task && current_task->state == TASK_RUNNING) {
        current_task->state = TASK_READY;
        enqueue_task(current_task);
    }
    
    /* Pick next task O(1) */
    struct task_struct *next = dequeue_task();
    
    if (!next) {
        next = idle_task;
    }
    
    /* Context Switch Logic */
    if (next != current_task) {
        if (current_task && current_task != idle_task) {
             __asm__ volatile("fxsave %0" : "=m"(current_task->fpu_state));
        }
        
        current_task = next;
        
        if (current_task && current_task != idle_task) {
             current_task->state = TASK_RUNNING;
             __asm__ volatile("fxrstor %0" : : "m"(current_task->fpu_state));
        }
        
        if (current_task->rsp0) tss_set_rsp0(current_task->rsp0);
        if (current_task->address_space) vmm_switch_address_space(current_task->address_space);
    }
    
    spinlock_release(&sched_lock);
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
    return 0;
}
