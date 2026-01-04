#include <stdio.h>
#include <unistd.h>

int main() {
    printf("Ainuix Init System Starting...\n");
    
    // Mount filesystems
    // For now we just print a message
    printf("Mounting filesystems...\n");
    
    // Start the shell
    printf("Starting shell...\n");
    
    // In a real system, we'd execve("/bin/shell", ...) here
    // But for now, just print a message
    while(1) {
        printf("Init: Shell should start here\n");
        sleep(5); // Sleep for 5 seconds
    }
    
    return 0;
}