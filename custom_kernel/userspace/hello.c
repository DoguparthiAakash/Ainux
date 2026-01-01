
/* Simple Userspace "Hello World" to test ELF Loader */
/* We don't have standard libc, so we need inline assembly or a mini-lib */

#define SYS_WRITE 1
#define SYS_EXIT 60

static void sys_write(int fd, const char *buf, unsigned long count) {
    asm volatile (
        "syscall"
        : 
        : "a"(SYS_WRITE), "D"(fd), "S"(buf), "d"(count)
        : "rcx", "r11", "memory"
    );
}

static void sys_exit(int code) {
    asm volatile (
        "syscall"
        : 
        : "a"(SYS_EXIT), "D"(code)
        : "rcx", "r11", "memory"
    );
}

// Entry point
int main(int argc, char **argv) {
    (void)argc; (void)argv;
    // printf("Hello from Userspace! (ELF Loaded)\n");
    const char *msg = "Hello from Userspace! (ELF Loaded)\n";
    // Calculate length logic or hardcode
    unsigned long len = 0;
    const char *p = msg;
    while (*p++) len++;
    
    // sys_write(1, "A", 1);
    
    // DEBUG: Loop forever
    // while(1);
    
    return 0;
}
