#ifndef AINUX_H
#define AINUX_H

#include <stdint.h>

// Ainux Syscall Definitions
#define SYS_EXIT 60
#define SYS_GET_TASKS 500
#define SYS_DISK_READ 505
#define SYS_DISK_WRITE 506
#define SYS_DISK_IDENTIFY 507

static inline long syscall0(long n) {
    long ret;
    __asm__ __volatile__ ("syscall" : "=a"(ret) : "a"(n) : "rcx", "r11", "memory");
    return ret;
}

static inline long syscall1(long n, long a1) {
    long ret;
    __asm__ __volatile__ ("syscall" : "=a"(ret) : "a"(n), "D"(a1) : "rcx", "r11", "memory");
    return ret;
}

static inline long syscall2(long n, long a1, long a2) {
    long ret;
    __asm__ __volatile__ ("syscall" : "=a"(ret) : "a"(n), "D"(a1), "S"(a2) : "rcx", "r11", "memory");
    return ret;
}

static inline long syscall3(long n, long a1, long a2, long a3) {
    long ret;
    __asm__ __volatile__ ("syscall" : "=a"(ret) : "a"(n), "D"(a1), "S"(a2), "d"(a3) : "rcx", "r11", "memory");
    return ret;
}

// Wrapper for sys_get_tasks
static inline long sys_get_tasks(char *buf, unsigned long len) {
    return syscall2(SYS_GET_TASKS, (long)buf, (long)len);
}

// Wrapper for sys_exit
static inline void sys_exit(int status) {
    syscall1(SYS_EXIT, (long)status);
}

#endif // AINUX_H
