#include <stddef.h>
#include <string.h>
#include "nano_c.h"
#include "shell.h" /* For kprint */
#include "drivers/keyboard.h"
#include "gfx/gfx.h"
#include "gfx/font.h"
#include "ex/io/initrd.h"

extern void kprint(const char *msg);
extern void *kmalloc(size_t size);
extern void kfree(void *ptr);

#define MAX_TOKEN_LEN 100
#define MAX_STR_LEN 128
#define MAX_VARS 128
extern void *kmalloc(size_t size);
extern void kfree(void *ptr);

/* Safe Embedded Font for Scripting */
static const uint8_t font_8x8_script[95][8] = {
    {0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00}, // Space
    {0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00}, // !
    {0x36, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00}, // "
    {0x36, 0x36, 0x7F, 0x36, 0x7F, 0x36, 0x36, 0x00}, // #
    {0x0C, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x0C, 0x00}, // $
    {0x00, 0x63, 0x33, 0x18, 0x0C, 0x66, 0x63, 0x00}, // %
    {0x1C, 0x36, 0x1C, 0x6E, 0x3B, 0x33, 0x6E, 0x00}, // &
    {0x06, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00}, // '
    {0x18, 0x0C, 0x06, 0x06, 0x06, 0x0C, 0x18, 0x00}, // (
    {0x06, 0x0C, 0x18, 0x18, 0x18, 0x0C, 0x06, 0x00}, // )
    {0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00}, // *
    {0x00, 0x0C, 0x0C, 0x3F, 0x0C, 0x0C, 0x00, 0x00}, // +
    {0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x06}, // ,
    {0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x00, 0x00}, // -
    {0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x00}, // .
    {0x60, 0x30, 0x18, 0x0C, 0x06, 0x03, 0x01, 0x00}, // /
    {0x3E, 0x63, 0x73, 0x7B, 0x6F, 0x67, 0x3E, 0x00}, // 0
    {0x0C, 0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x3F, 0x00}, // 1
    {0x1E, 0x33, 0x30, 0x1C, 0x06, 0x33, 0x3F, 0x00}, // 2
    {0x1E, 0x33, 0x30, 0x1C, 0x30, 0x33, 0x1E, 0x00}, // 3
    {0x38, 0x3C, 0x36, 0x33, 0x7F, 0x30, 0x78, 0x00}, // 4
    {0x3F, 0x03, 0x1F, 0x30, 0x30, 0x33, 0x1E, 0x00}, // 5
    {0x1C, 0x06, 0x03, 0x1F, 0x33, 0x33, 0x1E, 0x00}, // 6
    {0x3F, 0x33, 0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x00}, // 7
    {0x1E, 0x33, 0x33, 0x1E, 0x33, 0x33, 0x1E, 0x00}, // 8
    {0x1E, 0x33, 0x33, 0x3E, 0x30, 0x18, 0x0E, 0x00}, // 9
    {0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x00}, // :
    {0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x06}, // ;
    {0x18, 0x0C, 0x06, 0x03, 0x06, 0x0C, 0x18, 0x00}, // <
    {0x00, 0x00, 0x3F, 0x00, 0x00, 0x3F, 0x00, 0x00}, // =
    {0x06, 0x0C, 0x18, 0x30, 0x18, 0x0C, 0x06, 0x00}, // >
    {0x1E, 0x33, 0x30, 0x18, 0x0C, 0x00, 0x0C, 0x00}, // ?
    {0x3E, 0x63, 0x7B, 0x7B, 0x7B, 0x03, 0x1E, 0x00}, // @
    {0x0C, 0x1E, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x00}, // A
    {0x3F, 0x66, 0x66, 0x3E, 0x66, 0x66, 0x3F, 0x00}, // B
    {0x3C, 0x66, 0x03, 0x03, 0x03, 0x66, 0x3C, 0x00}, // C
    {0x1F, 0x36, 0x66, 0x66, 0x66, 0x36, 0x1F, 0x00}, // D
    {0x7F, 0x46, 0x16, 0x1E, 0x16, 0x46, 0x7F, 0x00}, // E
    {0x7F, 0x46, 0x16, 0x1E, 0x16, 0x06, 0x0F, 0x00}, // F
    {0x3C, 0x66, 0x03, 0x03, 0x73, 0x66, 0x7C, 0x00}, // G
    {0x33, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x33, 0x00}, // H
    {0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00}, // I
    {0x78, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E, 0x00}, // J
    {0x67, 0x66, 0x36, 0x1E, 0x36, 0x66, 0x67, 0x00}, // K
    {0x0F, 0x06, 0x06, 0x06, 0x46, 0x66, 0x7F, 0x00}, // L
    {0x63, 0x77, 0x7F, 0x7F, 0x6B, 0x63, 0x63, 0x00}, // M
    {0x63, 0x67, 0x6F, 0x7B, 0x73, 0x63, 0x63, 0x00}, // N
    {0x1C, 0x36, 0x63, 0x63, 0x63, 0x36, 0x1C, 0x00}, // O
    {0x3F, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x0F, 0x00}, // P
    {0x1E, 0x33, 0x33, 0x33, 0x3B, 0x1E, 0x38, 0x00}, // Q
    {0x3F, 0x66, 0x66, 0x3E, 0x36, 0x66, 0x67, 0x00}, // R
    {0x1E, 0x33, 0x07, 0x0E, 0x38, 0x33, 0x1E, 0x00}, // S
    {0x3F, 0x2D, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00}, // T
    {0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x3F, 0x00}, // U
    {0x33, 0x33, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00}, // V
    {0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00}, // W
    {0x63, 0x63, 0x36, 0x1C, 0x1C, 0x36, 0x63, 0x00}, // X
    {0x33, 0x33, 0x33, 0x1E, 0x0C, 0x0C, 0x1E, 0x00}, // Y
    {0x7F, 0x63, 0x31, 0x18, 0x4C, 0x66, 0x7F, 0x00}, // Z
    {0x1E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x1E, 0x00}, // [
    {0x03, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00}, // backslash
    {0x1E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x1E, 0x00}, // ]
    {0x08, 0x1C, 0x36, 0x63, 0x00, 0x00, 0x00, 0x00}, // ^
    {0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF}, // _
    {0x0C, 0x0C, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00}, // `
    {0x00, 0x00, 0x1E, 0x30, 0x3E, 0x33, 0x6E, 0x00}, // a
    {0x07, 0x06, 0x06, 0x3E, 0x66, 0x66, 0x3B, 0x00}, // b
    {0x00, 0x00, 0x1E, 0x33, 0x03, 0x33, 0x1E, 0x00}, // c
    {0x38, 0x30, 0x30, 0x3e, 0x33, 0x33, 0x6E, 0x00}, // d
    {0x00, 0x00, 0x1E, 0x33, 0x3f, 0x03, 0x1E, 0x00}, // e
    {0x1C, 0x36, 0x06, 0x0f, 0x06, 0x06, 0x0F, 0x00}, // f
    {0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x1F}, // g
    {0x07, 0x06, 0x36, 0x6E, 0x66, 0x66, 0x67, 0x00}, // h
    {0x0C, 0x00, 0x0E, 0x0C, 0x0C, 0x0C, 0x1E, 0x00}, // i
    {0x30, 0x00, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E}, // j
    {0x07, 0x06, 0x66, 0x36, 0x1E, 0x36, 0x67, 0x00}, // k
    {0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00}, // l
    {0x00, 0x00, 0x33, 0x7F, 0x7F, 0x6B, 0x63, 0x00}, // m
    {0x00, 0x00, 0x1F, 0x33, 0x33, 0x33, 0x33, 0x00}, // n
    {0x00, 0x00, 0x1E, 0x33, 0x33, 0x33, 0x1E, 0x00}, // o
    {0x00, 0x00, 0x3B, 0x66, 0x66, 0x3E, 0x06, 0x0F}, // p
    {0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x78}, // q
    {0x00, 0x00, 0x3B, 0x6E, 0x66, 0x06, 0x0F, 0x00}, // r
    {0x00, 0x00, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x00}, // s
    {0x08, 0x0C, 0x3E, 0x0C, 0x0C, 0x2C, 0x18, 0x00}, // t
    {0x00, 0x00, 0x33, 0x33, 0x33, 0x33, 0x6E, 0x00}, // u
    {0x00, 0x00, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00}, // v
    {0x00, 0x00, 0x63, 0x6B, 0x7F, 0x7F, 0x36, 0x00}, // w
    {0x00, 0x00, 0x63, 0x36, 0x1C, 0x36, 0x63, 0x00}, // x
    {0x00, 0x00, 0x33, 0x33, 0x33, 0x3E, 0x30, 0x1F}, // y
    {0x00, 0x00, 0x3F, 0x19, 0x0C, 0x26, 0x3F, 0x00}, // z
    {0x38, 0x0C, 0x0C, 0x07, 0x0C, 0x0C, 0x38, 0x00}, // {
    {0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x18, 0x00}, // |
    {0x07, 0x0C, 0x0C, 0x38, 0x0C, 0x0C, 0x07, 0x00}, // }
    {0x6E, 0x3B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00}, // ~
};

/* Utility: Simplified string to int */

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
    TOK_EQ, TOK_NEQ, TOK_LT, TOK_GT, TOK_RETURN, TOK_INPUT,
    TOK_COMMA, TOK_MOD,
    TOK_RAND, TOK_EXIT,
    TOK_RECT, TOK_TEXT,
    TOK_SCANF, TOK_AND, TOK_OR, TOK_NOT, TOK_AMP,
    TOK_CPP_USING, TOK_CPP_NAMESPACE, TOK_CPP_COUT, TOK_CPP_CIN, TOK_CPP_ENDL,
    TOK_LSHIFT, TOK_RSHIFT, TOK_GE, TOK_LE,
    TOK_PEEK, TOK_POKE, TOK_MALLOC, TOK_FREE,
    TOK_VIDEOBASE,
    TOK_NEW, TOK_DELETE,
    TOK_LBRACKET, TOK_RBRACKET
} TokenType;

typedef struct {
    TokenType type;
    char str_val[MAX_STR_LEN]; /* For IDs and Strings */
    int int_val;      /* For NUMs */
} Token;

static const char *src_ptr;
static Token curr_tok;

static void next_token() {
    while (*src_ptr == ' ' || *src_ptr == '\n' || *src_ptr == '\t' || *src_ptr == '\r') src_ptr++;
    
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
            if (*src_ptr == '\\') {
                src_ptr++;
                if (*src_ptr == 'n') curr_tok.str_val[len++] = '\n';
                else if (*src_ptr == 't') curr_tok.str_val[len++] = '\t';
                else if (*src_ptr == 'r') curr_tok.str_val[len++] = '\r';
                else if (*src_ptr == '"') curr_tok.str_val[len++] = '"';
                else if (*src_ptr == '\\') curr_tok.str_val[len++] = '\\';
                else curr_tok.str_val[len++] = *src_ptr;
                if (*src_ptr) src_ptr++;
            } else {
                curr_tok.str_val[len++] = *src_ptr++;
            }
        }
        curr_tok.str_val[len] = '\0';
        if (*src_ptr == '"') src_ptr++;
        return;
    }

    /* Char Literal */
    if (*src_ptr == '\'') {
        src_ptr++;
        int val = 0;
        if (*src_ptr == '\\') {
            src_ptr++;
            if (*src_ptr == 'n') val = '\n';
            else if (*src_ptr == 't') val = '\t';
            else if (*src_ptr == '0') val = '\0';
            else val = *src_ptr;
        } else {
            val = *src_ptr;
        }
        if (*src_ptr) src_ptr++;
        if (*src_ptr == '\'') src_ptr++;
        curr_tok.type = TOK_NUM;
        curr_tok.int_val = val; 
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
        else if (len==5 && curr_tok.str_val[0]=='i' && curr_tok.str_val[1]=='n' && curr_tok.str_val[2]=='p' && curr_tok.str_val[3]=='u' && curr_tok.str_val[4]=='t') curr_tok.type = TOK_INPUT;
        else if (len==4 && curr_tok.str_val[0]=='r' && curr_tok.str_val[1]=='a' && curr_tok.str_val[2]=='n' && curr_tok.str_val[3]=='d') curr_tok.type = TOK_RAND;
        else if (len==4 && curr_tok.str_val[0]=='e' && curr_tok.str_val[1]=='x' && curr_tok.str_val[2]=='i' && curr_tok.str_val[3]=='t') curr_tok.type = TOK_EXIT;
        else if (len==4 && curr_tok.str_val[0]=='r' && curr_tok.str_val[1]=='e' && curr_tok.str_val[2]=='c' && curr_tok.str_val[3]=='t') curr_tok.type = TOK_RECT;
        else if (len==4 && curr_tok.str_val[0]=='t' && curr_tok.str_val[1]=='e' && curr_tok.str_val[2]=='x' && curr_tok.str_val[3]=='t') curr_tok.type = TOK_TEXT;
        else if (len==5 && curr_tok.str_val[0]=='s' && curr_tok.str_val[1]=='c' && curr_tok.str_val[2]=='a' && curr_tok.str_val[3]=='n' && curr_tok.str_val[4]=='f') curr_tok.type = TOK_SCANF;
        else if (len==5 && curr_tok.str_val[0]=='u' && curr_tok.str_val[1]=='s' && curr_tok.str_val[2]=='i' && curr_tok.str_val[3]=='n' && curr_tok.str_val[4]=='g') curr_tok.type = TOK_CPP_USING;
        else if (len==9 && curr_tok.str_val[0]=='n' && curr_tok.str_val[1]=='a' && curr_tok.str_val[2]=='m' && curr_tok.str_val[3]=='e' && curr_tok.str_val[4]=='s' && curr_tok.str_val[5]=='p' && curr_tok.str_val[6]=='a' && curr_tok.str_val[7]=='c' && curr_tok.str_val[8]=='e') curr_tok.type = TOK_CPP_NAMESPACE;
        else if (len==4 && curr_tok.str_val[0]=='c' && curr_tok.str_val[1]=='o' && curr_tok.str_val[2]=='u' && curr_tok.str_val[3]=='t') curr_tok.type = TOK_CPP_COUT;
        else if (len==3 && curr_tok.str_val[0]=='c' && curr_tok.str_val[1]=='i' && curr_tok.str_val[2]=='n') curr_tok.type = TOK_CPP_CIN;
        else if (len==4 && curr_tok.str_val[0]=='e' && curr_tok.str_val[1]=='n' && curr_tok.str_val[2]=='d' && curr_tok.str_val[3]=='l') curr_tok.type = TOK_CPP_ENDL;
        else if (len==4 && curr_tok.str_val[0]=='p' && curr_tok.str_val[1]=='e' && curr_tok.str_val[2]=='e' && curr_tok.str_val[3]=='k') curr_tok.type = TOK_PEEK;
        else if (len==4 && curr_tok.str_val[0]=='p' && curr_tok.str_val[1]=='o' && curr_tok.str_val[2]=='k' && curr_tok.str_val[3]=='e') curr_tok.type = TOK_POKE;
        else if (len==6 && curr_tok.str_val[0]=='m' && curr_tok.str_val[1]=='a' && curr_tok.str_val[2]=='l' && curr_tok.str_val[3]=='l' && curr_tok.str_val[4]=='o' && curr_tok.str_val[5]=='c') curr_tok.type = TOK_MALLOC;
        else if (len==4 && curr_tok.str_val[0]=='f' && curr_tok.str_val[1]=='r' && curr_tok.str_val[2]=='e' && curr_tok.str_val[3]=='e') curr_tok.type = TOK_FREE;
        else if (len==9 && curr_tok.str_val[0]=='v' && curr_tok.str_val[1]=='i' && curr_tok.str_val[2]=='d' && curr_tok.str_val[3]=='e' && curr_tok.str_val[4]=='o' && curr_tok.str_val[5]=='b' && curr_tok.str_val[6]=='a' && curr_tok.str_val[7]=='s' && curr_tok.str_val[8]=='e') curr_tok.type = TOK_VIDEOBASE;
        else if (len==3 && curr_tok.str_val[0]=='n' && curr_tok.str_val[1]=='e' && curr_tok.str_val[2]=='w') curr_tok.type = TOK_NEW;
        else if (len==6 && curr_tok.str_val[0]=='d' && curr_tok.str_val[1]=='e' && curr_tok.str_val[2]=='l' && curr_tok.str_val[3]=='e' && curr_tok.str_val[4]=='t' && curr_tok.str_val[5]=='e') curr_tok.type = TOK_DELETE;
        else curr_tok.type = TOK_ID;
        return;
    }

    char c = *src_ptr++;
    switch (c) {
        case '#': /* Preprocessor: Skip line */
            while (*src_ptr && *src_ptr != '\n') src_ptr++;
            next_token(); /* Recurse to find next real token */
            break;
        case '{': curr_tok.type = TOK_LBRACE; break;
        case '}': curr_tok.type = TOK_RBRACE; break;
        case '(': curr_tok.type = TOK_LPAREN; break;
        case ')': curr_tok.type = TOK_RPAREN; break;
        case ';': curr_tok.type = TOK_SEMI; break;
        case '+': curr_tok.type = TOK_PLUS; break;
        case '-': curr_tok.type = TOK_MINUS; break;
        case '*': curr_tok.type = TOK_MUL; break;
        case '/': 
            if (*src_ptr == '/') {
                /* Comment // */
                while (*src_ptr && *src_ptr != '\n') src_ptr++;
                next_token();
            } else if (*src_ptr == '*') {
                /* Block Comment */
                src_ptr++;
                while (*src_ptr) {
                    if (*src_ptr == '*' && *(src_ptr+1) == '/') {
                        src_ptr += 2;
                        break;
                    }
                    src_ptr++;
                }
                next_token();
            } else {
                curr_tok.type = TOK_DIV;
            }
            break;
        case '=': 
            if (*src_ptr == '=') { src_ptr++; curr_tok.type = TOK_EQ; }
            else curr_tok.type = TOK_ASSIGN; 
            break;
        case ',': curr_tok.type = TOK_COMMA; break;
        case '[': curr_tok.type = TOK_LBRACKET; break;
        case ']': curr_tok.type = TOK_RBRACKET; break;
        case '%': curr_tok.type = TOK_MOD; break;
        case '&': 
            if (*src_ptr == '&') { src_ptr++; curr_tok.type = TOK_AND; }
            else curr_tok.type = TOK_AMP;
            break;
        case '|':
            if (*src_ptr == '|') { src_ptr++; curr_tok.type = TOK_OR; }
            else kprint("Err: Unknown char '|'\n");
            break;
        case '!':
            if (*src_ptr == '=') { src_ptr++; curr_tok.type = TOK_NEQ; }
            else curr_tok.type = TOK_NOT;
            break;
        case '>':
            if (*src_ptr == '=') { src_ptr++; curr_tok.type = TOK_GE; }
            else if (*src_ptr == '>') { src_ptr++; curr_tok.type = TOK_RSHIFT; } /* >> */
            else curr_tok.type = TOK_GT;
            break;
        case '<':
            if (*src_ptr == '=') { src_ptr++; curr_tok.type = TOK_LE; }
            else if (*src_ptr == '<') { src_ptr++; curr_tok.type = TOK_LSHIFT; } /* << */
            else curr_tok.type = TOK_LT;
            break;
        default: kprint("Err: Unknown token\n"); break;
    }
}

/* ================= COMPILER / CODEGEN ================= */

#define MAX_CODE 4096
static int code[MAX_CODE];
static int pc = 0;

/* Symbol Table */
static char var_names[MAX_VARS][32];
static int var_count = 0;

static int get_var_index(const char *name) {
    /* Search existing */
    for (int i=0; i<var_count; i++) {
        if (strcmp(var_names[i], name) == 0) return i;
    }
    /* Add new */
    if (var_count < MAX_VARS) {
        strcpy(var_names[var_count], name);
        return var_count++;
    }
    kprint("Err: Too many vars\n");
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
    } else if (curr_tok.type == TOK_INPUT) {
        emit(OP_INPUT);
        next_token();
    } else if (curr_tok.type == TOK_NEW) {
        /* new int or new int[size] */
        next_token();
        if (curr_tok.type == TOK_INT) next_token(); /* Skip type */
        
        if (curr_tok.type == TOK_LBRACKET) {
             next_token();
             parse_expr(); /* Size */
             if (curr_tok.type == TOK_RBRACKET) next_token();
             /* Size is on stack. Multiply by 8 (sizeof(int64)) */
             emit(OP_IMM); emit(8);
             emit(OP_MUL);
        } else {
             /* Scalar: Size 8 */
             emit(OP_IMM); emit(8);
        }
        emit(OP_MALLOC);
    } else if (curr_tok.type == TOK_RAND) {
        emit(OP_RAND);
        next_token();
        /* handle optional () for rand() */
        if (curr_tok.type == TOK_LPAREN) {
            next_token();
            if (curr_tok.type == TOK_RPAREN) next_token();
        }
    }
}

/* Precedence Helper */
static void parse_unary() {
    if (curr_tok.type == TOK_NOT) {
        next_token();
        parse_unary();
        emit(OP_NOT);
    } else {
        parse_factor();
    }
}

static void parse_term() {
    parse_unary();
    while (curr_tok.type == TOK_MUL || curr_tok.type == TOK_DIV || curr_tok.type == TOK_MOD) {
        TokenType op = curr_tok.type;
        next_token();
        parse_unary();
        if (op == TOK_MUL) emit(OP_MUL);
        else if (op == TOK_DIV) emit(OP_DIV);
        else emit(OP_MOD);
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

static void parse_relational() {
    parse_additive();
    /* Comparisons */
    if (curr_tok.type == TOK_LT || curr_tok.type == TOK_GT || curr_tok.type == TOK_EQ || curr_tok.type == TOK_NEQ) {
        TokenType op = curr_tok.type;
        next_token();
        parse_additive();
        if (op == TOK_LT) emit(OP_CMP_LT);
        else if (op == TOK_GT) emit(OP_CMP_GT);
        else if (op == TOK_EQ) emit(OP_CMP_EQ);
        else if (op == TOK_NEQ) { emit(OP_CMP_EQ); emit(OP_NOT); } /* NEQ is !(==) */
    }
}

static void parse_logic_and() {
    parse_relational();
    while (curr_tok.type == TOK_AND) {
        next_token();
        parse_relational();
        emit(OP_AND);
    }
}

static void parse_logic_or() {
    parse_logic_and();
    while (curr_tok.type == TOK_OR) {
        next_token();
        parse_logic_and();
        emit(OP_OR);
    }
}

static void parse_expr() {
    parse_logic_or();
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
        if (curr_tok.type == TOK_SEMI) next_token();
    } else if (curr_tok.type == TOK_CPP_USING) {
        /* using namespace std; - Just ignore until semicolon */
        next_token();
        while (curr_tok.type != TOK_SEMI && curr_tok.type != TOK_EOF) next_token();
        if (curr_tok.type == TOK_SEMI) next_token();
    } else if (curr_tok.type == TOK_CPP_COUT) {
        next_token();
        /* cout << expr << expr ... ; */
        while (curr_tok.type == TOK_LSHIFT) {
            next_token();
            if (curr_tok.type == TOK_CPP_ENDL) {
                 emit(OP_PRINT_STR); emit(add_string("\n")); /* Primitive newline */
                 next_token();
            } else if (curr_tok.type == TOK_STRING) {
                emit(OP_PRINT_STR);
                emit(add_string(curr_tok.str_val));
                next_token();
            } else {
                parse_expr(); /* Pushes value on stack (int) */
                emit(OP_PRINT); /* Prints int */
            }
        }
        if (curr_tok.type == TOK_SEMI) next_token();
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
                next_token();
                
                if (curr_tok.type == TOK_COMMA) {
                    /* Handle arguments for PrintF */
                    int argc = 0;
                    while (curr_tok.type == TOK_COMMA) {
                        next_token();
                        parse_expr(); /* Push arg */
                        argc++;
                    }
                    emit(OP_PRINT_FMT);
                    emit(str_idx);
                    emit(argc);
                } else {
                    emit(OP_PRINT_STR);
                    emit(str_idx);
                }
            } else {
                parse_expr();
                emit(OP_PRINT);
            }
            
            if (curr_tok.type == TOK_RPAREN) next_token();
            if (curr_tok.type == TOK_SEMI) next_token();
        }
    } else if (curr_tok.type == TOK_PRINT) {
        next_token();
        if (curr_tok.type == TOK_LPAREN) next_token();
        if (curr_tok.type == TOK_STRING) {
            emit(OP_PRINT_STR);
            emit(add_string(curr_tok.str_val));
            next_token();
        } else {
            parse_expr();
            emit(OP_PRINT);
        }
        if (curr_tok.type == TOK_RPAREN) next_token();
        if (curr_tok.type == TOK_SEMI) next_token();
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
        if (curr_tok.type == TOK_RPAREN) next_token();
        emit(OP_MALLOC);
    } else if (curr_tok.type == TOK_VIDEOBASE) {
        next_token();
        if (curr_tok.type == TOK_LPAREN) {
             next_token();
             if (curr_tok.type == TOK_RPAREN) next_token();
        }
        emit(OP_VIDEOBASE);
    } else if (curr_tok.type == TOK_NUM) {
        next_token();
        if (curr_tok.type == TOK_LPAREN) { /* exit(0) */
            next_token();
            parse_expr(); /* consume arg but ignore for now */
            if (curr_tok.type == TOK_RPAREN) next_token();
        }
        if (curr_tok.type == TOK_SEMI) next_token();
        emit(OP_EXIT);
    } else if (curr_tok.type == TOK_EXIT) {
        next_token();
        if (curr_tok.type == TOK_LPAREN) { /* exit(0) */
            next_token();
            parse_expr(); /* consume arg but ignore for now */
            if (curr_tok.type == TOK_RPAREN) next_token();
        }
        if (curr_tok.type == TOK_SEMI) next_token();
        emit(OP_EXIT);
    } else if (curr_tok.type == TOK_RECT) { /* rect(x,y,w,h,c) */
        next_token();
        if (curr_tok.type == TOK_LPAREN) next_token();
        parse_expr(); /* x */
        if (curr_tok.type == TOK_COMMA) next_token();
        parse_expr(); /* y */
        if (curr_tok.type == TOK_COMMA) next_token();
        parse_expr(); /* w */
        if (curr_tok.type == TOK_COMMA) next_token();
        parse_expr(); /* h */
        if (curr_tok.type == TOK_COMMA) next_token();
        parse_expr(); /* color */
        if (curr_tok.type == TOK_RPAREN) next_token();
        if (curr_tok.type == TOK_SEMI) next_token();
        emit(OP_GFX_RECT);
    } else if (curr_tok.type == TOK_TEXT) { /* text(x,y,str,c) */
        next_token();
        if (curr_tok.type == TOK_LPAREN) next_token();
        parse_expr(); /* x */
        if (curr_tok.type == TOK_COMMA) next_token();
        parse_expr(); /* y */
        if (curr_tok.type == TOK_COMMA) next_token();
        
        /* String */
        int str_idx = 0;
        if (curr_tok.type == TOK_STRING) {
            str_idx = add_string(curr_tok.str_val);
            next_token();
        }
        
        if (curr_tok.type == TOK_COMMA) next_token();
        parse_expr(); /* color */
        if (curr_tok.type == TOK_RPAREN) next_token();
        if (curr_tok.type == TOK_SEMI) next_token();
        
        
        emit(OP_GFX_TEXT);
        emit(str_idx);
    } else if (curr_tok.type == TOK_SCANF) { /* scanf("%d", &var) */
        next_token();
        if (curr_tok.type == TOK_LPAREN) next_token();
        /* Format string (ignored for now, assuming %d) */
        if (curr_tok.type == TOK_STRING) next_token();
        if (curr_tok.type == TOK_COMMA) next_token();
        
        /* &var */
        if (curr_tok.type == TOK_AMP) next_token();
        if (curr_tok.type == TOK_ID) {
            int var_idx = get_var_index(curr_tok.str_val);
            next_token();
            emit(OP_SCANF);
            emit(var_idx);
        }
        
        if (curr_tok.type == TOK_RPAREN) next_token();
        if (curr_tok.type == TOK_SEMI) next_token();
        if (curr_tok.type == TOK_SEMI) next_token();
    } else if (curr_tok.type == TOK_POKE) {
        next_token();
        if (curr_tok.type == TOK_LPAREN) next_token();
        parse_expr(); /* Address */
        if (curr_tok.type == TOK_COMMA) next_token();
        parse_expr(); /* Value */
        if (curr_tok.type == TOK_RPAREN) next_token();
        if (curr_tok.type == TOK_SEMI) next_token();
        emit(OP_POKE);
    } else if (curr_tok.type == TOK_DELETE) {
        next_token();
        if (curr_tok.type == TOK_LBRACKET) { /* delete[] */
            next_token();
            if (curr_tok.type == TOK_RBRACKET) next_token();
        }
        parse_expr(); /* Pointer */
        emit(OP_FREE);
        if (curr_tok.type == TOK_SEMI) next_token();
    } else if (curr_tok.type == TOK_FREE) {
        next_token();
        if (curr_tok.type == TOK_LPAREN) next_token();
        parse_expr(); /* Address */
        if (curr_tok.type == TOK_RPAREN) next_token();
        if (curr_tok.type == TOK_SEMI) next_token();
        emit(OP_FREE);
    } else {
        /* Empty or unknown */
        if (curr_tok.type != TOK_RBRACE && curr_tok.type != TOK_EOF) next_token();
    }
}

/* ================= VM EXECUTION ================= */

static int64_t stack[1024];
static int sp = 0;
/* globals moved to top */
static int64_t globals[MAX_VARS];

/* Preprostessor: Handle #include */
static char *preprocess(const char *src) {
    if (!src) return NULL;
    
    int len = strlen(src);
    int capacity = len + 8192; /* generous buffer for includes */
    char *out = (char*)kmalloc(capacity);
    if (!out) return (char*)src;
    
    int out_idx = 0;
    const char *p = src;
    
    while (*p) {
        /* Check for #include */
        if (*p == '#' && strncmp(p, "#include", 8) == 0) {
             p += 8;
             while (*p == ' ' || *p == '\t') p++;
             
             char filename[64];
             int f_idx = 0;
             int is_std = 0;
             
             if (*p == '<') { is_std = 1; p++; }
             else if (*p == '"') { is_std = 0; p++; }
             
             while (*p && *p != '>' && *p != '"' && *p != '\n') {
                 if (f_idx < 63) filename[f_idx++] = *p;
                 p++;
             }
             if (*p == '>' || *p == '"') p++;
             filename[f_idx] = 0;
             
             /* Resolve path */
             char path[128];
             if (is_std) {
                 strcpy(path, "/lib/");
                 strcat(path, filename);
             } else {
                 if (filename[0] == '/') strcpy(path, filename);
                 else {
                     strcpy(path, "/");
                     strcat(path, filename);
                 }
             }
             
             struct initrd_file *f = initrd_find_file(path);
             /* Try without leading / if fail */
             if (!f && path[0] == '/') f = initrd_find_file(path+1);
             
             if (f) {
                 if (out_idx + f->size >= capacity) {
                     capacity += f->size + 4096;
                     char *new_out = (char*)kmalloc(capacity);
                     memcpy(new_out, out, out_idx);
                     kfree(out);
                     out = new_out;
                 }
                 memcpy(out + out_idx, f->data, f->size);
                 out_idx += f->size;
                 out[out_idx++] = '\n';
             } else {
                 kprint("[PP] Include not found: "); kprint(path); kprint("\n");
             }
             
             while (*p && *p != '\n') p++;
        } else {
            if (out_idx >= capacity - 1) {
                capacity += 4096;
                 char *new_out = (char*)kmalloc(capacity);
                 memcpy(new_out, out, out_idx);
                 kfree(out);
                 out = new_out;
            }
            out[out_idx++] = *p++;
        }
    }
    out[out_idx] = 0;
    return out;
}

void nano_c_run(const char *source) {
    kprint("[NanoC] Entered.\n");
    if (!source) return;
    
    /* char *processed = preprocess(source); */
    const char *processed = source;
    
    /* Reset Compiler State */
    src_ptr = processed;
    pc = 0;
    sp = 0;
    str_count = 0;
    var_count = 0;
    for(int i=0; i<MAX_VARS; i++) globals[i]=0;
    
    kprint("[Compiler] Compiling...\n");
    kprint("[Debug] Priming Lexer...\n");
    next_token(); /* Prime lexer */
    kprint("[Debug] First Token: "); print_n(curr_tok.type);
    
    /* Parse until EOF */
    /* ... (logic) ... */
    if (curr_tok.type == TOK_INT) {
        /* Check if it is main() */
        /* ... */
        /* Since parsing logic is fragile, let's just loop and let parse_stmt handle it or skip top level structs */
        /* But wait, if main is skipped, we need to ensure we don't skip the BODY content if we treat it as global */
        /* The previous logic was: */
        next_token();
        if (curr_tok.type == TOK_ID) { 
             next_token();
             if (curr_tok.type == TOK_LPAREN) {
                 next_token();
                 if (curr_tok.type == TOK_RPAREN) next_token();
                 if (curr_tok.type == TOK_LBRACE) next_token();
             }
        }
    }

    while (curr_tok.type != TOK_EOF && curr_tok.type != TOK_RBRACE) {
        kprint("[Compiler] Parsing Stmt. Token: "); print_n(curr_tok.type);
        parse_stmt();
    }
    emit(OP_EXIT);
    
    kprint("[VM] Compiled instructions: "); print_n(pc);
    kprint("[VM] Running...\n");
    
    /* VM Loop */
    int ip = 0;
    while (ip < pc) {
        /* Check Interrupts */
        keyboard_poll();
        if (keyboard_is_ctrl_active()) {
             if (keyboard_available()) {
                 char c = keyboard_getchar();
                 if (c == 'c' || c == 'C') {
                     kprint("\n^C\n");
                     goto done;
                 }
                 if (c == 'z' || c == 'Z') {
                     kprint("\n^Z\n");
                     goto done;
                 }
             }
        }

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
            
            /* PEEK: Pop addr, Push val */
            case OP_PEEK: {
                uint64_t addr = (uint64_t)stack[sp-1];
                /* Safety Check? Nah, live dangerously for "Power" */
                /* Actually we can map "Globals" or direct memory. */
                /* cast to int* and read */
                int *ptr = (int*)addr;
                stack[sp-1] = *ptr;
                break;
            }
            /* POKE: Pop val, Pop addr */
            case OP_POKE: {
                int val = stack[--sp];
                uint64_t addr = (uint64_t)stack[--sp];
                int *ptr = (int*)addr;
                *ptr = val;
                break;
            }
            /* MALLOC: Pop size, Push addr */
            case OP_MALLOC: {
                int size = stack[sp-1];
                void *ptr = kmalloc(size);
                stack[sp-1] = (uint64_t)ptr; /* Return address as int */
                break;
            }
            /* FREE: Pop addr */
            case OP_FREE: {
                uint64_t addr = (uint64_t)stack[--sp];
                kfree((void*)addr);
                break;
            }
            case OP_VIDEOBASE: {
                /* Expose Framebuffer Address */
                uint64_t w, h, p;
                void *addr;
                gfx_get_info(&w, &h, &p, &addr);
                stack[sp++] = (uint64_t)addr;
                break;
            }
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
            case OP_INPUT: {
                char c = keyboard_getchar();
                /* Echo input */
                char s[2] = {c, 0};
                kprint(s);
                stack[sp++] = (int)c;
                break;
            }
            case OP_MOD: sp--; if(stack[sp]!=0) stack[sp-1] = stack[sp-1] % stack[sp]; else stack[sp-1]=0; break;
            case OP_PRINT_FMT: {
                 int str_idx = code[ip++];
                 int argc = code[ip++];
                 
                 /* Pop args into temporary buffer (reverse order) */
                 int args[16]; 
                 for (int i = 0; i < argc; i++) {
                     sp--;
                     args[argc - 1 - i] = stack[sp]; 
                 }
                 
                 const char *fmt = string_table[str_idx];
                 int arg_i = 0;
                 while (*fmt) {
                     if (*fmt == '%' && arg_i < argc) {
                         fmt++;
                         if (*fmt == 'd') { print_n(args[arg_i++]); fmt++; }
                         else if (*fmt == 'c') { 
                            char c[2] = {(char)args[arg_i++], 0}; 
                            kprint(c); 
                            fmt++; 
                         }
                         else if (*fmt == 's') {
                             /* String pointer? Not supported yet as args are ints... unless we map */
                             kprint("<STR>"); fmt++; arg_i++;
                         } else { kprint("%"); }
                     } else {
                         char c[2] = {*fmt, 0};
                         kprint(c);
                         fmt++;
                     }
                 }
                 break;
            }
            case OP_RAND: {
                static unsigned long next = 1;
                next = next * 1103515245 + 12345;
                stack[sp++] = (unsigned int)(next/65536) % 32768;
                break;
            }
            case OP_GFX_RECT: {
                uint32_t color = stack[--sp];
                int h = stack[--sp];
                int w = stack[--sp];
                int y = stack[--sp];
                int x = stack[--sp];
                gfx_fill_rect(x, y, w, h, color);
                break;
            }
            case OP_GFX_TEXT: {
                int str_idx = code[ip++];
                uint32_t color = stack[--sp];
                int y = stack[--sp];
                int x = stack[--sp];
                
                /* Simple text draw using font_8x8 from font.h */
                const char *s = string_table[str_idx];
                int cur_x = x;
                while (*s) {
                    char c = *s++;
                    /* This is basically gfx_draw_char_transparent logic if exists */
                    /* But we don't have it exposed in gfx.h? font.h only has data. */
                    /* Assume kernel/gfx.c has gfx_draw_char or we implement it manually? */
                    /* We don't have direct access to gfx functions except those in gfx.h */
                    /* Let's assume we can add gfx_draw_char later or use direct buffer? No, VM is kernel. */
                    /* Let's assume gfx_draw_char exists or fallback to rects? Rects is slow. */
                    /* Let's just assume we can call `draw_char_8x8` if we copy it? */
                    /* For now, just print to console as fallback? No user wants graphics. */
                    /* I'll use kprint for now if GFX not ready, OR check gfx.c source quickly? */
                    /* Better: Draw using put_pixel loop here. */
                    if (c >= 32 && c <= 126) {
                        const uint8_t *glyph = font_8x8_script[c - 32];
                        for (int row=0; row<8; row++) {
                            for (int col=0; col<8; col++) {
                                if ((glyph[row] >> col) & 1) {
                                    gfx_put_pixel_safe(cur_x + col, y + row, color);
                                }
                            }
                        }
                    }
                    cur_x += 8;
                }
                break;
            }
            case OP_SCANF: {
                int var_idx = code[ip++];
                /* Read integer from keyboard until newline */
                int val = 0;
                int sign = 1;
                /* Non-blocking input loop with Ctrl+C check */
                char c = 0;
                
                /* Wait for first char (skipping whitespace) */
                while (1) {
                    keyboard_poll();
                    if (keyboard_is_ctrl_active()) {
                         /* If we see 'c' or 'z' we should abort. But getting scan code here is hard without consuming it. */
                         /* For simplicity, just abort if Ctrl is held? No. */
                         /* Let's peek? No peek. */
                         /* Let's just consume char. If it is 'c'/'z' AND Ctrl, abort. */
                         /* Else put back? We can't unget. */
                    }
                    if (keyboard_available()) {
                        c = keyboard_getchar();
                        /* Check for Signal */
                        if (keyboard_is_ctrl_active()) {
                             if (c == 'c' || c == 'C') { kprint("^C\n"); goto done; }
                             if (c == 'z' || c == 'Z') { kprint("^Z\n"); goto done; }
                        }
                        
                        if (c == ' ' || c == '\n') continue;
                        break;
                    }
                }
                 
                /* Sign */
                if (c == '-') { sign = -1; c = 0; }
                
                /* Digits */
                while (1) {
                    if (c >= '0' && c <= '9') {
                        char s[2] = {c, 0};
                        kprint(s); /* Echo */
                        val = val * 10 + (c - '0');
                    } else if (c == '\n' || c == ' ') {
                        break;
                    }

                    /* Get next char */
                    while (!keyboard_available()) {
                         keyboard_poll();
                    }
                    c = keyboard_getchar();
                    if (keyboard_is_ctrl_active() && (c == 'c' || c == 'C')) { kprint("^C\n"); goto done; }
                }
                
                /* End of num */
                kprint("\n");
                globals[var_idx] = val * sign;
                break;
            }
            case OP_AND: { int b=stack[--sp]; int a=stack[--sp]; stack[sp++] = (a && b); break; }
            case OP_OR:  { int b=stack[--sp]; int a=stack[--sp]; stack[sp++] = (a || b); break; }
            case OP_NOT: { stack[sp-1] = !stack[sp-1]; break; }
        }
    }
done:
    kprint("\n[VM] Done.\n");
    
    /* Garbage Collection */
    if (processed != source && processed) {
        kfree(processed);
    }
}
