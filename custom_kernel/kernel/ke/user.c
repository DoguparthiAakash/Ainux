#include "user.h"
#include "shell.h"
#include "io/fat32.h"
#include "libc/string.h"
#include "libc/stdlib.h"
#include "drivers/keyboard.h"
#include "mm/heap.h"
#include "aes.h" 
#include "log.h"

/* Simple User Database Format:
   username:password\n
   
   Stored at /users.db (ENCRYPTED with AES-128 ECB)
*/

#define USER_DB_PATH "users.db"
#define MAX_LINE_LEN 128

/* Hardcoded 128-bit key for demonstration. 
   In a real OS, this might be sealed in TPM or obfuscated. */
static const uint8_t aes_key[16] = { 
    0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, // Secret
    0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c  // Key
};

extern void kprint(const char *msg);
extern void term_clear(void);

/* Helper: Read a line from keyboard with echo masking */
void user_get_input_masked(char *buf, int max) {
    int i = 0;
    while (1) {
        if (keyboard_available()) {
            char c = keyboard_getchar();
            if (c == '\n' || c == '\r') {
                buf[i] = '\0';
                kprint("\n");
                return;
            } else if (c == '\b') {
                if (i > 0) {
                    i--;
                    kprint("\b \b"); 
                }
            } else if (c >= 32 && c <= 126) {
                if (i < max - 1) {
                    buf[i++] = c;
                    kprint("*");
                }
            }
        }
    }
}

void user_get_input(char *buf, int max) {
    int i = 0;
    while (1) {
        if (keyboard_available()) {
            char c = keyboard_getchar();
            if (c == '\n' || c == '\r') {
                buf[i] = '\0';
                kprint("\n");
                return;
            } else if (c == '\b') {
                if (i > 0) {
                    i--;
                    kprint("\b \b");
                }
            } else if (c >= 32 && c <= 126) {
                if (i < max - 1) {
                    buf[i++] = c;
                    char tmp[2] = {c, 0};
                    kprint(tmp);
                }
            }
        }
    }
}

/* Fallback RAM Users for LiveISO/Disk Error cases */
typedef struct {
    char username[64];
    char password[64];
    int active;
} ram_user_t;

static ram_user_t ram_users[4]; /* Support up to 4 users in RAM */
static int use_ram_fallback = 0;

/* --- Encryption Helpers --- */

// Pad input to 16-byte boundary using PKCS#7
// Returns new size, allocates new buffer. Caller must free.
// *out_buf set to new buffer.
static uint32_t encrypt_data(uint8_t *in_buf, uint32_t in_size, uint8_t **out_buf) {
    uint8_t padding = 16 - (in_size % 16);
    uint32_t new_size = in_size + padding;
    
    *out_buf = kmalloc(new_size);
    memcpy(*out_buf, in_buf, in_size);
    
    // Apply PKCS#7 padding
    for (uint32_t i = in_size; i < new_size; i++) {
        (*out_buf)[i] = padding;
    }
    
    struct AES_ctx ctx;
    AES_init_ctx(&ctx, aes_key);
    
    for (uint32_t i = 0; i < new_size; i += 16) {
        AES_ECB_encrypt(&ctx, *out_buf + i);
    }
    
    return new_size;
}

// Decrypts data, removes padding.
// Returns plain size. Allocates new buffer `out_buf`.
static uint32_t decrypt_data(uint8_t *in_buf, uint32_t in_size, uint8_t **out_buf) {
    if (in_size == 0 || in_size % 16 != 0) return 0; // Invalid
    
    uint8_t *temp = kmalloc(in_size);
    memcpy(temp, in_buf, in_size);
    
    struct AES_ctx ctx;
    AES_init_ctx(&ctx, aes_key);
    
    for (uint32_t i = 0; i < in_size; i += 16) {
        AES_ECB_decrypt(&ctx, temp + i);
    }
    
    // Check Padding
    uint8_t padding = temp[in_size - 1];
    if (padding == 0 || padding > 16) {
         // Bad padding or wrong key?
         kfree(temp);
         return 0;
    }
    
    uint32_t plain_size = in_size - padding;
    *out_buf = kmalloc(plain_size + 1); // +1 for safety null terminator
    memcpy(*out_buf, temp, plain_size);
    (*out_buf)[plain_size] = 0;
    
    kfree(temp);
    return plain_size;
}


void user_init(void) {
    /* Check if DB exists */
    uint8_t *data;
    uint32_t size;
    int res = fat32_read_file(USER_DB_PATH, &data, &size);
    if (res != 0) {
        /* Failed to read from disk. Could be missing file or disk error. */
    } else {
        kfree(data);
    }
    
    /* Clear RAM users */
    for(int i=0; i<4; i++) ram_users[i].active = 0;
}

int user_add(const char *username, const char *password) {
    /* Try Disk First */
    if (!use_ram_fallback) {
        /* Read existing file first */
        uint8_t *enc_data = NULL;
        uint32_t enc_size = 0;
        
        uint8_t *plain_data = NULL;
        uint32_t plain_size = 0;

        char line[MAX_LINE_LEN];
        strcpy(line, username);
        strcat(line, ":");
        strcat(line, password);
        strcat(line, "\n");
        size_t line_len = strlen(line);
        
        int write_success = 0;
        
        // 1. Read and Decrypt existing DB
        if (fat32_read_file(USER_DB_PATH, &enc_data, &enc_size) == 0) {
             plain_size = decrypt_data(enc_data, enc_size, &plain_data);
             kfree(enc_data);
             if (plain_size == 0) {
                 // Decryption fail (or empty/invalid file). Overwrite?
                 // Let's assume empty.
             }
        }
        
        // 2. Append new user
        uint32_t new_plain_size = plain_size + line_len;
        uint8_t *new_plain_buf = kmalloc(new_plain_size);
        if (plain_data) {
            memcpy(new_plain_buf, plain_data, plain_size);
            kfree(plain_data);
        }
        memcpy(new_plain_buf + plain_size, line, line_len);
        
        // 3. Encrypt new combined buffer
        uint8_t *final_enc_buf;
        uint32_t final_enc_size = encrypt_data(new_plain_buf, new_plain_size, &final_enc_buf);
        kfree(new_plain_buf);
            
        // 4. Write back
        if (fat32_write_file(USER_DB_PATH, final_enc_buf, final_enc_size) == 0) {
            write_success = 1;
        }
        kfree(final_enc_buf);
        
        if (write_success) return 0;
        
        kprint("Disk Write Failed! Switching to ephemeral RAM storage for this session.\n");
        use_ram_fallback = 1;
    }
    
    /* RAM Fallback */
    for(int i=0; i<4; i++) {
        if (!ram_users[i].active) {
            strcpy(ram_users[i].username, username);
            strcpy(ram_users[i].password, password);
            ram_users[i].active = 1;
            return 0;
        }
    }
    kprint("RAM User DB Full!\n");
    return -1;
}

int user_check(const char *username, const char *password) {
    if (!use_ram_fallback) {
        uint8_t *enc_data;
        uint32_t enc_size;
        
        if (fat32_read_file(USER_DB_PATH, &enc_data, &enc_size) == 0) {
            /* 1. Decrypt */
            uint8_t *plain_data = NULL;
            uint32_t plain_size = decrypt_data(enc_data, enc_size, &plain_data);
            kfree(enc_data);
            
            if (plain_size > 0 && plain_data) {
                /* Disk Check Logic (on plain_data) */
                char *p = (char*)plain_data;
                char *end = (char*)plain_data + plain_size;
                char file_user[64];
                char file_pass[64];
                
                int found = 0;
                while (p < end && *p) {
                    while (p < end && (*p == 0 || *p == '\xff')) p++;
                    if (p >= end) break;
    
                    int i=0;
                    while(p < end && *p != ':' && *p != '\n' && *p != 0 && i < 63) file_user[i++] = *p++;
                    file_user[i] = 0;
                    if (*p == ':') p++;
                    
                    i=0;
                    while(p < end && *p != '\n' && *p != 0 && i < 63) {
                         if (*p != '\r') file_pass[i++] = *p;
                         p++;
                    }
                    file_pass[i] = 0;
                    if (p < end && *p == '\n') p++;
                    
                    if (strcmp(username, file_user) == 0 && strcmp(password, file_pass) == 0) {
                        found = 1;
                        break;
                    }
                }
                kfree(plain_data);
                if (found) return 1;
            }
        }
    }
    
    /* Check RAM Users */
    for(int i=0; i<4; i++) {
        if (ram_users[i].active) {
            if (strcmp(username, ram_users[i].username) == 0 && strcmp(password, ram_users[i].password) == 0) {
                return 1;
            }
        }
    }
    
    return 0;
}

static char current_user[64] = "root";

const char* user_get_current(void) {
    return current_user;
}

void user_set_current(const char *username) {
    if (username) {
        char *d = current_user;
        const char *s = username;
        int i=0;
        while(*s && i < 63) { *d++ = *s++; i++; }
        *d = 0;
    }
}

int user_login_loop(void) {
    char user[64];
    char pass[64];
    
    /* Check if DB exists. If not, First Run Setup. */
    uint8_t *dummy;
    uint32_t dsize;
    
    /* Note: If DB exists but is technically garbage/unencrypted from previous run, 
       decrypt_data will likely fail validation and user_check will return false.
       But user_add will overwrite it properly.
       
       However, we check existence here. If it exists, we assume we need to login.
    */
    
    if (fat32_read_file(USER_DB_PATH, &dummy, &dsize) != 0) {
        term_clear();
        kprint("=== First Run Setup ===\n");
        kprint("No users found (or error). Create Root Account.\n\n");
        
        while(1) {
            kprint("New Username: ");
            user_get_input(user, 64);
            if (strlen(user) > 0) break;
        }
        
        while(1) {
            kprint("New Password: ");
            user_get_input_masked(pass, 64);
            char confirm[64];
            kprint("Confirm Password: ");
            user_get_input_masked(confirm, 64);
            
            if (strcmp(pass, confirm) == 0) break;
            kprint("Passwords do not match. Try again.\n");
        }
        
        if (user_add(user, pass) == 0) {
            kprint("User created! Please login.\n");
        } else {
            kprint("Error: Failed to save user to disk!\n");
        }
        /* Fall through to login loop */
    } else {
        kfree(dummy);
    }
    
    while (1) {
        term_clear();
        /* Premium ASCII Banner */
        kprint_color(KLOG_COLOR_CYAN, "\n    ___    _             _      \n   /   |  (_)___  __  __(_)_  __\n  / /| | / / __ \\/ / / / /| |/_/\n / ___ |/ / / / / /_/ / />  <  \n/_/  |_/_/_/ /_/\\__,_/_/_/|_|  \n\n");
        kprint_color(KLOG_COLOR_RESET, "      Welcome to Ainuix OS\n");
        kprint_color(KLOG_COLOR_RESET, "      --------------------\n\n");
        
        kprint("Username: ");
        user_get_input(user, 64);
        
        kprint("Password: ");
        user_get_input_masked(pass, 64);
        
        if (user_check(user, pass)) {
            user_set_current(user);
            kprint("\nLogin Successful.\n");
            return 1;
        } else {
            kprint("\nInvalid Credentials. Try again.\n");
            
            /* Debug: Print info about failure */
            // We can't easily debug encrypted file content without decrypting, 
            // and we already tried that validation in user_check.
            
            /* sleep */
            for(volatile int i=0; i<10000000; i++);
        }
    }
    return 0;
}
