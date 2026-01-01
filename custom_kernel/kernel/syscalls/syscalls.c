#include "syscalls.h"
#include "log.h"
#include "sched/sched.h"
#include "drivers/timer.h" // For sys_sleep
#include "gfx/gfx.h" // For sys_draw_rect
#include "drivers/gfx/wm.h"
#include "io/vfs.h" // For file operations
#include "libc/stdio.h" // For file descriptors

// Forward declaration of specific syscall implementations
long sys_write(int fd, const char *buf, uint64_t count);
long sys_read(int fd, char *buf, uint64_t count);
long sys_open(const char *pathname, int flags, ...);
int sys_close(int fd);
long sys_lseek(int fd, long offset, int whence);
void sys_exit(int error_code);
void sys_yield(void);
void sys_sleep(uint64_t ms);
void sys_draw_rect(int x, int y, int w, int h, uint32_t color);

// struct registers must match the push order in assembly (first pushed = highest address)
// Assembly pushes: RAX, RBX, RCX, RDX, RSI, RDI, RBP, R8, R9, R10, R11, R12, R13, R14, R15
// Stack grows DOWN, so first pushed is at highest address
// When we pass %rsp as pointer, we point to TOP of stack (R15)
struct registers {
    // Order matches pop order (reverse of push)
    uint64_t r15, r14, r13, r12, r11, r10, r9, r8;
    uint64_t rbp, rdi, rsi, rdx, rcx, rbx, rax;
    // Pushed by CPU on interrupt
    uint64_t rip, cs, rflags, rsp, ss;
};

// Define syscall numbers
#define SYS_READ 0
#define SYS_WRITE 1
#define SYS_OPEN 2
#define SYS_CLOSE 3
#define SYS_LSEEK 8

// Wrapper called by ASM
// We rename the internal handler to match what ASM calls
void syscall_handler_c_stub(struct registers *regs) {
    uint64_t num = regs->rax;
    // Trace Dispatch removed
    
    // Arguments from specific registers (System V AMD64 ABI for syscalls uses RDI, RSI, RDX, R10, R8, R9)
    uint64_t arg1 = regs->rdi;
    uint64_t arg2 = regs->rsi;
    uint64_t arg3 = regs->rdx;
    uint64_t arg4 = regs->r10;
    uint64_t arg5 = regs->r8;
    uint64_t arg6 = regs->r9;

    switch (num) {
        case SYS_READ:
            regs->rax = sys_read((int)arg1, (char *)arg2, arg3);
            break;
        case SYS_WRITE:
            regs->rax = sys_write((int)arg1, (const char *)arg2, arg3);
            break;
        case SYS_OPEN:
            regs->rax = sys_open((const char *)arg1, (int)arg2, (int)arg3);
            break;
        case SYS_CLOSE:
            regs->rax = sys_close((int)arg1);
            break;
        case SYS_LSEEK:
            regs->rax = sys_lseek((int)arg1, (long)arg2, (int)arg3);
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
            regs->rax = -1;
            break;
    }
}

// ------ Implementations ------

long sys_read(int fd, char *buf, uint64_t count) {
    // In a real system, this would read from file descriptors
    // For now, we'll return 0 (EOF) for all reads
    (void)fd; (void)buf; (void)count;
    return 0;
}

long sys_write(int fd, const char *buf, uint64_t count) {
    (void)fd;
    if (count > 0) {
        for (uint64_t i = 0; i < count; i++) {
            char c[2] = {buf[i], 0};
            kprint(c);
        }
    }
    return count;
}

long sys_open(const char *pathname, int flags, ...) {
    // In a real system, this would open files through VFS
    // For now, return -1 (error) for all files except special cases
    (void)pathname; (void)flags;
    kprint("[SYSCALL] Open: ");
    kprint(pathname);
    kprint("\n");
    return -1; // For now, all opens fail
}

int sys_close(int fd) {
    // For now, return 0 (success)
    (void)fd;
    return 0;
}

long sys_lseek(int fd, long offset, int whence) {
    // For now, return -1 (not implemented)
    (void)fd; (void)offset; (void)whence;
    return -1;
}

void sys_exit(int error_code) {
    kprint("[SYSCALL] Task Exited with code: ");
    // primitive print
    if (error_code == 0) kprint("0");
    else {
        char buf[32]; int n=error_code; int i=0; 
        if(n<0) { kprint("-"); n=-n; }
        while(n>0) { buf[i++]='0'+(n%10); n/=10; }
        for(int j=i-1; j>=0; j--) { char c[2]={buf[j],0}; kprint(c); }
    }
    kprint("\n");
    
    /* Correctly terminate the task */
    sched_exit(error_code);
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
    /* Check if task has a guided window */
    struct task_struct *curr = sched_get_current();
    if (curr && curr->output_window) {
        /* Virtual Layer: Draw to Window Buffer */
        wm_fill_rect((Window*)curr->output_window, x, y, w, h, color);
    } else {
        /* Direct Hardware Access (Kernel Mode / Fullscreen) */
        gfx_fill_rect(x, y, w, h, color);
    }
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
