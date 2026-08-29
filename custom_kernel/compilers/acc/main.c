#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern void compile_to_asm(const char* source, const char* out_asm);
extern int assemble(const char* asm_file, const char* obj_file);
extern int link_obj(const char* obj_file, const char* out_elf);

int main(int argc, char **argv) {
    if (argc < 2) {
        printf("Ainux C Compiler (acc) v0.1\n");
        printf("Usage: acc <source.c> [-o output.elf]\n");
        return 1;
    }
    
    const char *source_file = argv[1];
    const char *output_file = "a.elf";
    
    if (argc >= 4 && strcmp(argv[2], "-o") == 0) {
        output_file = argv[3];
    }
    
    printf("[acc] Compiling %s...\n", source_file);
    
    // Step 1: Read source
    FILE *f = fopen(source_file, "r");
    if (!f) {
        printf("Error: Cannot open %s\n", source_file);
        return 1;
    }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    
    char *source = malloc(size + 1);
    fread(source, 1, size, f);
    source[size] = 0;
    fclose(f);
    
    // Step 2: Compile to Assembly (calls extern implemented in C/Assembly)
    const char *asm_file = "temp.s";
    compile_to_asm(source, asm_file);
    free(source);
    
    printf("[acc] Assembling %s...\n", asm_file);
    
    // Step 3: Assemble (wrapper calling native assembler)
    const char *obj_file = "temp.o";
    if (assemble(asm_file, obj_file) != 0) {
        printf("Error: Assembly failed\n");
        return 1;
    }
    
    printf("[acc] Linking to %s...\n", output_file);
    
    // Step 4: Link
    if (link_obj(obj_file, output_file) != 0) {
        printf("Error: Linking failed\n");
        return 1;
    }
    
    printf("[acc] Done. Created %s\n", output_file);
    return 0;
}
