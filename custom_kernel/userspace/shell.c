#include <stdint.h>

/* From syscalls.c */
long sys_write(int fd, const char *buf, uint64_t count);
void sys_exit(int code);
void sys_yield(void);

void print(const char *s) {
    uint64_t len = 0;
    while(s[len]) len++;
    sys_write(1, s, len); /* 1 = stdout */
}

int main(int argc, char **argv) {
    (void)argc; (void)argv;
    
    print("\n\n");
    print("==================================\n");
    print("   Hello from Userspace Shell!    \n");
    print("   Running in Ring 3 (User Mode)  \n");
    print("==================================\n");
    print("\n");
    
    print("Yielding to kernel loop...\n");
    for(int i=0; i<5; i++) {
        print(".");
        sys_yield();
    }
    print("\nDone.\n");
    
    return 0;
}
