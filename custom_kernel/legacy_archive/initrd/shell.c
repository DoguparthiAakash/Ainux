#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main() {
    printf("Ainuix Shell v0.1\n");
    printf("Type 'help' for commands\n");
    
    char input[256];
    while(1) {
        printf("$ ");
        fflush(stdout);
        
        if (fgets(input, sizeof(input), stdin) == NULL) {
            break;
        }
        
        // Remove newline
        input[strcspn(input, "\n")] = 0;
        
        if (strcmp(input, "help") == 0) {
            printf("Available commands: help, echo, exit\n");
        } else if (strncmp(input, "echo ", 5) == 0) {
            printf("%s\n", input + 5);
        } else if (strcmp(input, "exit") == 0) {
            break;
        } else if (strlen(input) > 0) {
            printf("Command not found: %s\n", input);
        }
    }
    
    return 0;
}