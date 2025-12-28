#include "nano_asm.h"
#include "libc/stdio.h"
#include "libc/stdlib.h"
#include "libc/string.h"
#include "mm/heap.h"
#include "shell.h" /* for kprint */

/* Minimal x86_64 Assembler 
   Supports: MOV, ADD, SUB, RET, INT
   Registers: RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI (0-7)
*/

extern void kprint(const char *msg);

typedef enum {
    REG_RAX = 0, REG_RCX = 1, REG_RDX = 2, REG_RBX = 3,
    REG_RSP = 4, REG_RBP = 5, REG_RSI = 6, REG_RDI = 7,
    REG_NONE = -1
} Register;

static int is_digit(char c) { return c >= '0' && c <= '9'; }
static int is_alpha(char c) { return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'); }
static int is_space(char c) { return c == ' ' || c == '\t'; }

static int tok_eq(const char *tok, const char *target);

static int parse_reg(const char *s) {
    if (((s[0] == 'r' || s[0] == 'R') && (s[2] == 'x' || s[2] == 'X'))) {
        if (s[1] == 'a' || s[1] == 'A') return REG_RAX;
        if (s[1] == 'b' || s[1] == 'B') return REG_RBX;
        if (s[1] == 'c' || s[1] == 'C') return REG_RCX;
        if (s[1] == 'd' || s[1] == 'D') return REG_RDX;
    }
    /* Explicit Checks */
    if (tok_eq(s, "RAX")) return REG_RAX;
    if (tok_eq(s, "RCX")) return REG_RCX;
    if (tok_eq(s, "RDX")) return REG_RDX;
    if (tok_eq(s, "RBX")) return REG_RBX;
    if (tok_eq(s, "RSP")) return REG_RSP;
    if (tok_eq(s, "RBP")) return REG_RBP;
    if (tok_eq(s, "RSI")) return REG_RSI;
    if (tok_eq(s, "RDI")) return REG_RDI;
    
    if (tok_eq(s, "R8")) return 8;
    if (tok_eq(s, "R9")) return 9;
    if (tok_eq(s, "R10")) return 10;
    if (tok_eq(s, "R11")) return 11;
    if (tok_eq(s, "R12")) return 12;
    if (tok_eq(s, "R13")) return 13;
    if (tok_eq(s, "R14")) return 14;
    if (tok_eq(s, "R15")) return 15;
    
    return REG_NONE;
}

static uint64_t parse_imm(const char *s) {
    uint64_t val = 0;
    while (*s >= '0' && *s <= '9') {
        val = val * 10 + (*s - '0');
        s++;
    }
    return val;
}

static int str_cmp_nocase(const char *s1, const char *s2) {
    while (*s1 && *s2) {
        char a = *s1;
        char b = *s2;
        if (a >= 'a' && a <= 'z') a -= 32;
        if (b >= 'a' && b <= 'z') b -= 32;
        if (a != b) return a - b;
        s1++; s2++;
    }
    return *s1 - *s2;
}

/* Check tokens ignoring case */
static int tok_eq(const char *tok, const char *target) {
    return str_cmp_nocase(tok, target) == 0;
}

/* Helper to generate REX prefix */
/* REX = 0100 W R X B */
/* W=1 for 64-bit operand size */
/* R=1 if Reg field extends (reg >= 8) */
/* X=1 if Index field extends */
/* B=1 if RM/Base field extends (rm >= 8) */
static uint8_t rex(int is_64, int reg_r, int reg_b) {
    uint8_t r = 0x40;
    if (is_64) r |= 0x08;
    if (reg_r >= 8) r |= 0x04;
    if (reg_b >= 8) r |= 0x01;
    return r;
}

void asm_run(const char *source) {
    /* Allocate executable page */
    uint8_t *code = (uint8_t*)kmalloc(4096);
    if (!code) { kprint("ASM: Out of memory\n"); return; }
    
    int ip = 0;
    const char *ptr = source;
    
    kprint("[ASM] Assembling...\n");
    
    while (*ptr) {
        /* Skip whitespace/newlines */
        while (*ptr && (is_space(*ptr) || *ptr == '\n')) ptr++;
        if (!*ptr) break;
        
        if (*ptr == ';') { /* Comment */
            while (*ptr && *ptr != '\n') ptr++;
            continue;
        }

        /* Parse Mnemonic */
        char mnemonic[16];
        int i=0;
        while (is_alpha(*ptr) && i<15) mnemonic[i++] = *ptr++;
        mnemonic[i] = 0;
        
        if (tok_eq(mnemonic, "RET")) {
            code[ip++] = 0xC3;
            continue;
        }
        
        while (is_space(*ptr)) ptr++;
        
        /* Parse Ops */
        char op1_str[16];
        char op2_str[16];
        int op1_reg = REG_NONE;
        int op2_reg = REG_NONE;
        uint64_t op2_imm = 0;
        int has_imm = 0;
        
        /* Read Op1 */
        i=0; 
        while (is_alpha(*ptr) || is_digit(*ptr)) op1_str[i++] = *ptr++;
        op1_str[i] = 0;
        op1_reg = parse_reg(op1_str);
        
        while (is_space(*ptr)) ptr++;
        if (*ptr == ',') {
            ptr++;
            while (is_space(*ptr)) ptr++;
            
            /* Read Op2 */
            i=0;
            if (is_digit(*ptr)) {
                has_imm = 1;
                op2_imm = parse_imm(ptr);
                while (is_digit(*ptr)) ptr++;
            } else {
                while (is_alpha(*ptr) || is_digit(*ptr)) op2_str[i++] = *ptr++;
                op2_str[i] = 0;
                op2_reg = parse_reg(op2_str);
            }
        }
        
        /* Emit Code */
        if (tok_eq(mnemonic, "MOV")) {
            if (op1_reg != REG_NONE) {
                if (has_imm) {
                    /* MOV R64, IMM64: REX.W+B B8+rd imm64 */
                    code[ip++] = rex(1, 0, op1_reg);
                    code[ip++] = 0xB8 + (op1_reg & 7);
                    *(uint64_t*)&code[ip] = op2_imm;
                    ip += 8;
                } else if (op2_reg != REG_NONE) {
                    /* MOV R64, R64: REX.W+R+B 89 /r */
                    /* ModRM: src=op2(reg), dst=op1(rm) */
                    /* MR encoding: 89 mod(11) reg(op2) rm(op1) */
                    code[ip++] = rex(1, op2_reg, op1_reg);
                    code[ip++] = 0x89;
                    code[ip++] = 0xC0 | ((op2_reg & 7) << 3) | (op1_reg & 7);
                }
            } else {
                 kprint("ASM: Invalid Reg1 in MOV\n");
            }
        } else if (tok_eq(mnemonic, "ADD")) {
            if (op1_reg != REG_NONE && op2_reg != REG_NONE) {
                 /* ADD R64, R64: 48 01 */
                 code[ip++] = rex(1, op2_reg, op1_reg);
                 code[ip++] = 0x01;
                 code[ip++] = 0xC0 | ((op2_reg & 7) << 3) | (op1_reg & 7);
            }
        } else if (tok_eq(mnemonic, "SUB")) {
            if (op1_reg != REG_NONE && op2_reg != REG_NONE) {
                 /* SUB R64, R64: 48 29 */
                 code[ip++] = rex(1, op2_reg, op1_reg);
                 code[ip++] = 0x29;
                 code[ip++] = 0xC0 | ((op2_reg & 7) << 3) | (op1_reg & 7);
            }
        } else if (tok_eq(mnemonic, "INT")) {
            /* INT imm8: CD imm8 */
            if (has_imm) {
                code[ip++] = 0xCD;
                code[ip++] = (uint8_t)op2_imm;
            }
        } else {
             kprint("ASM: Unknown instruction: "); kprint(mnemonic); kprint("\n");
        }
    }
    
    kprint("[ASM] Executing...\n");
    /* Cast to func */
    uint64_t (*func)(void) = (uint64_t (*)(void))code;
    uint64_t ret = func();
    
    kprint("[ASM] Finished. RAX="); printf("%d", (int)ret); kprint("\n");
    kfree(code);
}
