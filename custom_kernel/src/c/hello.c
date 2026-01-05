
typedef unsigned long long size_t;
typedef long long ssize_t;

static inline long syscall3(long n, long a1, long a2, long a3) {
    unsigned long ret;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(n), "D"(a1), "S"(a2), "d"(a3)
        : "rcx", "r11", "memory"
    );
    return ret;
}

void _start() {
    // Intentional Crash
    *(volatile int*)0 = 1234;

    syscall3(1, 1, (long)"Hello from Userspace C!\n", 24);
    syscall3(60, 0, 0, 0);
    for(;;);
}
