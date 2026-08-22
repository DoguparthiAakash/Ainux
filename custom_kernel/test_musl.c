#include <stdio.h>
#include <unistd.h>
#include <sys/mman.h>

int main(int argc, char **argv) {
    printf("Hello from musl-libc on Ainux!\n");
    
    // Test basic memory allocation
    void *ptr = mmap(NULL, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (ptr != MAP_FAILED) {
        printf("mmap successful at %p\n", ptr);
        munmap(ptr, 4096);
    } else {
        printf("mmap failed!\n");
    }
    
    return 42; // Expecting this exit code in the kernel
}
