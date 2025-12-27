#ifndef NANO_C_H
#define NANO_C_H

#include <stdint.h>

/* Virtual Machine Instructions */
typedef enum {
    OP_EXIT = 0,
    OP_IMM,     /* Push Immediate: [OP, val] */
    OP_ADD,     /* Pop a, b; Push a+b */
    OP_SUB,     /* Pop a, b; Push b-a */
    OP_MUL,     /* Pop a, b; Push a*b */
    OP_DIV,     /* Pop a, b; Push b/a */
    OP_PRINT,   /* Pop a; Print a */
    OP_PRINT_STR, /* Print String: [OP, str_index] */
    OP_LOAD,    /* Load var: [OP, var_index]; Push val */
    OP_STORE,   /* Store var: [OP, var_index]; Pop val */
    OP_CMP_LT,  /* Pop a, b; Push (b < a) */
    OP_CMP_GT,  /* Pop a, b; Push (b > a) */
    OP_CMP_EQ,  /* Pop a, b; Push (b == a) */
    OP_JZ,      /* Jump if Zero: [OP, addr]; Pop cond */
    OP_JMP,     /* Jump: [OP, addr] */
    OP_INPUT,   /* Push Input char: [OP]; Push char */
    OP_MOD,     /* Pop a, b; Push a%b */
    OP_PRINT_FMT, /* [OP, str_idx, num_args]; Pop num_args values; format print */
    OP_RAND,     /* Push Random int */
    OP_GFX_RECT, /* [OP]; Pop x, y, w, h, color; Draw Rect */
    OP_GFX_TEXT,  /* [OP, str_idx]; Pop x, y, color; Draw Text */
    OP_SCANF,    /* [OP]; Pop addr; Read Int to addr */
    OP_AND, OP_OR, OP_NOT
} OpCode;

/* Run a C-like script source code */
void nano_c_run(const char *source);

#endif
