#include <stddef.h>
#include "nano_c.h"
#include "shell.h" /* For kprint */

extern void kprint(const char *msg);
extern void *kmalloc(size_t size);
extern void kfree(void *ptr);

/* Utility: Simplified string to int */
static int my_atoi(const char *s) {
    int res = 0;
    while (*s >= '0' && *s <= '9') {
        res = res * 10 + (*s - '0');
        s++;
    }
    return res;
}

static void print_n(int n) {
    if (n == 0) { kprint("0\n"); return; }
    char buf[32];
    int i=0; 
    int neg = (n < 0);
    if (neg) n = -n;
    while(n>0) { buf[i++] = '0' + (n%10); n/=10; }
    if (neg) buf[i++] = '-';
    for(int j=i-1; j>=0; j--) { char c[2]={buf[j],0}; kprint(c); }
    kprint("\n");
}

/* ================= STRINGS TABLE ================= */
#define MAX_STRINGS 64
#define MAX_STR_LEN 128
static char string_table[MAX_STRINGS][MAX_STR_LEN];
static int str_count = 0;

static int add_string(const char *s) {
    if (str_count >= MAX_STRINGS) return -1;
    int i = 0;
    while (s[i] && i < MAX_STR_LEN-1) {
        string_table[str_count][i] = s[i];
        i++;
    }
    string_table[str_count][i] = '\0';
    return str_count++;
}

/* ================= LEXER ================= */

typedef enum {
    TOK_EOF, TOK_INT, TOK_ID, TOK_NUM, TOK_STRING, 
    TOK_IF, TOK_WHILE, TOK_PRINT, TOK_PRINTF,
    TOK_LBRACE, TOK_RBRACE, TOK_LPAREN, TOK_RPAREN, TOK_SEMI, TOK_ASSIGN,
    TOK_PLUS, TOK_MINUS, TOK_MUL, TOK_DIV, 
    TOK_EQ, TOK_NEQ, TOK_LT, TOK_GT, TOK_RETURN
} TokenType;

typedef struct {
    TokenType type;
    char str_val[MAX_STR_LEN]; /* For IDs and Strings */
    int int_val;      /* For NUMs */
} Token;

static const char *src_ptr;
static Token curr_tok;

static void next_token() {
    while (*src_ptr == ' ' || *src_ptr == '\n' || *src_ptr == '\t') src_ptr++;
    
    if (*src_ptr == '\0') {
        curr_tok.type = TOK_EOF;
        return;
    }

    /* String Literal */
    if (*src_ptr == '"') {
        src_ptr++;
        curr_tok.type = TOK_STRING;
        int len = 0;
        while (*src_ptr && *src_ptr != '"' && len < MAX_STR_LEN-1) {
            curr_tok.str_val[len++] = *src_ptr++;
        }
        curr_tok.str_val[len] = '\0';
        if (*src_ptr == '"') src_ptr++;
        return;
    }

    if (*src_ptr >= '0' && *src_ptr <= '9') {
        curr_tok.type = TOK_NUM;
        curr_tok.int_val = 0;
        while (*src_ptr >= '0' && *src_ptr <= '9') {
            curr_tok.int_val = curr_tok.int_val * 10 + (*src_ptr - '0');
            src_ptr++;
        }
        return;
    }

    if ((*src_ptr >= 'a' && *src_ptr <= 'z') || (*src_ptr >= 'A' && *src_ptr <= 'Z') || (*src_ptr == '_')) {
        int len = 0;
        while ((*src_ptr >= 'a' && *src_ptr <= 'z') || (*src_ptr >= 'A' && *src_ptr <= 'Z') || (*src_ptr >= '0' && *src_ptr <= '9') || (*src_ptr == '_')) {
            if (len < 31) curr_tok.str_val[len++] = *src_ptr;
            src_ptr++;
        }
        curr_tok.str_val[len] = '\0';
        
        /* Keywords */
        if (len==3 && curr_tok.str_val[0]=='i' && curr_tok.str_val[1]=='n' && curr_tok.str_val[2]=='t') curr_tok.type = TOK_INT;
        else if (len==2 && curr_tok.str_val[0]=='i' && curr_tok.str_val[1]=='f') curr_tok.type = TOK_IF;
        else if (len==5 && curr_tok.str_val[0]=='w' && curr_tok.str_val[1]=='h' && curr_tok.str_val[2]=='i' && curr_tok.str_val[3]=='l' && curr_tok.str_val[4]=='e') curr_tok.type = TOK_WHILE;
        else if (len==5 && curr_tok.str_val[0]=='p' && curr_tok.str_val[1]=='r' && curr_tok.str_val[2]=='i' && curr_tok.str_val[3]=='n' && curr_tok.str_val[4]=='t') curr_tok.type = TOK_PRINT;
        else if (len==6 && curr_tok.str_val[0]=='p' && curr_tok.str_val[1]=='r' && curr_tok.str_val[2]=='i' && curr_tok.str_val[3]=='n' && curr_tok.str_val[4]=='t' && curr_tok.str_val[5]=='f') curr_tok.type = TOK_PRINTF;
        else if (len==6 && curr_tok.str_val[0]=='r' && curr_tok.str_val[1]=='e' && curr_tok.str_val[2]=='t' && curr_tok.str_val[3]=='u' && curr_tok.str_val[4]=='r' && curr_tok.str_val[5]=='n') curr_tok.type = TOK_RETURN;
        else curr_tok.type = TOK_ID;
        return;
    }

    char c = *src_ptr++;
    switch (c) {
        case '{': curr_tok.type = TOK_LBRACE; break;
        case '}': curr_tok.type = TOK_RBRACE; break;
        case '(': curr_tok.type = TOK_LPAREN; break;
        case ')': curr_tok.type = TOK_RPAREN; break;
        case ';': curr_tok.type = TOK_SEMI; break;
        case '+': curr_tok.type = TOK_PLUS; break;
        case '-': curr_tok.type = TOK_MINUS; break;
        case '*': curr_tok.type = TOK_MUL; break;
        case '/': curr_tok.type = TOK_DIV; break;
        case '=': 
            if (*src_ptr == '=') { src_ptr++; curr_tok.type = TOK_EQ; }
            else curr_tok.type = TOK_ASSIGN; 
            break;
        case '!':
            if (*src_ptr == '=') { src_ptr++; curr_tok.type = TOK_NEQ; }
            else { kprint("Err: Unknown char '!'\n"); }
            break;
        case '<': curr_tok.type = TOK_LT; break;
        case '>': curr_tok.type = TOK_GT; break;
        default: kprint("Err: Unknown token\n"); break;
    }
}

/* ================= COMPILER / CODEGEN ================= */

#define MAX_CODE 4096
static int code[MAX_CODE];
static int pc = 0;

/* Simple Symbol Table: 26 global vars (a-z) */
/* Map 'a' to index 0... */
static int get_var_index(const char *name) {
    if (name[0] >= 'a' && name[0] <= 'z') return name[0] - 'a';
    if (name[0] >= 'A' && name[0] <= 'Z') return name[0] - 'A';
    return 0; /* Fallback */
}

static void emit(int op) {
    if (pc < MAX_CODE) code[pc++] = op;
    else kprint("Err: Code too large\n");
}

/* Forward Declarations */
static void parse_expr();

static void parse_factor() {
    if (curr_tok.type == TOK_NUM) {
        emit(OP_IMM);
        emit(curr_tok.int_val);
        next_token();
    } else if (curr_tok.type == TOK_ID) {
        emit(OP_LOAD);
        emit(get_var_index(curr_tok.str_val));
        next_token();
    } else if (curr_tok.type == TOK_LPAREN) {
        next_token();
        parse_expr();
        if (curr_tok.type == TOK_RPAREN) next_token();
        else kprint("Err: Expected ')'\n");
    }
}

static void parse_term() {
    parse_factor();
    while (curr_tok.type == TOK_MUL || curr_tok.type == TOK_DIV) {
        TokenType op = curr_tok.type;
        next_token();
        parse_factor();
        if (op == TOK_MUL) emit(OP_MUL);
        else emit(OP_DIV);
    }
}

static void parse_additive() {
    parse_term();
    while (curr_tok.type == TOK_PLUS || curr_tok.type == TOK_MINUS) {
        TokenType op = curr_tok.type;
        next_token();
        parse_term();
        if (op == TOK_PLUS) emit(OP_ADD);
        else emit(OP_SUB);
    }
}

static void parse_expr() {
    parse_additive();
    /* Comparisons */
    if (curr_tok.type == TOK_LT || curr_tok.type == TOK_GT || curr_tok.type == TOK_EQ) {
        TokenType op = curr_tok.type;
        next_token();
        parse_additive();
        if (op == TOK_LT) emit(OP_CMP_LT);
        else if (op == TOK_GT) emit(OP_CMP_GT);
        else if (op == TOK_EQ) emit(OP_CMP_EQ);
    }
}

static void parse_stmt() {
    if (curr_tok.type == TOK_INT) {
        next_token(); /* eat 'int' */
        if (curr_tok.type == TOK_ID) {
            int var_idx = get_var_index(curr_tok.str_val);
            next_token();
            if (curr_tok.type == TOK_ASSIGN) {
                next_token();
                parse_expr();
                emit(OP_STORE);
                emit(var_idx);
            }
            if (curr_tok.type == TOK_SEMI) next_token();
        }
    } else if (curr_tok.type == TOK_ID) {
        int var_idx = get_var_index(curr_tok.str_val);
        next_token();
        if (curr_tok.type == TOK_ASSIGN) {
            next_token();
            parse_expr();
            emit(OP_STORE);
            emit(var_idx);
            if (curr_tok.type == TOK_SEMI) next_token();
        } else if (curr_tok.type == TOK_LPAREN) { 
            /* Function call? Just skip for now if not assignment */
            while(curr_tok.type != TOK_SEMI && curr_tok.type != TOK_EOF) next_token();
            if (curr_tok.type == TOK_SEMI) next_token();
        }
    } else if (curr_tok.type == TOK_PRINT || curr_tok.type == TOK_PRINTF) {
        next_token(); /* eat print/printf */
        if (curr_tok.type == TOK_LPAREN) {
            next_token();
            
            if (curr_tok.type == TOK_STRING) {
                int str_idx = add_string(curr_tok.str_val);
                emit(OP_PRINT_STR);
                emit(str_idx);
                next_token();
            } else {
                parse_expr();
                emit(OP_PRINT);
            }
            
            if (curr_tok.type == TOK_RPAREN) next_token();
            if (curr_tok.type == TOK_SEMI) next_token();
        }
    } else if (curr_tok.type == TOK_IF) {
        next_token();
        if (curr_tok.type == TOK_LPAREN) next_token();
        parse_expr(); /* Condition */
        if (curr_tok.type == TOK_RPAREN) next_token();
        
        emit(OP_JZ);
        int jmp_addr_idx = pc++;
        
        if (curr_tok.type == TOK_LBRACE) {
            next_token();
            while (curr_tok.type != TOK_RBRACE && curr_tok.type != TOK_EOF) {
                parse_stmt();
            }
            if (curr_tok.type == TOK_RBRACE) next_token();
        } else {
             parse_stmt();
        }
        
        /* Fixup Jump */
        code[jmp_addr_idx] = pc; 
    } else if (curr_tok.type == TOK_WHILE) {
        next_token();
        int loop_start = pc;
        if (curr_tok.type == TOK_LPAREN) next_token();
        parse_expr();
        if (curr_tok.type == TOK_RPAREN) next_token();
        
        emit(OP_JZ);
        int jmp_out_idx = pc++;
        
        if (curr_tok.type == TOK_LBRACE) {
            next_token();
            while (curr_tok.type != TOK_RBRACE && curr_tok.type != TOK_EOF) {
                parse_stmt();
            }
            if (curr_tok.type == TOK_RBRACE) next_token();
        } else {
            parse_stmt();
        }
        
        emit(OP_JMP);
        emit(loop_start);
        
        code[jmp_out_idx] = pc;
    } else if (curr_tok.type == TOK_RETURN) {
        next_token();
        /* Ignore return value for now */
        while(curr_tok.type != TOK_SEMI && curr_tok.type != TOK_EOF) next_token();
        if (curr_tok.type == TOK_SEMI) next_token();
        emit(OP_EXIT);
    } else {
        /* Empty or unknown */
        if (curr_tok.type != TOK_RBRACE && curr_tok.type != TOK_EOF) next_token();
    }
}

/* ================= VM EXECUTION ================= */

static int stack[1024];
static int sp = 0;
static int globals[26];

void nano_c_run(const char *source) {
    if (!source) return;
    
    /* Reset Compiler State */
    src_ptr = source;
    pc = 0;
    sp = 0;
    str_count = 0;
    for(int i=0; i<26; i++) globals[i]=0;
    
    kprint("[Compiler] Compiling...\n");
    next_token(); /* Prime lexer */
    
    /* Parse until EOF */
    /* If 'int main() {' skip it for simplicity? Or just treat top-level as main */
    if (curr_tok.type == TOK_INT) {
        next_token();
        if (curr_tok.type == TOK_ID) { /* main? */
            next_token();
            if (curr_tok.type == TOK_LPAREN) {
                next_token(); 
                if (curr_tok.type == TOK_RPAREN) next_token(); /* ) */
                /* Handle args? No */
                if (curr_tok.type == TOK_LBRACE) next_token(); /* { */
            }
        }
    }

    while (curr_tok.type != TOK_EOF && curr_tok.type != TOK_RBRACE) {
        parse_stmt();
    }
    emit(OP_EXIT);
    
    kprint("[VM] Running...\n");
    
    /* VM Loop */
    int ip = 0;
    while (ip < pc) {
        int op = code[ip++];
        switch (op) {
            case OP_EXIT: goto done;
            case OP_IMM: stack[sp++] = code[ip++]; break;
            case OP_ADD: sp--; stack[sp-1] = stack[sp-1] + stack[sp]; break;
            case OP_SUB: sp--; stack[sp-1] = stack[sp-1] - stack[sp]; break;
            case OP_MUL: sp--; stack[sp-1] = stack[sp-1] * stack[sp]; break;
            case OP_DIV: sp--; if(stack[sp]!=0) stack[sp-1] = stack[sp-1] / stack[sp]; else stack[sp-1]=0; break;
            case OP_CMP_LT: sp--; stack[sp-1] = (stack[sp-1] < stack[sp]); break;
            case OP_CMP_GT: sp--; stack[sp-1] = (stack[sp-1] > stack[sp]); break;
            case OP_CMP_EQ: sp--; stack[sp-1] = (stack[sp-1] == stack[sp]); break;
            case OP_PRINT: sp--; print_n(stack[sp]); break;
            case OP_PRINT_STR: {
                int str_idx = code[ip++];
                if (str_idx < str_count) {
                    kprint(string_table[str_idx]);
                    /* kprint("\n"); optional? Let's assume printf doesn't usually newline unless \n is in string */
                    /* But my string parser keeps \n in string if inside ""? No, my parser is simple */
                    /* Let's just print newline for now for simplicity or raw? Raw is better */
                }
                break;
            }
            case OP_STORE: {
                int var_idx = code[ip++];
                sp--;
                globals[var_idx] = stack[sp];
                break;
            }
            case OP_LOAD: {
                int var_idx = code[ip++];
                stack[sp++] = globals[var_idx];
                break;
            }
            case OP_JZ: {
               int addr = code[ip++];
               sp--;
               if (stack[sp] == 0) ip = addr;
               break; 
            }
            case OP_JMP: {
                int addr = code[ip++];
                ip = addr;
                break;
            }
        }
    }
done:
    kprint("[VM] Done.\n");
}
