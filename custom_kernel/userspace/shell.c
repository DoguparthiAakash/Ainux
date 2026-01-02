#include <stdint.h>

/* From syscalls.c */
long sys_write(int fd, const char *buf, uint64_t count);
long sys_read(int fd, char *buf, uint64_t count);
long sys_spawn(const char *path);
long sys_wait(int pid);
void sys_exit(int code);
void sys_yield(void);

/* Helper for string comparison */
int strcmp(const char *a, const char *b) {
    while (*a && *b && *a == *b) { a++; b++; }
    return (*a - *b);
}

void print(const char *s) {
    uint64_t len = 0;
    while(s[len]) len++;
    sys_write(1, s, len); 
}

/* Helper to read a line */
void read_line(char *buf, int max) {
    int i = 0;
    while (i < max - 1) {
        char c;
        long ret = sys_read(0, &c, 1);
        
        if (ret > 0) {
            if (c == '\n' || c == '\r') {
                print("\n");
                break;
            } else if (c == '\b') {
                if (i > 0) {
                    i--;
                    print("\b \b");
                }
            } else {
                buf[i++] = c;
                char echo[2] = {c, 0};
                print(echo);
            }
        } else {
             /* Yield if no input to avoid burning CPU */
             sys_yield();
        }
    }
    buf[i] = 0;
}

int main(int argc, char **argv) {
    (void)argc; (void)argv;
    
    print("\033[32mAinux Userspace Shell (Ring 3)\033[0m\n");
    print("Type 'help' for commands.\n\n");
    
    char cmd[128];
    
    while (1) {
        print("\033[36mAinux-User>\033[0m ");
        read_line(cmd, 128);
        
        if (strcmp(cmd, "help") == 0) {
            print("Available Commands:\n");
            print("  help    - Show this help\n");
            print("  echo    - Echo text\n");
            print("  test    - Run a loop test\n");
            print("  exit    - Exit shell\n");
        } else if (strcmp(cmd, "exit") == 0) {
            print("Exiting userspace shell...\n");
            break;
        } else if (strcmp(cmd, "test") == 0) {
             print("Running Loop Test...\n");
             for(int i=0; i<5; i++) {
                 print("Tick\n");
                 sys_yield();
             }
        } else if (cmd[0] == '.' && cmd[1] == '/') {
             /* Run command like ./hello.elf */
             /* Map to /initrd/ for now or handle cwd */
             /* Let's construct path: /initrd/ + cmd+2 */
             
             char path[64] = "/initrd/";
             char *src = cmd + 2;
             int idx = 8;
             while (*src && idx < 63) path[idx++] = *src++;
             path[idx] = 0;
             
             long pid = sys_spawn(path);
             if (pid > 0) {
                 print("Spawned PID: ");
                 /* print pid logic */
                 print("\n");
                 long ret = sys_wait(pid);
                 if (ret != 0) {
                     print("\033[31m[Process Exited with Error: ");
                     char buf[16]; int n=ret; if(n<0){print("-"); n=-n;}
                     /* simple itoa */
                     int i=0; do{buf[i++]=(n%10)+'0'; n/=10;}while(n>0);
                     while(i>0) { char c[2]={buf[--i],0}; print(c); }
                     print("]\033[0m\n");
                 } else {
                     print("Process Exited Successfully.\n");
                 }
             } else {
                 print("Failed to spawn.\n");
             }
             
        } else if (cmd[0] == 'e' && cmd[1] == 'c' && cmd[2] == 'h' && cmd[3] == 'o') {
             print(cmd + 5); /* Simple echo */
             print("\n");
        } else if (cmd[0] == 0) {
             continue; /* Empty */
        } else {
             print("Unknown command: ");
             print(cmd);
             print("\n");
        }
    }
    
    return 0;
}
