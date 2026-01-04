#include <stdint.h>

/* Syscall Numbers matching kernel/syscalls/syscalls.c */
#define SYS_READ 0
#define SYS_WRITE 1
#define SYS_OPEN 2
#define SYS_CLOSE 3
#define SYS_EXIT 60
#define SYS_YIELD 24
#define SYS_SLEEP 35

/* Generic Syscall Function */
static uint64_t syscall0(uint64_t num) {
    uint64_t ret;
    __asm__ volatile ("syscall" : "=a"(ret) : "a"(num) : "rcx", "r11", "memory");
    return ret;
}

static uint64_t syscall1(uint64_t num, uint64_t arg1) {
    uint64_t ret;
    /* Syscall arguments in RDI, RSI, RDX, R10, R8, R9 */
    __asm__ volatile ("syscall" : "=a"(ret) : "a"(num), "D"(arg1) : "rcx", "r11", "memory");
    return ret;
}

static uint64_t syscall2(uint64_t num, uint64_t arg1, uint64_t arg2) {
    uint64_t ret;
    __asm__ volatile ("syscall" : "=a"(ret) : "a"(num), "D"(arg1), "S"(arg2) : "rcx", "r11", "memory");
    return ret;
}

static uint64_t syscall3(uint64_t num, uint64_t arg1, uint64_t arg2, uint64_t arg3) {
    uint64_t ret;
    __asm__ volatile ("syscall" : "=a"(ret) : "a"(num), "D"(arg1), "S"(arg2), "d"(arg3) : "rcx", "r11", "memory");
    return ret;
}

/* Wrappers */
void sys_exit(int code) {
    syscall1(SYS_EXIT, (uint64_t)code);
    while(1); /* Should not return */
}

long sys_write(int fd, const char *buf, uint64_t count) {
    return syscall3(SYS_WRITE, (uint64_t)fd, (uint64_t)buf, count);
}

long sys_read(int fd, char *buf, uint64_t count) {
    return syscall3(SYS_READ, (uint64_t)fd, (uint64_t)buf, count);
}

void sys_yield(void) {
    syscall0(SYS_YIELD);
}

void sys_sleep(uint64_t ms) {
    syscall1(SYS_SLEEP, ms);
}

#define SYS_SPAWN 59
#define SYS_WAIT  61

long sys_spawn(const char *path) {
    return syscall1(SYS_SPAWN, (uint64_t)path);
}

long sys_wait(int pid) {
    return syscall1(SYS_WAIT, (uint64_t)pid);
}
