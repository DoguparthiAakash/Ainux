
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
    syscall3(1, 1, (long)"\n[Init] Hello from Userspace C!\n", 32);
    syscall3(1, 1, (long)"[Init] Type something: ", 23);

    char buf[1];
    while(1) {
        // Read 1 char from fd=0 (stdin)
        long n = syscall3(7, 0, (long)buf, 1);
        if (n == 1) {
            // Write to fd=1 (stdout)
            syscall3(1, 1, (long)buf, 1);
            if (buf[0] == '\r') {
               buf[0] = '\n';
               syscall3(1, 1, (long)buf, 1);
            }
        } else {
            // Yield CPU if no data (syscall 24)
            syscall3(24, 0, 0, 0);
        }
    }
    
    syscall3(60, 0, 0, 0);
    for(;;);
}
