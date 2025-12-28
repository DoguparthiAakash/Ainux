#include "sched.h"
#include "../mm/heap.h"
#include "../mm/pmm.h"
#include "../log.h"
#include "../drivers/timer.h" // For timer_handler_callback

// Global Scheduler State
struct task_struct *current_task = 0;
struct task_struct *task_head = 0;

// Assembly stub defined at the bottom
extern void timer_isr_stub(void);

void sched_init(void) {
    // Create task for the running kernel process (PID 0)
    current_task = (struct task_struct*)kmalloc(sizeof(struct task_struct));
    current_task->id = 0;
    current_task->state = TASK_RUNNING;
    current_task->next = current_task; // Circular list
    current_task->rsp = 0; // Will be set on first switch

    task_head = current_task;
    kprint("Scheduler initialized. PID 0 created.\n");
}

struct task_struct* sched_create_task(void (*entry_point)(void)) {
    struct task_struct *new_task = (struct task_struct*)kmalloc(sizeof(struct task_struct));
    
    // Allocate 4KB Stack
    // Allocate 4KB Stack
    void *stack_base = kmalloc(4096);
    uint64_t *stack_top = (uint64_t*)(stack_base + 4096);
    uint64_t *stack = stack_top;

    // 1. Hardware Frame
    // Stack grows DOWN. Order pushed by CPU on interrupt: SS, RSP, RFLAGS, CS, RIP
    
    stack--; *stack = 0x10;   // SS (Kernel Data Segment)
    stack--; *stack = (uint64_t)stack_top; // RSP (Points to the very top)
    stack--; *stack = 0x202;  // RFLAGS (Interrupts Enabled)
    stack--; *stack = 0x08;   // CS (Kernel Code Segment)
    stack--; *stack = (uint64_t)entry_point; // RIP

    // 2. Software Frame (GPRs) - 15 registers
    // RAX, RBX, RCX, RDX, RSI, RDI, RBP, R8, R9, R10, R11, R12, R13, R14, R15
    for (int i = 0; i < 15; i++) {
        stack--;
        *stack = 0;
    }

    new_task->rsp = (uint64_t)stack;
    new_task->id = task_head->id + 1; // Simple increment
    new_task->state = TASK_READY;
    new_task->next = task_head->next;
    
    // Insert into lists
    task_head->next = new_task;
    
    return new_task;
}

// C Handler called by Assembly Stub
uint64_t timer_isr_handler_c(uint64_t rsp) {
    // 1. Tick the timer
    timer_handler_callback();

    // 2. Save current RSP
    if (current_task) {
        current_task->rsp = rsp;
    }

    // 3. Switch Task (Round Robin)
    current_task = current_task->next;

    // 4. Return new RSP
    return current_task->rsp;
}

// Assembly Stub
// This must be global so IDT can link to it
__asm__(
    ".global timer_isr_stub\n"
    "timer_isr_stub:\n"
    "    cli\n"                 // Ensure interrupts disabled
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
    
    "    mov %rsp, %rdi\n"      // Checkpoint: Pass RSP as 1st arg
    "    call timer_isr_handler_c\n"
    "    mov %rax, %rsp\n"      // Restore RSP (potentially different)
    
    "    mov $0x20, %al\n"
    "    outb %al, $0x20\n"     // Send EOI to PIC (Master)
    
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
