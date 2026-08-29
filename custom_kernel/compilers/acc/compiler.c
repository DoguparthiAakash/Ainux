#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

// Extremely basic Single-pass C subset to x86_64 assembly compiler

extern int fast_tokenize(const char* source, char* out_tokens); // Defined in assembly

void compile_to_asm(const char* source, const char* out_asm) {
    FILE* out = fopen(out_asm, "w");
    if (!out) return;

    fprintf(out, ".section .text\n");
    fprintf(out, ".global _start\n");
    fprintf(out, "_start:\n");
    
    // Very rudimentary parser just to prove the concept for Ainux
    const char *p = source;
    while (*p) {
        while (isspace(*p)) p++;
        if (strncmp(p, "int main", 8) == 0) {
            p += 8;
            while (*p && *p != '{') p++;
            if (*p == '{') p++;
            continue;
        }
        
        if (strncmp(p, "return", 6) == 0) {
            p += 6;
            while (isspace(*p)) p++;
            int ret_val = 0;
            if (isdigit(*p)) {
                ret_val = atoi(p);
                while (isdigit(*p)) p++;
            }
            // Generate exit syscall for x86_64
            fprintf(out, "    movq $60, %%rax\n"); // sys_exit
            fprintf(out, "    movq $%d, %%rdi\n", ret_val);
            fprintf(out, "    syscall\n");
            while (*p && *p != ';') p++;
            if (*p == ';') p++;
            continue;
        }
        
        if (strncmp(p, "puts", 4) == 0) {
            p += 4;
            while (*p && *p != '"') p++;
            if (*p == '"') p++;
            char str[256];
            int i = 0;
            while (*p && *p != '"' && i < 255) {
                str[i++] = *p++;
            }
            str[i] = 0;
            if (*p == '"') p++;
            
            // Add string to .data section
            fprintf(out, ".section .data\n");
            fprintf(out, "str_%p:\n", p);
            fprintf(out, "    .ascii \"%s\\n\"\n", str);
            fprintf(out, "str_len_%p = . - str_%p\n", p, p);
            
            // Switch back to text
            fprintf(out, ".section .text\n");
            fprintf(out, "    movq $1, %%rax\n"); // sys_write
            fprintf(out, "    movq $1, %%rdi\n"); // stdout
            fprintf(out, "    lea str_%p(%%rip), %%rsi\n", p);
            fprintf(out, "    movq $str_len_%p, %%rdx\n", p);
            fprintf(out, "    syscall\n");
            
            while (*p && *p != ';') p++;
            if (*p == ';') p++;
            continue;
        }
        p++;
    }
    
    // Add default exit if missing
    fprintf(out, "    movq $60, %%rax\n");
    fprintf(out, "    movq $0, %%rdi\n");
    fprintf(out, "    syscall\n");
    
    fclose(out);
}

// Dummy wrapper for assemble and link (these would use native binutils or built-in methods)
int assemble(const char* asm_file, const char* obj_file) {
    char cmd[256];
    // In Ainux, we might use an embedded assembler, but for now we simulate via system calls
    // or compile using host gcc for testing in WSL.
    // Wait, inside Ainux we don't have gcc/as. 
    // We will just invoke nuxa (our assembler tool) if it exists.
    sprintf(cmd, "nuxa %s %s", asm_file, obj_file);
    return system(cmd); 
}

int link_obj(const char* obj_file, const char* out_elf) {
    char cmd[256];
    // Simulate linking
    sprintf(cmd, "cp %s %s", obj_file, out_elf);
    return system(cmd);
}
