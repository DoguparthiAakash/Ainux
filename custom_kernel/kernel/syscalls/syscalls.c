#include "syscalls.h"
#include "../log.h"
#include "../sched/sched.h"
#include "../drivers/timer.h" // For sys_sleep
#include "../gfx.h" // For sys_draw_rect

// Forward declaration of specific syscall implementations
void sys_write(int fd, const char *buf, uint64_t count);
void sys_exit(int error_code);
void sys_yield(void);
void sys_sleep(uint64_t ms);
void sys_draw_rect(int x, int y, int w, int h, uint32_t color);

// struct registers must match the push order in assembly (Reverse of pop)
// Stack Top -> [R15, R14 ... RAX] -> [Interrupt Frame]
struct registers {
    // Pushed manually
    uint64_t r15, r14, r13, r12, r11, r10, r9, r8, rbp, rdi, rsi, rdx, rcx, rbx, rax;
    // Pushed by CPU
    uint64_t rip, cs, rflags, rsp, ss;
};

// Wrapper called by ASM
// We rename the internal handler to match what ASM calls
void syscall_handler_c_stub(struct registers *regs) {
    uint64_t syscall_num = regs->rax;

    // Arguments from specific registers (System V AMD64 ABI for syscalls uses RDI, RSI, RDX, R10, R8, R9)
    uint64_t arg1 = regs->rdi;
    uint64_t arg2 = regs->rsi;
    uint64_t arg3 = regs->rdx;
    uint64_t arg4 = regs->r10;
    uint64_t arg5 = regs->r8;
    uint64_t arg6 = regs->r9;

    switch (syscall_num) {
        case SYS_WRITE:
            sys_write((int)arg1, (const char *)arg2, arg3);
            regs->rax = arg3; // Return bytes written
            break;
        case SYS_EXIT:
            sys_exit((int)arg1);
            break;
        case SYS_YIELD:
            sys_yield();
            break;
        case SYS_SLEEP:
            sys_sleep(arg1);
            break;
        case SYS_DRAW_RECT:
            sys_draw_rect((int)arg1, (int)arg2, (int)arg3, (int)arg4, (uint32_t)arg5);
            break;
        default:
            kprint("Unknown Syscall: ");
            // ... (print logic)
            char buf[32];
             int n = syscall_num;
             int i=0; 
             if(n==0) { kprint("0"); }
             while(n>0) { buf[i++] = '0' + (n%10); n/=10; }
             for(int j=i-1; j>=0; j--) { char c[2]={buf[j],0}; kprint(c); }
            kprint("\n");
            regs->rax = -1;
            break;
    }
}

// ------ Implementations ------

void sys_write(int fd, const char *buf, uint64_t count) {
    // For now, ignore FD and always write to kernel log/screen
    (void)fd;
    if (count > 0) {
        // We need to be careful with pointers from user space in real OS
        // Here we assume shared address space
        for (uint64_t i = 0; i < count; i++) {
            char c[2] = {buf[i], 0};
            kprint(c);
        }
    }
}

void sys_exit(int error_code) {
    kprint("[SYSCALL] Task Exited with code: ");
    // primitive print
    char c = '0' + error_code; 
    char s[2] = {c, 0};
    kprint(s);
    kprint("\n");
    
    // For now, catch it in a loop or re-enable interrupts and halt
    while(1) {
        __asm__ volatile ("hlt");
    }
}

void sys_yield(void) {
    // Manually trigger a schedule
    // Actually, we can just call schedule() if we expose it?
    // Or wait for timer.
    // For now, let's busy wait a tiny bit to let timer catch up?
    // Proper yield requires calling 'schedule()' from sched.c
    // Let's assume the timer interrupt handles it for now.
    // But we can implement a "give up remainder of slice" later.
    __asm__ volatile("hlt"); // HLT until next interrupt (timer)
}

void sys_sleep(uint64_t ms) {
    uint64_t ticks_needed = ms / 10; /* 100Hz = 10ms per tick */
    if (ticks_needed == 0) ticks_needed = 1;
    
    uint64_t start_ticks = timer_get_ticks();
    
    while (timer_get_ticks() < start_ticks + ticks_needed) {
        // Yield CPU while waiting
        sys_yield();
    }
}

void sys_draw_rect(int x, int y, int w, int h, uint32_t color) {
    // Basic bounds check could be here
    gfx_fill_rect(x, y, w, h, color);
}

// ------ Assembly Stub ------

__asm__(
    ".global syscall_isr_stub\n"
    "syscall_isr_stub:\n"
    "    cli\n"
    // Push GPRs
    "    push %rax\n" // Syscall Num
    "    push %rbx\n"
    "    push %rcx\n"
    "    push %rdx\n" // Arg3
    "    push %rsi\n" // Arg2
    "    push %rdi\n" // Arg1
    "    push %rbp\n"
    "    push %r8\n"
    "    push %r9\n"
    "    push %r10\n"
    "    push %r11\n"
    "    push %r12\n"
    "    push %r13\n"
    "    push %r14\n"
    "    push %r15\n"
    
    "    mov %rsp, %rdi\n" // Pass pointer to struct registers
    "    call syscall_handler_c_stub\n" 
    
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
    "    pop %rax\n" // Correctly restored with return value if modified in struct
    
    "    sti\n"
    "    iretq\n"
);
