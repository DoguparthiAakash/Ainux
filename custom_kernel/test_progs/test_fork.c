#include <stdio.h>
#include <unistd.h>
#include <sys/wait.h>

int main() {
    printf("Starting fork test...\n");
    
    int pid = fork();
    if (pid == 0) {
        printf("I am the child process (PID: %d)\n", getpid());
        // Do some work in child
        for (int i = 0; i < 3; i++) {
            printf("Child working... %d\n", i);
            // sleep equivalent...
        }
        printf("Child exiting.\n");
        _exit(42);
    } else if (pid > 0) {
        printf("I am the parent process, child is %d\n", pid);
        int status = 0;
        int ret = wait(&status);
        printf("Child (pid %d) exited with status %d\n", ret, status);
    } else {
        printf("Fork failed!\n");
    }
    
    return 0;
}
