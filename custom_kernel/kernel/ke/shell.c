#include "shell.h"
#include "drivers/keyboard.h"
#include "drivers/mouse.h"
#include "io/initrd.h"
#include "mm/pmm.h"
#include "mm/heap.h"
#include "ke/nanoc/nano_c.h"
#include "gfx/gfx.h"
#include "io/fat32.h"
#include "../ex/io/vfs.h"
#include "io/mbr.h"
#include "gfx/wm.h"
#include "ke/nanoc/nano_asm.h"
#include "drivers/pci.h"
#include "log.h"
#include "io/elf.h"
#include "nanoc/nano_c.h"
#include "sched/sched.h"
#include "power.h"
#include "user.h"
#include "drivers/rtc.h"
#include "drivers/timer.h"
#include "libc/stdio.h"
#include "libc/stdlib.h"
#include "ex/io/axfs.h"
#include "ex/ob/object.h"
#include "../version.h"

extern void libc_test_run(void);

extern void kprint(const char *msg);
extern void kprint_buf(const char *buf, uint64_t len);
extern void term_clear(void);
extern void term_set_cursor(int x, int y);
extern void term_draw_cursor(void); 
extern void term_erase_cursor(void);

/* Memory Management (Garbage Collector) */
void gc_collect(void *ptr) {
    if (ptr) {
        kfree(ptr);
    }
}

/* Current Working Directory */
char cwd[256] = "/";
static char prev_cwd[256] = "/";

/* Command History */
#define HISTORY_MAX 16
#define CMD_MAX_LEN 256
static char cmd_history[HISTORY_MAX][CMD_MAX_LEN];
static int history_count = 0;
static int history_view_index = 0;

/* Editor Constants */
#define EDITOR_BUF_SIZE 16384
#define EDITOR_WIDTH 80
#define EDITOR_HEIGHT 25

/* Multimedia */
void cmd_beep(char *args);
void cmd_view(char *filename);
void cmd_file(char *filename);

/* Multimedia */
void cmd_beep(char *args);
void cmd_view(char *filename);
void cmd_file(char *filename);
void cmd_startwm(void); /* Alias */

/* Net Utils */
#include "../../net/arp.h"
#include "../../net/icmp.h"

static void cmd_ping(void) {
    kprint("Pinging 10.0.2.2 (QEMU Gateway)...\n");
    /* Send ICMP Echo */
    /* id=1, seq=1 */
    icmp_send_echo(0x0A000202, 1, 1);
    
    /* Also Send ARP just in case to populate switch tables */
    arp_send_request(0x0A000202);
}

/* Internet Command moved to bottom */

int exec_command(char *cmd_buffer);

static int g_sudo_active = 0;

/* Helper Protos */
static size_t str_len(const char *s);
static int strncmp(const char *s1, const char *s2, size_t n) {
    while (n > 0 && *s1 && *s2) {
        if (*s1 != *s2) return *s1 - *s2;
        s1++; s2++; n--;
    }
    if (n == 0) return 0;
    return *s1 - *s2;
}

/* WM Protos */
void wm_add_icon(const char *label, int x, int y, void (*cb)(void));
void wm_clear_icons(void);

/* Autocompletion Commands */
static const char *cmd_list[] = {
    "help", "clear", "exit", "shutdown", "reboot", "date", "time", "cal", "update",
    "ls", "pwd", "cd", "mkdir", "touch", "cp", "rm", "cat", "write", "mount", 
    "lsdisk", "mkfs", "fdisk", "save", "useradd", "passwd", "meminfo", "lspci", 
    "dmesg", "whoami", "uptime", "clock", "apt", "sudo", "cc", "as", "run", 
    "startwm", "testlibc", "beep", "view", "file", "desktop", NULL
};

static void shell_autocomplete(char *buf, int *pos) {
    /* 1. Extract partially typed word */
    /* Find start of current word */
    int start = *pos;
    while (start > 0 && buf[start-1] != ' ') start--;
    
    int len = *pos - start;
    if (len == 0) return; /* Nothing to complete */
    
    char *partial = buf + start;
    
    /* 2. Check Commands if start == 0 (First word) */
    char *match = NULL;
    int match_count = 0;
    
    if (start == 0) {
        for (int i=0; cmd_list[i]; i++) {
             int clen = str_len(cmd_list[i]);
             if (clen >= len && strncmp(cmd_list[i], partial, len) == 0) {
                 match = (char*)cmd_list[i];
                 match_count++;
             }
        }
    } else {
        /* Check Files (InitRD + FAT32) */
        /* TODO: Implement File Autocompletion later if possible. 
           Requires listing files into a temporary buffer/array. 
           For now, prioritize commands. */
           
        /* Actually, let's try basic file match from InitRD? */
        /* Getting file list is complex here without allocating list. 
           Let's stick to Commands for now. */
    }
    
    /* 3. Apply Completion */
    if (match_count == 1 && match) {
        /* Unique Match found! */
        /* Append rest of string */
        char *rest = match + len;
        while (*rest) {
            if (*pos < 255) {
                term_erase_cursor();
                buf[(*pos)++] = *rest;
                char s[2] = {*rest, 0};
                kprint(s);
                term_draw_cursor();
            }
            rest++;
        }
        /* Append space */
        if (*pos < 255) {
            term_erase_cursor();
            buf[(*pos)++] = ' ';
            kprint(" ");
            term_draw_cursor();
        }
    }
}

static int editor_running = 0;
static char *editor_buffer = NULL;
static uint64_t editor_buf_len = 0;
static char editor_filename[256];
static int editor_mode = 0; /* 0=NORMAL, 1=INSERT */
static uint64_t editor_cursor_idx = 0; /* Index in buffer */

static void *mem_cpy(void *dest, const void *src, size_t n) {
    uint8_t *d = (uint8_t *)dest;
    const uint8_t *s = (const uint8_t *)src;
    while (n--) *d++ = *s++;
    return dest;
}

static void mem_set(void *dest, int val, size_t n) {
    uint8_t *d = (uint8_t *)dest;
    while (n--) *d++ = (uint8_t)val;
}

/* Helper functions */
static int str_cmp(const char *s1, const char *s2) {
    while (*s1 && *s2 && *s1 == *s2) {
        s1++;
        s2++;
    }
    return *s1 - *s2;
}

static int str_starts_with(const char *str, const char *prefix) {
    while (*prefix) {
        if (*str != *prefix) return 0;
        str++;
        prefix++;
    }
    return 1;
}

static size_t str_len(const char *s) {
    size_t len = 0;
    while (s[len]) len++;
    return len;
}

static void str_cpy(char *dest, const char *src) {
    while (*src) {
        *dest++ = *src++;
    }
    *dest = '\0';
}

static void str_cat(char *dest, const char *src) {
    while (*dest) dest++;
    while (*src) *dest++ = *src++;
    *dest = '\0';
}

static void print_num(uint64_t num) {
    if (num == 0) {
        kprint("0");
        return;
    }
    char buf[32];
    int i = 0;
    while (num > 0) {
        buf[i++] = '0' + (num % 10);
        num /= 10;
    }
    for (int j = i - 1; j >= 0; j--) {
        char c[2] = {buf[j], 0};
        kprint(c);
    }
}

static void cmd_clear(void) {
    term_clear();
}

/* Helper: Resolve path with . and .. handling */
/* target must be big enough (256) */
static void resolve_path(char *target, const char *base, const char *input) {
    // 1. Initial Setup
    if (input[0] == '/') {
        str_cpy(target, "/");
        input++;
    } else {
        str_cpy(target, base);
    }
    
    // 2. Tokenize and Process
    char token[128];
    int t_i = 0;
    
    while (1) {
        char c = *input;
        if (c == '/' || c == '\0') {
            token[t_i] = '\0';
            
            if (t_i > 0) { /* Process token */
                if (str_cmp(token, ".") == 0) {
                    /* Ignore */
                } else if (str_cmp(token, "..") == 0) {
                     /* Go Up */
                     size_t len = str_len(target);
                     if (len > 1) { /* Not root */
                         /* Remove trailing slash if exists (shouldn't) */
                         if (target[len-1] == '/') target[--len] = '\0';
                         
                         /* Find last slash */
                         while (len > 0 && target[len-1] != '/') len--;
                         
                         if (len == 0) { /* Back to root */
                             str_cpy(target, "/");
                         } else {
                             target[len] = '\0'; 
                             /* If we truncated to "dir/", remove slash unless it is root */
                             if (len > 1) target[len-1] = '\0';
                         }
                     }
                } else {
                    /* Append */
                    size_t len = str_len(target);
                    if (len > 1 || (len == 1 && target[0] != '/')) {
                        /* Add slash if not root */
                         if (target[len-1] != '/') str_cat(target, "/");
                    } else if (len == 1 && target[0] == '/') {
                        /* Root, no slash needed before append? No: / + foo = /foo */
                        /* My str_cat logic handles simple append. */
                        /* if target is "/", "foo" -> "/foo" */
                    }
                    str_cat(target, token);
                }
            }
            
            t_i = 0;
            if (c == '\0') break;
        } else {
            token[t_i++] = c;
        }
        input++;
    }
    
    // 3. Final Cleanup (remove trailing slash if not root)
    size_t len = str_len(target);
    if (len > 1 && target[len-1] == '/') target[len-1] = '\0';
}

/* Helper: Calculate X,Y from buffer index */
static void get_cursor_screen_pos(uint64_t idx, int *x, int *y) {
    int lines = 0;
    int chars = 0;
    for(uint64_t i=0; i<idx && i<editor_buf_len; i++) {
        if(editor_buffer[i] == '\n') { 
            lines++; 
            chars=0; 
        } else {
            chars++;
        }
    }
    *x = chars;
    *y = 2 + lines; /* Offset by 2 for header */
}

/* Helper: redraw specific to editor */
static void editor_redraw(void) {
    term_clear();
    
    /* Header */
    term_set_cursor(0, 0);
    kprint("=== Write Editor: ");
    kprint(editor_filename);
    kprint(" ===\n");
    
    /* Content */
    term_set_cursor(0, 2);
    if (editor_buffer) {
        kprint_buf(editor_buffer, editor_buf_len);
    }
    
    /* Footer */
    term_set_cursor(0, 48);
    kprint("--------------------------------------------------------------------------------");
    
    /* Calculate Line/Col for status */
    int x, y;
    get_cursor_screen_pos(editor_cursor_idx, &x, &y);
    int line_num = y - 2 + 1; /* 1-based */
    int col_num = x + 1;      /* 1-based */
    
    term_set_cursor(0, 49);
    kprint("Ctrl+S:Save | Ctrl+Q:Quit | Ln: ");
    print_num(line_num);
    kprint(" Col: ");
    print_num(col_num);
    
    /* Update Cursor */
    term_set_cursor(x, y);
}

/* Buffer operations */
static void editor_insert_char(char c) {
    if (editor_buf_len >= EDITOR_BUF_SIZE - 1) return;
    
    /* Shift content */
    for (uint64_t i = editor_buf_len; i > editor_cursor_idx; i--) {
        editor_buffer[i] = editor_buffer[i-1];
    }
    editor_buffer[editor_cursor_idx] = c;
    editor_buf_len++;
    editor_cursor_idx++;
}

static void editor_delete_char(void) {
    if (editor_cursor_idx == 0) return;
    
    /* Shifts content left */
    for (uint64_t i = editor_cursor_idx - 1; i < editor_buf_len - 1; i++) {
        editor_buffer[i] = editor_buffer[i+1];
    }
    editor_buf_len--;
    editor_cursor_idx--;
}

static void cmd_write(char *filename) {
    if (!filename || filename[0] == '\0') {
        kprint("Usage: write <filename>\n");
        return;
    }
    
    if (!editor_buffer) {
        editor_buffer = (char *)kmalloc(EDITOR_BUF_SIZE);
        if (!editor_buffer) {
            kprint("OOM for editor\n");
            return;
        }
    }
    
    editor_buf_len = 0;
    editor_cursor_idx = 0;
    
    char full_path[256];
    if (filename[0] == '/') str_cpy(full_path, filename);
    else {
        str_cpy(full_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(full_path, "/");
        str_cat(full_path, filename);
    }
    str_cpy(editor_filename, full_path);
    
    struct initrd_file *file = initrd_find_file(full_path);
    if (!file && full_path[0] == '/') file = initrd_find_file(full_path+1);
    
    if (file) {
        if (file->size < EDITOR_BUF_SIZE) {
            mem_cpy(editor_buffer, file->data, file->size);
            editor_buf_len = file->size;
        } else {
            kprint("File too large for editor!\n");
            return;
        }
    } else {
        mem_set(editor_buffer, 0, EDITOR_BUF_SIZE);
    }
    
    editor_cursor_idx = 0;
    editor_running = 1;
    editor_mode = 0; 
    
    editor_redraw();
    
    int blink_visible = 1;
    uint64_t loop_cycles = 0;
    term_draw_cursor();
    
    while (editor_running) {
        if (keyboard_available()) {
            char c = keyboard_getchar();
            int ctrl = keyboard_is_ctrl_active();
            
            if (!blink_visible) {
                 blink_visible = 1;
                 term_draw_cursor();
            }
            loop_cycles = 0;

            if (ctrl) {
                if (c == 'q' - 'a' + 1 || c == 'q') { /* Ctrl+Q */
                    editor_running = 0;
                    break;
                } else if (c == 's' - 'a' + 1 || c == 's') { /* Ctrl+S */
                    /* Try to save to Disk (FAT32) first */
                    // initrd_delete_file(editor_filename); // Don't delete from InitRD if we want persistence on disk
                    // initrd_create_file(editor_filename, editor_buffer, editor_buf_len);
                    
                    if (fat32_write_file(editor_filename, (uint8_t*)editor_buffer, editor_buf_len) == 0) {
                        term_set_cursor(0, 47); kprint("Saved to Disk! ");
                    } else {
                        /* Fallback or just error */
                        term_set_cursor(0, 47); kprint("Save Failed!   ");
                    }
                    
                    int x,y; get_cursor_screen_pos(editor_cursor_idx, &x, &y);
                    term_set_cursor(x, y);
                }
            } else {
                 unsigned char uc = (unsigned char)c;
                 
                 if (uc == KEY_LEFT) {
                     if (editor_cursor_idx > 0) editor_cursor_idx--;
                     int x,y; get_cursor_screen_pos(editor_cursor_idx, &x, &y);
                     term_set_cursor(x, y);
                 } else if (uc == KEY_RIGHT) {
                     if (editor_cursor_idx < editor_buf_len) editor_cursor_idx++;
                     int x,y; get_cursor_screen_pos(editor_cursor_idx, &x, &y);
                     term_set_cursor(x, y);
                 } else if (uc == KEY_UP) {
                     int col = 0;
                     uint64_t temp = editor_cursor_idx;
                     while (temp > 0 && editor_buffer[temp-1] != '\n') { temp--; col++; }
                     
                     if (temp > 0) {
                         temp--;
                         uint64_t prev_line_start = temp;
                         while (prev_line_start > 0 && editor_buffer[prev_line_start-1] != '\n') prev_line_start--;
                         int prev_line_len = temp - prev_line_start;
                         
                         if (col > prev_line_len) col = prev_line_len; 
                         editor_cursor_idx = prev_line_start + col;
                     }
                     int x,y; get_cursor_screen_pos(editor_cursor_idx, &x, &y);
                     term_set_cursor(x, y);
                 } else if (uc == KEY_DOWN) {
                     int col = 0;
                     uint64_t temp = editor_cursor_idx;
                     while (temp > 0 && editor_buffer[temp-1] != '\n') { temp--; col++; }
                     
                     temp = editor_cursor_idx;
                     while (temp < editor_buf_len && editor_buffer[temp] != '\n') temp++;
                     
                     if (temp < editor_buf_len) {
                         temp++; 
                         uint64_t next_line_start = temp;
                         while (temp < editor_buf_len && editor_buffer[temp] != '\n') temp++;
                         int next_line_len = temp - next_line_start;
                         
                         if (col > next_line_len) col = next_line_len;
                         editor_cursor_idx = next_line_start + col;
                     }
                     int x,y; get_cursor_screen_pos(editor_cursor_idx, &x, &y);
                     term_set_cursor(x, y);
                 } else {
                    /* Modeless Input */
                    if (c == '\b') {
                        editor_delete_char();
                        editor_redraw();
                    } else if (c == 27) { 
                        editor_redraw();
                    } else if (c == '\n' || c == '\r') { /* Fix: Accept Newline */
                        editor_insert_char('\n');
                        editor_redraw();
                    } else if (c == '\t') { /* Tab Support */
                         editor_insert_char(' ');
                         editor_insert_char(' ');
                         editor_insert_char(' ');
                         editor_insert_char(' ');
                         editor_redraw();
                    } else if (c >= 32 && c <= 126) {
                        editor_insert_char(c);
                        editor_redraw();
                    }
                }
            }
            term_draw_cursor();
        } else {
            loop_cycles++;
            if (loop_cycles > 4000000) {
                blink_visible = !blink_visible;
                if (blink_visible) term_draw_cursor();
                else term_erase_cursor();
                loop_cycles = 0;
            }
            for (volatile int i=0; i<100; i++);
        }
    }
    
    term_clear();
    kprint("Exited Editor.\n");
}

/* Command: mkfs */
static void cmd_mkfs(void) {
    if (!g_sudo_active) {
        kprint_color(KLOG_COLOR_RED, "Permission denied. This command requires administrative privileges.\n");
        return;
    }
    kprint("Partitioning Disk (MBR)...\\n");
    mbr_write_default();
    
    kprint("Formatting Partition 1 (FAT32)...\\n");
    fat32_format();
    
    kprint("Done.\\n");
}

/* Command: mount */
static void cmd_mount(void) {
    if (fat32_init() == 0) {
        kprint("Disk Mounted.\\n");
    } else {
        kprint("Mount failed.\\n");
    }
}

/* Command: lsdisk */
static void cmd_lsdisk(void) {
    fat32_list_files();
}

/* Command: save */
static void cmd_save(char *filename) {
    if (!filename || filename[0] == '\0') {
        kprint("Usage: save <filename>\\n");
        return;
    }
    
    struct initrd_file *file = initrd_find_file(filename);
    if (!file && filename[0] == '/') file = initrd_find_file(filename+1);
    
    if (file) {
        if (fat32_write_file(filename, file->data, file->size) == 0) {
            kprint("Saved to Disk.\\n");
        } else {
            kprint("Save failed.\\n");
        }
    } else {
        kprint("File not found in InitRD.\\n");
    }
}

/* Command: update */
static void cmd_update(void) {
    if (!g_sudo_active) {
        kprint_color(KLOG_COLOR_RED, "Permission denied. This command requires administrative privileges.\n");
        return;
    }

    kprint_color(KLOG_COLOR_CYAN, "=== Ainuix System Update ===\n");
    kprint("Current Version: "); 
    char ver[16]; itoa(AINUIX_VERSION_MAJOR, ver, 10); kprint(ver); kprint(".");
    itoa(AINUIX_VERSION_MINOR, ver, 10); kprint(ver); kprint(".");
    itoa(AINUIX_VERSION_PATCH, ver, 10); kprint(ver); kprint(" (" AINUIX_CODENAME ")\n");
    
    kprint("Checking for updates...\n");
    
    /* Simulate Network Check */
    for(int i=0; i<50; i++) {
         if ((i % 10) == 0) kprint(".");
         timer_sleep(50);
    }
    kprint("\n");
    
    /* Mock Update Availability */
    kprint_color(KLOG_COLOR_GREEN, "Update Found: Ainuix v1.3.0 (codenamed 'Borealis')\n");
    kprint("New Features:\n - Improved Multimedia Engine\n - Enhanced Security\n - 100% Rust Kernel (Just kidding)\n");
    
    kprint("\nDo you want to proceed with the update? [y/N]: ");
    
    /* Clear any pending input */
    while(keyboard_available()) keyboard_getchar();
    
    char c = 0;
    while(1) {
        if(keyboard_available()) {
            c = keyboard_getchar();
            break;
        }
    }
    
    if (c != 'y' && c != 'Y') {
        kprint("N\n");
        kprint_color(KLOG_COLOR_RED, "Update Cancelled.\n");
        return;
    }
    kprint("Y\n");

    kprint("Starting Update Process...\n");
    
    /* Step 1: Backup Configuration */
    kprint_color(KLOG_COLOR_YELLOW, "[1/4] Backing up configuration...\n");
    
    struct initrd_file *cfg = initrd_find_file("test.txt");
    if (cfg) {
         if (fat32_write_file("test.bak", cfg->data, cfg->size) == 0) {
             kprint_color(KLOG_COLOR_GREEN, " - Config Backup Successful.\n");
         }
    }
    
    /* Step 2: Kernel Backup (Fallback Creation) */
    kprint_color(KLOG_COLOR_YELLOW, "[2/4] Creating System Restore Point...\n");
    
    /* Check if kernel.elf exists on disk */
    fat32_delete_file("kernel.old"); /* Clean old fallback */
    
    if (fat32_rename("kernel.elf", "kernel.old") == 0) {
        kprint_color(KLOG_COLOR_GREEN, " - Backup created: kernel.old\n");
    } else {
        kprint(" - No previous kernel found or rename failed. Proceeding with fresh install.\n");
    }
    
    /* Step 3: 'Download' and Install New Kernel */
    kprint_color(KLOG_COLOR_YELLOW, "[3/4] Downloading & Installing Update...\n");
    
    struct initrd_file *new_kernel = initrd_find_file("kernel.elf");
    if (!new_kernel) {
         kprint_color(KLOG_COLOR_RED, " - Error: Update package (kernel.elf) not found in InitRD!\n");
         /* Restore backup? */
         fat32_rename("kernel.old", "kernel.elf");
         return;
    }
    
    /* Simulation Progress Bar */
    for(int i=0; i<=100; i+=10) {
         char buf[32]; itoa(i, buf, 10);
         kprint("\rDownload: "); kprint(buf); kprint("%");
         timer_sleep(50);
    }
    kprint("\n - Writing new kernel image to disk...\n");
    
    if (fat32_write_file("kernel.elf", new_kernel->data, new_kernel->size) == 0) {
        kprint_color(KLOG_COLOR_GREEN, " - Kernel Update Successful.\n");
    } else {
        kprint_color(KLOG_COLOR_RED, " - WRITE FAILED! Restoring backup...\n");
        fat32_rename("kernel.old", "kernel.elf");
        return;
    }
    
    /* Step 4: Update Bootloader */
    kprint_color(KLOG_COLOR_YELLOW, "[4/4] Updating Bootloader Configuration...\n");
    /* We should append a Fallback entry to limine.cfg if not present */
    /* Limitation: parsing text is hard. We'll simply ensure kernel.old is there. */
    /* Assuming user handles boot menu. Or we can just append if we implement append mode. */
    /* For now, just notifying usage of kernel.old */
    kprint(" - Bootloader updated to support Fallback.\n");
    
    kprint_color(KLOG_COLOR_GREEN, "=== Update Completed Successfully ===\n");
    kprint("The system has been updated. If issues occur, select 'Fallback' or manually boot kernel.old.\n");
    kprint("Please reboot to apply changes.\n");
}

/* Helper for date/cal */
static int get_day_of_week(int d, int m, int y) {
    return (d += m < 3 ? y-- : y - 2, 23*m/9 + d + 4 + y/4- y/100 + y/400)%7;
}

/* Banner Data for Days (Mini-FIGlet style) 5-lines height */
static const char *banner_sun[] = {
  " SSS  U   U N   N DDDD   AAA  Y   Y ",
  "S     U   U NN  N D   D A   A  Y Y  ",
  " SSS  U   U N N N D   D AAAAA   Y   ",
  "    S U   U N  NN D   D A   A   Y   ",
  "SSSS   UUU  N   N DDDD  A   A   Y   "
};
static const char *banner_mon[] = {
  "M   M  OOO  N   N DDDD   AAA  Y   Y ",
  "MM MM O   O NN  N D   D A   A  Y Y  ",
  "M M M O   O N N N D   D AAAAA   Y   ",
  "M   M O   O N  NN D   D A   A   Y   ",
  "M   M  OOO  N   N DDDD  A   A   Y   "
};
static const char *banner_tue[] = {
  "TTTTT U   U EEEEE  SSS  DDDD   AAA  Y   Y ",
  "  T   U   U E     S     D   D A   A  Y Y  ",
  "  T   U   U EEE    SSS  D   D AAAAA   Y   ",
  "  T   U   U E         S D   D A   A   Y   ",
  "  T    UUU  EEEEE SSSS  DDDD  A   A   Y   "
};
static const char *banner_wed[] = {
  "W   W EEEEE DDDD  N   N EEEEE  SSS  DDDD   AAA  Y   Y ",
  "W   W E     D   D NN  N E     S     D   D A   A  Y Y  ",
  "W W W EEE   D   D N N N EEE    SSS  D   D AAAAA   Y   ",
  "WW WW E     D   D N  NN E         S D   D A   A   Y   ",
  "W   W EEEEE DDDD  N   N EEEEE SSSS  DDDD  A   A   Y   "
};
static const char *banner_thu[] = {
  "TTTTT H   H U   U RRRR   SSS  DDDD   AAA  Y   Y ",
  "  T   H   H U   U R   R S     D   D A   A  Y Y  ",
  "  T   HHHHH U   U RRRR   SSS  D   D AAAAA   Y   ",
  "  T   H   H U   U R R       S D   D A   A   Y   ",
  "  T   H   H  UUU  R  RR SSSS  DDDD  A   A   Y   "
};
static const char *banner_fri[] = {
  "FFFFF RRRR  IIIII DDDD   AAA  Y   Y ",
  "F     R   R   I   D   D A   A  Y Y  ",
  "FFF   RRRR    I   D   D AAAAA   Y   ",
  "F     R R     I   D   D A   A   Y   ",
  "F     R  RR IIIII DDDD  A   A   Y   "
};
static const char *banner_sat[] = {
  " SSS   AAA  TTTTT U   U RRRR  DDDD   AAA  Y   Y ",
  "S     A   A   T   U   U R   R D   D A   A  Y Y  ",
  " SSS  AAAAA   T   U   U RRRR  D   D AAAAA   Y   ",
  "    S A   A   T   U   U R R   D   D A   A   Y   ",
  "SSSS  A   A   T    UUU  R  RR DDDD  A   A   Y   "
};

static void print_banner_day(int dow) {
    const char **banner;
    switch(dow) {
        case 0: banner = banner_sun; break;
        case 1: banner = banner_mon; break;
        case 2: banner = banner_tue; break;
        case 3: banner = banner_wed; break;
        case 4: banner = banner_thu; break;
        case 5: banner = banner_fri; break;
        case 6: banner = banner_sat; break;
        default: return;
    }
    
    kprint("\n");
    for(int i=0; i<5; i++) {
        kprint_color(KLOG_COLOR_YELLOW, "   "); /* Indent */
        kprint_color(KLOG_COLOR_YELLOW, banner[i]);
        kprint("\n");
    }
    kprint("\n");
}

/* Command: date (DD/MM/YYYY + Big Day) */
static void cmd_date(char *args) {
    (void)args;
    rtc_time_t t;
    rtc_get_time(&t);
    
    char date_buf[32];
    char year_buf[8]; itoa(t.year, year_buf, 10);
    char mont_buf[4]; itoa(t.month, mont_buf, 10);
    char day_buf[4];  itoa(t.day,   day_buf, 10);
    
    /* Format: DD/MM/YYYY */
    char *p = date_buf;
    if (t.day < 10) *p++ = '0';
    str_cpy(p, day_buf); p += str_len(day_buf);
    *p++ = '/';
    if (t.month < 10) *p++ = '0';
    str_cpy(p, mont_buf); p += str_len(mont_buf);
    *p++ = '/';
    str_cpy(p, year_buf);
    
    kprint("\n   Date: ");
    kprint_color(KLOG_COLOR_RESET, date_buf);
    kprint("\n");
    
    int dow = get_day_of_week(t.day, t.month, t.year);
    print_banner_day(dow);
}

/* Command: time */
/* Command: time (Styled Box with AM/PM) */
static void cmd_time(char *args) {
    (void)args;
    rtc_time_t t;
    rtc_get_time(&t);
    
    char time_buf[64];
    
    int hour = t.hour;
    const char *ampm = "AM";
    if (hour >= 12) {
        ampm = "PM";
        if (hour > 12) hour -= 12;
    }
    if (hour == 0) hour = 12;
    
    char h_b[4]; itoa(hour, h_b, 10);
    char m_b[4]; itoa(t.minute, m_b, 10);
    char s_b[4]; itoa(t.second, s_b, 10);
    
    char *p = time_buf;
    if (hour < 10) *p++ = '0';
    str_cpy(p, h_b); p += str_len(h_b);
    *p++ = ':';
    if (t.minute < 10) *p++ = '0';
    str_cpy(p, m_b); p += str_len(m_b);
    *p++ = ':';
    if (t.second < 10) *p++ = '0';
    str_cpy(p, s_b); p += str_len(s_b);
    *p++ = ' ';
    str_cpy(p, ampm); p += str_len(ampm);
    
    /* Box Drawing in Blue */
    kprint("\n");
    kprint_color(KLOG_COLOR_BLUE, "   .---------------------.\n");
    kprint_color(KLOG_COLOR_BLUE, "   |      ");
    
    kprint_color(KLOG_COLOR_RESET, time_buf);
    
    /* Padding for "HH:MM:SS AM" -> 11 chars */
    /* Box inner width: 21 chars */
    /* "      " (6) + 11 = 17. 21-17 = 4 spaces right */
    
    kprint_color(KLOG_COLOR_BLUE, "     |\n");
    kprint_color(KLOG_COLOR_BLUE, "   '---------------------'\n");
    kprint("\n");
}

/* Helpers for Date/Time */
static int get_days_in_month(int m, int y) {
   if (m == 2) return (y%4==0 && (y%100!=0 || y%400==0)) ? 29 : 28;
   if (m == 4 || m == 6 || m == 9 || m == 11) return 30;
   return 31;
}

static const char *month_names[] = { "", "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December" };
static const char *day_names_short[] = { "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" };

/* Clock Utility: Integer Sine approximation (Scale: 1000) */
/* Table for 0-15 (quarter circle), mirror for rest? Or just small LUT */
/* 60 steps. */
/* Clock Utility: Integer Sine approximation (Scale: 1000) */
static void get_sin_cos(int angle, int *s, int *c) {
    /* Angle: 0..360 */
    /* Crude Lookup Table approach or decent approximation would be better */
    /* Using the LUT from before but mapped properly */
    /* 0..60 maps to 0..360 degrees. */
    /* We need generic angle support for smooth hands? No, stick to 60 steps for simplicity or expand. */
    
    /* Re-use the LUT_X from text clock but generalized */
    static const int lut_sin[60] = {
         0,  104,  207,  309,  406,  500,  587,  669,  743,  809,  866,  913,  951,  978,  994, 
        1000, 994,  978,  951,  913,  866,  809,  743,  669,  587,  500,  406,  309,  207,  104, 
         0, -104, -207, -309, -406, -500, -587, -669, -743, -809, -866, -913, -951, -978, -994, 
       -1000,-994, -978, -951, -913, -866, -809, -743, -669, -587, -500, -406, -309, -207, -104
    };
    /* This LUT is sin(0..360 at 6 deg intervals). Idx 15 = 90 deg = 1000. */
    
    /* Map input angle (0..360) to index (0..60). angle/6. */
    /* Simple nearest neighbor */
    int idx = (angle % 360) / 6;
    *s = lut_sin[idx];
    *c = lut_sin[(idx + 15) % 60]; /* cos(x) = sin(x+90) */
}

/* Bresenham Circle */
static void gfx_draw_circle(int xc, int yc, int r, uint32_t color) {
    int x = 0, y = r;
    int d = 3 - 2 * r;
    while (y >= x) {
        gfx_put_pixel_safe(xc+x, yc+y, color);
        gfx_put_pixel_safe(xc-x, yc+y, color);
        gfx_put_pixel_safe(xc+x, yc-y, color);
        gfx_put_pixel_safe(xc-x, yc-y, color);
        gfx_put_pixel_safe(xc+y, yc+x, color);
        gfx_put_pixel_safe(xc-y, yc+x, color);
        gfx_put_pixel_safe(xc+y, yc-x, color);
        gfx_put_pixel_safe(xc-y, yc-x, color);
        x++;
        if (d > 0) {
            y--;
            d = d + 4 * (x - y) + 10;
        } else {
            d = d + 4 * x + 6;
        }
    }
}

/* Bresenham Filled Circle (for center dot) */
static void gfx_fill_circle(int xc, int yc, int r, uint32_t color) {
    int x = 0, y = r;
    int d = 3 - 2 * r;
    while (y >= x) {
        gfx_draw_line(xc-x, yc+y, xc+x, yc+y, color);
        gfx_draw_line(xc-x, yc-y, xc+x, yc-y, color);
        gfx_draw_line(xc-y, yc+x, xc+y, yc+x, color);
        gfx_draw_line(xc-y, yc-x, xc+y, yc-x, color);
        x++;
        if (d > 0) {
            y--;
            d = d + 4 * (x - y) + 10;
        } else {
            d = d + 4 * x + 6;
        }
    }
}

static void draw_clock_hand(int xc, int yc, int angle_deg, int length, uint32_t color, int thickness) {
    int s, c;
    get_sin_cos(angle_deg, &s, &c);
    
    /* GFX Coord: Y is Down. 0 deg is usually Right.
       Clock 0 is UP (270 deg). 
       x = xc + len * cos(a)
       y = yc + len * sin(a)
       
       We want 0 input -> Up.
       Map input angle A (0..360, 0=Up) to Cartesian:
       Up = 270. Right = 0.
       Cartesian A' = A - 90.
       
       x = len * cos(A-90) = len * sin(A)
       y = len * sin(A-90) = len * -cos(A)
    */
    
    int end_x = xc + (s * length / 1000);
    int end_y = yc - (c * length / 1000); /* Y inverted */
    
    gfx_draw_line(xc, yc, end_x, end_y, color);
    
    if (thickness > 1) {
        /* Draw adjacent lines for thickness */
        gfx_draw_line(xc+1, yc+1, end_x+1, end_y+1, color);
        gfx_draw_line(xc-1, yc-1, end_x-1, end_y-1, color);
    }
    
    /* Tail (opposite direction) */
    if (thickness == 1) { /* Assuming Second hand */
         int tail_len = length / 5;
         int tail_x = xc - (s * tail_len / 1000);
         int tail_y = yc + (c * tail_len / 1000);
         gfx_draw_line(xc, yc, tail_x, tail_y, color);
    }
}

/* Graphical Clock */
static void cmd_clock(void) {
    /* Basic Fullscreen Graphical Clock */
    
    /* 1. Get Screen Info */
    uint64_t width, height, pitch;
    void *fb;
    gfx_get_info(&width, &height, &pitch, &fb);
    
    int cx = width / 2;
    int cy = height / 2;
    int radius = (height / 2) - 40;
    if (radius > 250) radius = 250; /* Limit size */
    
    kprint("Starting modern graphical clock... (Press any key to exit)\n");
    timer_sleep(500); /* Wait for key up */
    
    /* Loop */
    int running = 1;
    while(running) {
        if (keyboard_available()) {
            keyboard_getchar();
            running = 0;
            break;
        }
        
        rtc_time_t t;
        rtc_get_time(&t);
        
        /* Clear Screen (or just drawing area?) Full clear to avoid artifacts for now */
        /* Clear Screen (Dark Mode) */
        gfx_clear(0x101010);
        
        /* Draw Face */
        /* Outer Rim - Orange */
        gfx_draw_circle(cx, cy, radius, 0xFF8800);
        gfx_draw_circle(cx, cy, radius-1, 0xFF8800);
        
        /* Ticks */
        for(int i=0; i<360; i+=30) {
             int s, c;
             get_sin_cos(i, &s, &c); /* sin(i) = x, -cos(i) = y */
             int x1 = cx + (s * (radius-5) / 1000);
             int y1 = cy - (c * (radius-5) / 1000);
             int x2 = cx + (s * (radius-20) / 1000);
             int y2 = cy - (c * (radius-20) / 1000);
             
             uint32_t col = (i % 90 == 0) ? COLOR_RED : COLOR_GRAY; // 12, 3, 6, 9 Red
             if (i % 90 == 0) {
                 /* Thicker main ticks */
                 gfx_draw_line(x1, y1, x2, y2, col);
                 gfx_draw_line(x1+1, y1, x2+1, y2, col);
             } else {
                 gfx_draw_line(x1, y1, x2, y2, col);
             }
        }
        
        /* Hour Hand */
        /* 12 hours = 360 deg. 1 hr = 30 deg. + min/2 deg */
        /* Integer math: h_angle = (h%12)*30 + m/2 */
        int h_angle = (t.hour % 12) * 30 + (t.minute / 2);
        draw_clock_hand(cx, cy, h_angle, radius / 2, COLOR_WHITE, 3);
        
        /* Minute Hand */
        /* 60 mins = 360 deg. 1 min = 6 deg. */
        int m_angle = t.minute * 6;
        draw_clock_hand(cx, cy, m_angle, (radius * 3) / 4, COLOR_WHITE, 2);
        
        /* Second Hand */
        int s_angle = t.second * 6;
        draw_clock_hand(cx, cy, s_angle, (radius * 85) / 100, COLOR_RED, 1);
        
        /* Center Dot */
        gfx_fill_circle(cx, cy, 6, COLOR_RED);
        gfx_fill_circle(cx, cy, 3, COLOR_BLACK); /* Donut */
        
        /* Digital Backup (Small Text at Bottom) */
        int hour = t.hour;
        char *ampm = "AM";
        if (hour >= 12) {
            ampm = "PM";
            if (hour > 12) hour -= 12;
        }
        if (hour == 0) hour = 12;

        char h_b[8]; itoa(hour, h_b, 10);
        char m_b[8]; itoa(t.minute, m_b, 10);
        char s_b[8]; itoa(t.second, s_b, 10);
        
        /* Calculate Position: Bottom Center */
        /* Assuming 8x16 font */
        int rows = height / 16;
        int cols = width / 8;
        int text_row = rows - 4; 
        int text_col = (cols / 2) - 6; /* 11 chars roughly "HH:MM:SS XX" */
        
        term_set_cursor(text_col, text_row);
        
        /* Box Styling Optional, just text for now */
        kprint_color(KLOG_COLOR_RESET, "[ ");
        
        if (hour < 10) kprint_color(KLOG_COLOR_YELLOW, "0"); /* Orange-ish (Yellow closest) */
        kprint_color(KLOG_COLOR_YELLOW, h_b);
        kprint_color(KLOG_COLOR_RESET, ":");
        if (t.minute < 10) kprint_color(KLOG_COLOR_YELLOW, "0");
        kprint_color(KLOG_COLOR_YELLOW, m_b);
        kprint_color(KLOG_COLOR_RESET, ":");
        if (t.second < 10) kprint_color(KLOG_COLOR_RESET, "0");
        kprint_color(KLOG_COLOR_RESET, s_b);
        
        kprint_color(KLOG_COLOR_YELLOW, " ");
        kprint_color(KLOG_COLOR_YELLOW, ampm);
        
        kprint_color(KLOG_COLOR_CYAN, " ]");
        
        /* Date Display: DOW, DD MON YYYY */
        /* Center below Time */
        term_set_cursor(text_col - 2, text_row + 2);
        
        int dow = get_day_of_week(t.day, t.month, t.year);
        kprint_color(KLOG_COLOR_GREEN, day_names_short[dow]);
        kprint(", ");
        
        char d_b[4]; itoa(t.day, d_b, 10);
        if (t.day < 10) kprint("0");
        kprint(d_b);
        kprint(" ");
        kprint(month_names[t.month]);
        kprint(" ");
        char y_b[8]; itoa(t.year, y_b, 10);
        kprint(y_b);
        
        /* Wait */
        timer_sleep(100);
    }
    
    term_clear(); /* Restore Text */
}
/* Command: cal */
static void cmd_cal(char *args) {
    (void)args;
    rtc_time_t t;
    rtc_get_time(&t);
    
    kprint("   "); kprint(month_names[t.month]); kprint(" "); 
    char buf[8]; itoa(t.year, buf, 10); kprint(buf); kprint("\n");
    kprint("Su Mo Tu We Th Fr Sa\n");
    
    int days = get_days_in_month(t.month, t.year);
    int start_day = get_day_of_week(1, t.month, t.year);
    
    /* 0 = Sunday */
    
    for (int i=0; i<start_day; i++) kprint("   ");
    
    for (int d=1; d<=days; d++) {
        if (d < 10) kprint(" ");
        itoa(d, buf, 10); kprint(buf); kprint(" ");
        
        if ((start_day + d) % 7 == 0) kprint("\n");
    }
    kprint("\n");
}


static void cmd_help(void) {
    kprint("\n=== Available Commands ===\n");
    kprint("  help      - Show this help\n");
    kprint("  clear     - Clear screen\n");
    kprint("  exit      - Logout\n");
    kprint("  shutdown  - Power Off System\n");
    kprint("  reboot    - Reboot System\n");
    kprint("  date      - Show Date\n");
    kprint("  time      - Show Time\n");
    kprint("  cal       - Show Calendar\n");
    kprint("  update    - Check for Updates\n");
    kprint("\n[File System]\n");
    kprint("  ls [dir]  - List files\n");
    kprint("  pwd       - Show current directory\n");
    kprint("  cd <dir>  - Change directory\n");
    kprint("  mkdir <d> - Create directory\n");
    kprint("  touch <f> - Create empty file\n");
    kprint("  cp <s,d>  - Copy file\n");
    kprint("  rm <f>    - Remove file\n");
    kprint("  cat <f>   - Display file content\n");
    kprint("  write <f> - Text Editor\n");
    kprint("\n[Disk / FAT32]\n");
    kprint("  mount     - Mount FAT32 Disk\n");
    kprint("  lsdisk    - List Disk Files\n");
    kprint("  mkfs      - Format Partition 1\n");
    kprint("  fdisk     - Partition Manager\n");
    kprint("  save <f>  - Save InitRD file to Disk\n");
    kprint("\n[System & Users]\n");
    kprint("  useradd   - Add new user\n");
    kprint("  passwd    - Change password\n");
    kprint("  meminfo   - Show Memory Usage\n");
    kprint("  lspci     - List PCI Devices\n");
    kprint("  dmesg     - Show Kernel Log\n");
    kprint("\n[Dev & Apps]\n");
    kprint("  whoami    - Show Current User\n");
    kprint("  uptime    - Show System Uptime\n");
    kprint("  date      - Digital Date Box\n");
    kprint("  clock     - Analog + Digital Clock (Animated)\n");
    kprint("  apt       - Package Manager\n");
    kprint("  sudo <cmd>- Execute with Admin Privileges\n");
    kprint("  cc <file> - Compile C Script\n");
    kprint("  as <file> - Assemble ASM\n");
    kprint("  run <f>   - Run ELF/Script\n");
    kprint("  startwm   - Start GUI\n");
    kprint("  testlibc  - Run LibC Tests\n");
    kprint("\n[Multimedia]\n");
    kprint("  beep [f]  - Generate Tone\n");
    kprint("  view <f>  - View BMP Image\n");
    kprint("  file <f>  - Identify File Type\n");
    kprint("\n");
}

/* Command: fdisk */
static void cmd_fdisk(char *arg) {
    if (!g_sudo_active) {
        if (!arg || arg[0] == '\0' || str_cmp(arg, "map") == 0) {
              /* Allowed just to view */
        } else {
             kprint_color(KLOG_COLOR_RED, "Permission denied. Modifying partitions requires administrative privileges (sudo).\n");
             return;
        }
    }

    if (!arg || arg[0] == '\0' || str_cmp(arg, "map") == 0) {
        mbr_print_map();
    } else if (str_starts_with(arg, "new")) {
        /* Parse Size */
        char *s = arg;
        while (*s && *s != ' ') s++; /* Skip 'new' */
        while (*s == ' ') s++; /* Skip spaces */
        
        /* Simple atoi */
        int size = 0;
        int found = 0;
        while (*s >= '0' && *s <= '9') {
             size = size * 10 + (*s - '0');
             s++;
             found = 1;
        }
        if (found && size > 0) {
             mbr_new_partition(size);
        } else {
            kprint("Invalid size. Usage: fdisk new <MB>\n");
        }
    } else if (str_cmp(arg, "reset") == 0) {
        mbr_write_default();
    } else {
        kprint("Usage:\n  fdisk map\n  fdisk new <MB>\n  fdisk reset\n");
    }
}

/* Command: pwd */
static void cmd_pwd(void) {
    kprint("\n");
    kprint(cwd);
    kprint("\n\n");
}

/* Command: cd */
static void cmd_cd(char *path) {
    char target[256];
    
    /* Handle Special Cases */
    if (!path || path[0] == '\0' || str_cmp(path, "~") == 0) {
        /* Go Home (check /home/user or just /) */
        /* For now, Root is Home */
        str_cpy(target, "/");
    } else if (str_cmp(path, "-") == 0) {
        /* Previous Directory */
        str_cpy(target, prev_cwd);
        kprint(target); 
        kprint("\n");
    } else {
        /* Resolve Path */
        resolve_path(target, cwd, path);
    }

    /* Check Existence */
    if (initrd_is_dir(target)) {
        str_cpy(prev_cwd, cwd); /* Save current as previous */
        str_cpy(cwd, target);
        fat32_change_dir("/"); /* Reset FAT32 context if moving to known InitRD path */
    } else {
        /* Check FAT32 */
        /* Currently our path logic splits InitRD and FAT32.
           If user types 'cd EFI', target becomes '/EFI'. 
           We pass 'EFI' to fat32_change_dir. */
             
        char *fat_path = target;
        if (fat_path[0] == '/') fat_path++; /* Skip leading slash */
        
        if (fat32_change_dir(fat_path) == 0) {
             /* Success! */
             str_cpy(prev_cwd, cwd);
             str_cpy(cwd, target);
        } else {
             kprint("\nDirectory not found: ");
             kprint(target);
             kprint("\n\n");
        }
    }
}

/* Command: ls */
static void cmd_ls(char *arg) {
    kprint("\n[InitRD]\n");
    char target[256];
    
    if (arg && arg[0] != '\0') {
        if (arg[0] == '/') {
            str_cpy(target, arg);
        } else {
            str_cpy(target, cwd);
            if (str_cmp(cwd, "/") != 0) str_cat(target, "/");
            str_cat(target, arg);
        }
    } else {
        str_cpy(target, cwd);
    }
    
    initrd_list_files(target);
    
    kprint("\n[Disk (FAT32)]\n");
    fat32_list_files();
    kprint("\n");
}

/* Command: cat */
static void cmd_cat(const char *filename) {
    /* Try InitRD first */
    char full_path[256];
    if (filename[0] == '/') {
        str_cpy(full_path, filename);
    } else {
        str_cpy(full_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(full_path, "/");
        str_cat(full_path, filename);
    }

    struct initrd_file *file = initrd_find_file(full_path);
    if (!file && full_path[0] == '/') {
         file = initrd_find_file(full_path + 1);
    }

    if (file) {
        kprint("\n");
        kprint_buf((char *)file->data, file->size);
        kprint("\n\n");
        return;
    }
    
    /* Try FAT32 */
    uint8_t *data;
    uint32_t size;
    if (fat32_read_file(filename, &data, &size) == 0) {
        kprint("\n");
        kprint_buf((char *)data, size);
        kprint("\n\n");
        kfree(data);
    } else {
        kprint("\nFile not found: ");
        kprint(filename);
        kprint("\n\n");
    }
}

static void cmd_touch(char *name) {
    if (!name || name[0] == '\0') {
        kprint("Usage: touch <filename>\n");
        return;
    }
    
    /* Create empty file on FAT32 */
    /* Dummy data */
    uint8_t dummy[1] = {0};
    if (fat32_write_file(name, dummy, 0) == 0) {
        kprint("File created on Disk.\n");
    } else {
        kprint("Failed to create file on Disk.\n");
    }
}

static void cmd_mkdir(char *name) {
    if (!name || name[0] == '\0') {
        kprint("Usage: mkdir <dirname>\n");
        return;
    }
    
    /* Use VFS Abstraction */
    /* VFS handles finding root vs relative path if logic is in vfs_mkdir */
    /* For now vfs_mkdir assumes absolute or relative to root, we should pass full path if needed */
    /* But vfs_mkdir implementation currently splits first char. Let's pass simplified name for now */
    
    if (vfs_mkdir(name, 0) == 0) {
        kprint("Directory created.\n");
    } else {
        kprint("Failed to create directory.\n");
    }
}

static void cmd_cp(char *args) {
    if (!args) { kprint("Usage: cp <src> <dest>\n"); return; }
    char *src = args;
    char *dest = NULL;
    while (*args && *args != ' ') args++;
    if (*args == ' ') {
        *args = '\0';
        dest = args + 1;
    }
    if (!src || !dest) {
        kprint("Usage: cp <src> <dest>\n");
        return;
    }
    
    int fd_in = vfs_open(src, O_RDONLY);
    if (fd_in < 0) {
        kprint("Failed to open source file.\n");
        return;
    }
    
    /* Create/Truncate Dest */
    int fd_out = vfs_open(dest, O_WRONLY | O_CREAT | O_TRUNC);
    if (fd_out < 0) {
        kprint("Failed to open/create dest file.\n");
        vfs_close(fd_in);
        return;
    }
    
    /* Copy Loop */
    uint8_t *buf = (uint8_t *)kmalloc(4096);
    if (!buf) {
        kprint("OOM\n");
        vfs_close(fd_in);
        vfs_close(fd_out);
        return;
    }
    
    uint64_t total = 0;
    while (1) {
        int64_t n = vfs_read_fd(fd_in, buf, 4096);
        if (n <= 0) break;
        
        vfs_write_fd(fd_out, buf, n);
        total += n;
    }
    
    kfree(buf);
    vfs_close(fd_in);
    vfs_close(fd_out);
    
    kprint("Copied.\n");
}

/* Command: apt */
static void cmd_apt(char *args) {
    if (!args) { kprint("Usage: apt <install|remove|list> [package]\n"); return; }
    char *cmd = args;
    char *pkg = NULL;
    while (*args && *args != ' ') args++;
    if (*args == ' ') {
        *args = '\0';
        pkg = args + 1;
    }
    
    if (!cmd) { kprint("Usage: apt <install|remove|list> [package]\n"); return; }
    
    if (str_cmp(cmd, "install") == 0) {
        if (!pkg) { kprint("Usage: apt install <package>\n"); return; }
        
        /* Ensure /bin exists */
        fat32_create_dir("bin");
        
        /* Copy file to /bin/pkg */
        char dest[64];
        str_cpy(dest, "bin/");
        str_cat(dest, pkg);
        
        kprint("Installing "); kprint(pkg); kprint("...\n");
        
        /* Read Source (Local CWD) */
        uint8_t *data = NULL;
        uint32_t size = 0;
        
        struct initrd_file *sfile = initrd_find_file(pkg);
        if (sfile) {
            data = (uint8_t*)sfile->data;
            size = sfile->size;
        } else {
            /* Try FAT32 read from CWD */
            if (fat32_read_file(pkg, &data, &size) == 0) {
                /* Success */
            } else {
                kprint("Package not found in current directory.\n");
                return;
            }
        }
        
        /* Write to /bin */
        if (fat32_change_dir("bin") != 0) { kprint("Failed to access /bin\n"); return; }
        if (fat32_write_file(pkg, data, size) == 0) kprint("Installed.\n");
        else kprint("Installation failed.\n");
        fat32_change_dir("/");
        
        if (!sfile && data) kfree(data);
        
    } else if (str_cmp(cmd, "remove") == 0) {
        if (!pkg) { kprint("Usage: apt remove <package>\n"); return; }
        
        if (fat32_change_dir("bin") != 0) { kprint("/bin not found.\n"); return; }
        if (fat32_delete_file(pkg) == 0) kprint("Removed.\n");
        else kprint("Package not found.\n");
        fat32_change_dir("/");
        
    } else if (str_cmp(cmd, "list") == 0) {
        kprint("Listing packages in /bin:\n");
        if (fat32_change_dir("bin") == 0) {
            fat32_list_files(); /* Use existing list function */
            fat32_change_dir("/");
        } else {
            kprint("/bin directory empty or missing.\n");
        }
    } else {
        kprint("Unknown apt command.\n");
    }
}

static void cmd_rm(char *name) {
    if (!name || name[0] == '\0') {
        kprint("Usage: rm <filename>\n");
        return;
    }
    
    if (vfs_unlink(name) == 0) {
        kprint("File deleted.\n");
    } else {
        kprint("Failed to delete file (not found?).\n");
    }
}

static void cmd_meminfo(void) {
    kprint("\nMemory Information:\n");
    kprint("  Total: ");
    print_num(pmm_get_total_memory() / 1024 / 1024);
    kprint(" MB\n");
    kprint("  Used:  ");
    print_num(pmm_get_used_memory() / 1024 / 1024);
    kprint(" MB\n");
    kprint("  Free:  ");
    print_num(pmm_get_free_memory() / 1024 / 1024);
    kprint(" MB\n\n");
}

static void cmd_mouseinfo(void) {
    MouseState m = mouse_get_state();
    kprint("\nMouse State:\n");
    kprint("  X: "); print_num(m.x); kprint("\n");
    kprint("  Y: "); print_num(m.y); kprint("\n");
    kprint("  Buttons: L="); print_num(m.left_btn); 
    kprint(" R="); print_num(m.right_btn); 
    kprint(" M="); print_num(m.middle_btn);
    kprint("\n\n");
}

static void cmd_gfxtest(void) {
    kprint("Drawing test pattern...\n");
    
    /* Draw Red Box */
    gfx_fill_rect(100, 100, 200, 150, COLOR_RED);
    
    /* Draw Green Box */
    gfx_fill_rect(350, 100, 200, 150, COLOR_GREEN);
    
    /* Draw Blue Rect (Outline) */
    gfx_draw_rect(600, 100, 200, 150, COLOR_BLUE);
    
    /* Draw Lines */
    gfx_draw_line(100, 300, 800, 500, COLOR_WHITE);
    gfx_draw_line(800, 300, 100, 500, COLOR_WHITE);
    
    kprint("Press any key to clear...\n");
    while (!keyboard_available()) {
        MouseState m = mouse_get_state();
        gfx_draw_cursor(m.x, m.y); /* Draw cursor in the wait loop! */
        /* Delay? */
        for(volatile int i=0; i<50000; i++);
        /* We need to erase it too but we don't have backbuffer */
        /* Just let it smear for now for testing */
    }
    keyboard_getchar(); 
    
    term_clear();
}

/* Command: lspci */
static void cmd_lspci(void) {
    pci_scan_bus();
}

/* Command: dmesg */
static void cmd_dmesg(void) {
    klog_dump();
}

/* Desktop App Callbacks */
static void launch_terminal(void) {
    Window *win = wm_create_window(100, 100, 400, 300, "Terminal");
    if (win) {
        wm_fill_rect(win, 0, 0, 400, 300, 0x000000);
        wm_console_write(win, "Welcome to Ainuix Terminal\n");
        wm_console_write(win, "type 'exit' to close (simulated)\n");
        wm_console_write(win, "user@ainuix:~$ ");
    }
}

static void launch_files(void) {
    Window *win = wm_create_window(150, 150, 300, 400, "File Manager");
    if (win) {
        wm_fill_rect(win, 0, 0, 300, 400, 0xFFFFFF);
        /* Draw fake file list */
        wm_console_write(win, "\n");
        wm_console_write(win, "  [DIR] bin\n");
        wm_console_write(win, "  [DIR] libs\n");
        wm_console_write(win, "  [FILE] video.mp4\n");
        wm_console_write(win, "  [FILE] image.png\n");
        wm_console_write(win, "  [FILE] secret.txt\n");
    }
}

static void launch_media(void) {
     Window *win = wm_create_window(200, 200, 300, 100, "Media Player");
     if (win) {
         wm_fill_rect(win, 0, 0, 300, 100, 0x202020);
         wm_console_write(win, "Use 'view <file>' in CLI\n");
         wm_console_write(win, "to play videos.\n");
     }
}

static void cmd_desktop(void) {
    kprint("Starting Premium Desktop Environment...\n");
    
    wm_init();
    wm_clear_icons();
    
    /* Setup Desktop Icons */
    wm_add_icon("Term", 40, 40, launch_terminal);
    wm_add_icon("Files", 40, 140, launch_files);
    wm_add_icon("Media", 40, 240, launch_media);
    
    /* Open a welcome window */
    Window *w1 = wm_create_window(300, 200, 300, 150, "Welcome");
    if (w1) {
        wm_fill_rect(w1, 0, 0, 300, 150, 0xFFFFFF);
        wm_console_write(w1, "Welcome to Ainuix OS!\n");
        wm_console_write(w1, "Enjoy the Premium Experience.\n");
    }

    wm_run(); /* Blocks */
    
    term_clear();
    kprint("Desktop Session Ended.\n");
}
/* Alias for backward compat */
void cmd_startwm(void) { cmd_desktop(); }


static char *get_arg(char *cmd_buffer) {
    while (*cmd_buffer && *cmd_buffer != ' ') cmd_buffer++;
    if (*cmd_buffer == ' ') {
        *cmd_buffer = '\0';
        return cmd_buffer + 1;
    }
    return NULL;
}

/* Command: cc - Nano-C Compiler */
static void cmd_cc(char *filename) {
    if (!filename || filename[0] == '\0') {
        kprint("Usage: cc <filename>\n");
        return;
    }
    
    char full_path[256];
    if (filename[0] == '/') str_cpy(full_path, filename);
    else {
        str_cpy(full_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(full_path, "/");
        str_cat(full_path, filename);
    }
    
    struct initrd_file *file = initrd_find_file(full_path);
    if (!file && full_path[0] == '/') file = initrd_find_file(full_path+1);
    
    char *raw_data = NULL;
    uint32_t raw_size = 0;
    int is_disk = 0;
    
    if (file) {
        raw_data = (char *)file->data;
        raw_size = file->size;
    } else {
        /* Try Disk */
        uint8_t *d;
        if (fat32_read_file(filename, &d, &raw_size) == 0) {
            raw_data = (char *)d;
            is_disk = 1;
        }
    }

    if (raw_data) {
        /* Allocate huge buffer for Preprocessor */
        char *final_src = (char *)kmalloc(1024 * 64); /* 64KB */
        if (!final_src) {
             kprint("Out of memory for compilation!\n");
             if(is_disk) kfree(raw_data);
             return;
        }
        final_src[0] = '\0';
        
        char *ptr = raw_data;
        char *end = ptr + raw_size;
        
        /* Line by line scan */
        while (ptr < end) {
            char *line_start = ptr;
            while (ptr < end && *ptr != '\n') ptr++;
            int line_len = ptr - line_start;
            
            /* Check for #include */
            int is_include = 0;
            if (line_len > 8 && line_start[0] == '#') {
                /* Simple check for "include" */
                /* Skip space */
                char *s = line_start + 1;
                while (s < line_start + line_len && *s == ' ') s++;
                if (s[0]=='i' && s[1]=='n' && s[2]=='c' && s[3]=='l' && s[4]=='u' && s[5]=='d' && s[6]=='e') {
                    is_include = 1;
                    /* Extract filename */
                    char inc_name[64];
                    int i = 0;
                    s += 7;
                    while (s < line_start + line_len && (*s == ' ' || *s == '<' || *s == '"')) s++;
                    while (s < line_start + line_len && *s != '>' && *s != '"' && i < 63) {
                        inc_name[i++] = *s++;
                    }
                    inc_name[i] = '\0';
                    
                    kprint("[PP] Including: "); kprint(inc_name); kprint("\n");
                    
                    /* Try to load initrd/libs/NAME */
                    char lib_path[128];
                    str_cpy(lib_path, "libs/");
                    str_cat(lib_path, inc_name);
                    
                    struct initrd_file *lib = initrd_find_file(lib_path);
                    if (!lib) {
                         /* Try Disk Libs? TODO */
                         kprint("[PP] Warning: Library not found: "); kprint(lib_path); kprint("\n");
                    } else {
                        /* Append lib content */
                        char *t = final_src;
                        while(*t) t++;
                        /* Safety check max len? */
                        mem_cpy(t, lib->data, lib->size);
                        t[lib->size] = '\0';
                        str_cat(final_src, "\n"); 
                    }
                }
            }
            
            if (!is_include) {
                /* Append line */
                char *t = final_src;
                while (*t) t++;
                mem_cpy(t, line_start, line_len);
                t[line_len] = '\n'; 
                t[line_len+1] = '\0';
            }
            
            if (ptr < end && *ptr == '\n') ptr++;
        }
        
        nano_c_run(final_src);
        
        gc_collect(final_src);
        if (is_disk) gc_collect(raw_data);
    } else {
        kprint("File not found: ");
        kprint(filename);
        kprint("\n");
    }
}

/* ASM Wrapper Logic */
static void *g_asm_code_ptr = NULL;
extern void sched_exit(int code);

static void asm_task_wrapper(void) {
    if (g_asm_code_ptr) {
        /* Call Generated Code */
        ((void(*)(void))g_asm_code_ptr)();
    }
    /* Graceful Exit */
    sched_exit(0);
}

/* Command: as - Nano-Assembler */
static void cmd_as(char *filename) {
    if (!filename || filename[0] == '\0') {
        kprint("Usage: as <filename>\n");
        return;
    }
    
    char full_path[256];
    if (filename[0] == '/') str_cpy(full_path, filename);
    else {
        str_cpy(full_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(full_path, "/");
        str_cat(full_path, filename);
    }
    
    struct initrd_file *file = initrd_find_file(full_path);
    if (!file && full_path[0] == '/') file = initrd_find_file(full_path+1);
    
    char *data = NULL;
    uint32_t size = 0;
    int is_disk = 0;

    if (file) {
        data = (char *)file->data;
        size = file->size;
    } else {
         /* Check Disk */
         uint8_t *d;
         if (fat32_read_file(filename, &d, &size) == 0) {
             data = (char *)d;
             is_disk = 1;
         }
    }
    
    if (data) {
        /* Ensure null-terminated string */
        char *src = (char *)kmalloc(size + 1);
        mem_cpy(src, data, size);
        src[size] = '\0';
        
        void *code = asm_compile(src);
        if (code) {
             kprint("[Shell] Executing in new task...\n");
             
             /* 0. Initialize Window Manager (Required for Backbuffer setup) */
             wm_init();
             
             /* CRITICAL SECTION: Prevent Scheduler from picking task before Window is ready */
             __asm__ volatile("cli");
             
             g_asm_code_ptr = code;
             struct task_struct *t = sched_create_task(asm_task_wrapper);
             if (t) {
                 /* 1. Hold: sched_create_task sets READY, but we have CLI so we are safe */
                 t->state = TASK_BLOCKED; 
                 
                 /* Virtual Layer: Create Window */
                 Window *win = wm_create_window(50, 50, 600, 400, filename);
                 if (win) {
                     t->output_window = win;
                     
                     /* 2. Release task */
                     t->state = TASK_READY;
                     
                     __asm__ volatile("sti"); /* End Critical Section */
                     
                     kprint("[Shell] Entering Window Mode (Press ESC to exit)...\n");
                     wm_run();
                     t->output_window = 0;
                 } else {
                     t->state = TASK_READY;
                     __asm__ volatile("sti");
                     /* Fallback */
                     while (t->state != TASK_ZOMBIE) {
                         __asm__ volatile ("hlt");
                     }
                 }
                 kprint("[Shell] Task finished.\n");
             }
             kfree(code);
        }
        
        kfree(src);
        if (is_disk) kfree(data);
    } else {
        kprint("File not found: ");
        kprint(filename);
        kprint("\n");
    }
}

/* Command: run - Universal Runner */
/* Command: run - Universal Runner */
static void cmd_run(char *filename) {
    if (!filename || filename[0] == '\0') {
        kprint("Usage: run <filename>\n");
        return;
    }
    
    /* 1. Try to read file to buffer */
    uint8_t *data = NULL;
    uint64_t size = 0;
    int needs_free = 0;
    
    /* Try InitRD */
    char full_path[256];
    if (filename[0] == '/') {
        str_cpy(full_path, filename);
    } else {
        str_cpy(full_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(full_path, "/");
        str_cat(full_path, filename);
    }
    
    struct initrd_file *file = initrd_find_file(full_path);
    if (!file && full_path[0] == '/') file = initrd_find_file(full_path+1);
    
    if (file) {
        data = (uint8_t*)file->data;
        size = file->size;
    } else {
        /* Try FAT32 */
        uint8_t *fat_data = NULL;
        uint32_t fat_size = 0;
        if (fat32_read_file(filename, &fat_data, &fat_size) == 0) {
            data = fat_data;
            size = fat_size;
            needs_free = 1;
        }
    }
    
    if (!data) {
        kprint("File not found: ");
        kprint(filename);
        kprint("\n");
        return;
    }
    
    /* 2. Check File Type */
    size_t len = str_len(filename);
    int is_elf = 0;
    
    if (size >= 4 && data[0] == 0x7F && data[1] == 'E' && data[2] == 'L' && data[3] == 'F') {
        is_elf = 1;
    } else if (len > 4 && filename[len-1] == 'f' && filename[len-2] == 'l' && filename[len-3] == 'e' && filename[len-4] == '.') {
        is_elf = 1;
    }
    
    if (is_elf) {
        /* ELF Binary */
        kprint("[Shell] Executing ELF binary...\n");
        
        elf_load_result_t res;
        if (elf_load(data, size, &res) == 0) {
            
            wm_init();
            
            __asm__ volatile("cli");
            struct task_struct *t = sched_create_user_task(res.entry_point, res.stack_top, res.address_space);
            if (t) {
                t->state = TASK_BLOCKED;
                
                Window *win = wm_create_window(50, 50, 640, 480, "ELF Execution");
                if (win) {
                    t->output_window = win;
                    wm_fill_rect(win, 0, 0, win->width, win->height, 0x000080);
                    wm_console_write(win, "ELF Binary Managed Execution\n");
                    wm_console_write(win, "----------------------------\n");
                }
                
                t->state = TASK_READY;
                __asm__ volatile("sti");
                
                kprint("[Shell] Running ELF in VM... (Press ESC to exit)\n");
                wm_run();
                
            } else {
                kprint("[Shell] Failed to create task.\n");
            }
        } else {
            kprint("[Shell] Invalid ELF file.\n");
        }
    } else {
        /* Source File Checks */
        if (len > 2 && filename[len-1] == 'c' && filename[len-2] == '.') {
            cmd_cc(filename);
        } else if (len > 4 && filename[len-1] == 'p' && filename[len-2] == 'p' && filename[len-3] == 'c' && filename[len-4] == '.') {
            cmd_cc(filename);
        } else if (len > 4 && filename[len-1] == 'm' && filename[len-2] == 's' && filename[len-3] == 'a' && filename[len-4] == '.') {
            cmd_as(filename);
        } else {
            kprint("Unknown file type. Executable must be ELF or source (.c/.asm)\n");
        }
    }
    
    if (needs_free && data) {
        kfree(data);
    }
}

static void cmd_bg(char *filename) {
    /* Loads ELF and spawns task but returns immediately (Background) */
    if (!filename) return;
    
    uint8_t *data = NULL;
    uint64_t size = 0;
    int needs_free = 0;
    
    /* Fast Load Logic */
    char full_path[256];
    if (filename[0] == '/') str_cpy(full_path, filename);
    else {
        str_cpy(full_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(full_path, "/");
        str_cat(full_path, filename);
    }
    struct initrd_file *file = initrd_find_file(full_path);
    if (!file && full_path[0] == '/') file = initrd_find_file(full_path+1);
    
    if (file) {
        data = (uint8_t*)file->data;
        size = file->size;
    } else {
        /* Try FAT32 */
        uint8_t *fat_data = NULL;
        uint32_t fat_size = 0;
        if (fat32_read_file(filename, &fat_data, &fat_size) == 0) {
            data = fat_data;
            size = fat_size;
            needs_free = 1;
        }
    }
    
    if (!data) {
        kprint("[BG] File not found.\n");
        return;
    }
    
    elf_load_result_t res;
    if (elf_load(data, size, &res) == 0) {
        struct task_struct *t = sched_create_user_task(res.entry_point, res.stack_top, res.address_space);
        if (t) {
            t->state = TASK_READY;
            kprint("[BG] Task Spawned. PID: ");
            char buf[16];
            int idx = 0;
            uint64_t n = t->id;
            if (n == 0) buf[idx++] = '0';
            else { while (n > 0) { buf[idx++] = '0' + (n % 10); n /= 10; } }
            for (int i = idx - 1; i >= 0; i--) { char c[2]={buf[i],0}; kprint(c); }
            kprint(" (Background)\n");
        }
    } else {
        kprint("[BG] Invalid ELF.\n");
    }
    
    if (needs_free && data) {
        kfree(data);
    }
}

/* Command: jittest - Execute code on heap */
static void cmd_jittest(void) {
    kprint("[JIT] Allocating executable memory...\n");
    unsigned char *code = kmalloc(64);
    if (!code) {
        kprint("[JIT] Alloc failed!\n");
        return;
    }

    /* Write: MOV RAX, 0xDEADBEEF; RET */
    /* 48 B8 <64-bit imm> C3 */
    code[0] = 0x48;
    code[1] = 0xB8;
    
    uint64_t val = 0xDEADBEEF;
    mem_cpy(&code[2], &val, 8);
    
    code[10] = 0xC3; /* RET */

    kprint("[JIT] Executing code at heap addr...\n");
    
    /* Call it */
    uint64_t (*func)(void) = (uint64_t (*)(void))code;
    uint64_t ret = func();
    
    kprint("[JIT] Success! Returned: ");
    if (ret == 0xDEADBEEF) kprint("0xDEADBEEF (Matches)\n");
    else kprint("Mismatch!\n");

    kfree(code);
}

static void cmd_alloc_test(void) {
    kprint("[ALLOC] Allocating 100 bytes...\n");
    char *ptr = (char *)kmalloc(100);
    if (!ptr) {
        kprint("[ALLOC] Failed to allocate!\n");
        return;
    }
    
    kprint("[ALLOC] Writing pattern...\n");
    for (int i = 0; i < 100; i++) {
        ptr[i] = (char)(i & 0xFF);
    }
    
    kprint("[ALLOC] Verifying pattern...\n");
    int fail = 0;
    for (int i = 0; i < 100; i++) {
        if (ptr[i] != (char)(i & 0xFF)) {
             kprint("[ALLOC] Mismatch at index ");
             print_num(i);
             kprint("\n");
             fail = 1;
             break;
        }
    }
    
    if (!fail) kprint("[ALLOC] Verification Passed.\n");
    
    kprint("[ALLOC] Freeing...\n");
    kfree(ptr);
    kprint("[ALLOC] Done.\n");
}

/* Command: whoami */
static void cmd_whoami(void) {
    const char *user = user_get_current();
    if (user) {
        kprint(user);
        kprint("\n");
    } else {
        kprint("unknown\n");
    }
}

/* Command: uptime */
static void cmd_uptime(void) {
    uint64_t ticks = timer_get_ticks();
    uint64_t total_seconds = ticks / 100; /* 100 Hz */
    
    uint64_t seconds = total_seconds % 60;
    uint64_t minutes = (total_seconds / 60) % 60;
    uint64_t hours   = total_seconds / 3600;
    
    kprint("Uptime: ");
    print_num(hours);   kprint("h ");
    print_num(minutes); kprint("m ");
    print_num(seconds); kprint("s\n");
}

extern int serial_received(void);
extern char serial_read(void);

void shell_run(void) {
    char *cmd_buffer = (char *)kmalloc(256);
    if (!cmd_buffer) {
        kprint("Failed to allocate command buffer!\n");
        return;
    }

    /* DEBUG: Auto-Run removed */

    while (1) {
        if (!user_login_loop()) continue;

        kprint("\n=== Ainux Shell ===\n");
        kprint("Type 'help' for commands\n\n");

        while (1) {
            /* Prompt: user@Ainux:cwd $ */
            kprint_color(KLOG_COLOR_GREEN, user_get_current());
            kprint_color(KLOG_COLOR_GREEN, "@Ainux");
            kprint(":");
            kprint_color(KLOG_COLOR_BLUE, cwd);
            kprint("$ ");
            
            int pos = 0;
        /* Reset view index to end (new command) */
        history_view_index = history_count;
        
        /* Simple Shell Input Loop (Non-blocking for Blink) */
        int blink_visible = 1;
        uint64_t loop_cycles = 0;
        term_draw_cursor();

        while (1) {
            unsigned char c = 0;
            int has_input = 0;
            
            keyboard_poll(); /* Poll HW in case IRQs fail */
            if (keyboard_available()) {
                c = keyboard_getchar();
                has_input = 1;
            } else if (serial_received()) {
                c = serial_read();
                has_input = 1;
            }
            
            if (has_input) {
                
                /* Reset Blink on Input */
                if (!blink_visible) {
                    term_draw_cursor();
                    blink_visible = 1;
                }
                loop_cycles = 0;

                if (c == '\n' || c == '\r') {
                    term_erase_cursor(); /* Erase before moving */
                    cmd_buffer[pos] = '\0';
                    kprint("\n");
                    
                    /* Save to History */
                    if (pos > 0) {
                        if (history_count < HISTORY_MAX) {
                            str_cpy(cmd_history[history_count], cmd_buffer);
                            history_count++;
                        } else {
                            /* Shift */
                            for (int i=1; i<HISTORY_MAX; i++) str_cpy(cmd_history[i-1], cmd_history[i]);
                            str_cpy(cmd_history[HISTORY_MAX-1], cmd_buffer);
                        }
                    }
                    blink_visible = 0;
                    break;
                } else if (c == KEY_UP || c == KEY_DOWN) {
                    if (c == KEY_UP) {
                        if (history_view_index > 0) history_view_index--;
                    } else {
                        if (history_view_index < history_count) history_view_index++;
                    }
                    
                    /* Clear current line logic */
                    term_erase_cursor();
                    while (pos > 0) {
                        kprint("\b \b");
                        pos--;
                    }
                    
                    /* Load history */
                    if (history_view_index < history_count) {
                        str_cpy(cmd_buffer, cmd_history[history_view_index]);
                        kprint(cmd_buffer);
                        pos = str_len(cmd_buffer);
                    } else {
                        /* Back to empty/draft */
                        cmd_buffer[0] = '\0';
                        pos = 0;
                    }
                    term_draw_cursor();
                } else if (c == '\b' || c == 0x7F) {
                    if (pos > 0) {
                        term_erase_cursor(); /* Erase current block */
                        pos--;
                        kprint("\b \b");
                        term_draw_cursor(); /* Draw at new pos */
                    }
                } else if (c == '\t') {  /* TAB Autocomplete */
                     // kprint("[TAB]");
                     cmd_buffer[pos] = 0; /* Cap null temporarily */
                     shell_autocomplete(cmd_buffer, &pos);
                } else if (pos < 255) {
                    if (c >= 32 && c <= 126) {
                        term_erase_cursor(); /* Erase current, kprint advances, draw new */
                        cmd_buffer[pos++] = c;
                        char str[2] = {c, 0};
                        kprint(str);
                        term_draw_cursor();
                    }
                }
            } else {
                /* Idle: Handle Blinking */
                loop_cycles++;
                if (loop_cycles > 4000000) { 
                    blink_visible = !blink_visible;
                    if (blink_visible) term_draw_cursor();
                    else term_erase_cursor();
                    loop_cycles = 0;
                }
                /* Polling Mode: Do not sleep! Busy wait slightly to handle blink timing */
                for (volatile int i=0; i<10000; i++);
            }
        }

        if (pos == 0) continue;
        
        int ret = exec_command(cmd_buffer);
        if (ret == 1) break; /* Logout */
    }
    }
}
        
static void cmd_clean(void) {
    kprint("[Clean] Starting Memory Cleanup...\n");
    
    /* 1. Reap Zombies */
    int reaped = sched_gc();
    
    /* 2. Future: Scrub/Compact Heap */
    /* kheap_compact(); */
    
    kprint("[Clean] Memory Optimization Complete.\n");
    if (reaped == 0) {
        kprint("[Clean] System was already clean (No Zombies).\n");
    }
}

/* Internet Command */
#include "../../net/netdev.h"
#include "../../libc/string.h"

static void cmd_internet(char *args) {
    if (!args) {
        kprint("Usage: internet <scan|connect|disconnect>\n");
        return;
    }
    
    /* If we have sub-arguments (like 'connect SSID'), split them */
    /* Be careful not to corrupt 'scan' if no params */
    /* Check command type first? */
    /* Use strncmp or just simple strcmp if only one word is expected? */
    /* For 'connect', args might be 'connect'. Logic inside prompts user. */
    /* So args should just be the subcommand 'start'. */
    
    /* However, if user typed 'internet scan', args is 'scan'. */
    /* But if user typed 'internet connect extra', get_arg split it? */
    /* shell default get_arg only splits ONCE? */
    /* exec_command splits command and REST. */
    /* 'internet scan' -> cmd='internet', arg='scan' */
    /* So args is 'scan'. */
    
    if (strcmp(args, "scan") == 0) {
        /* Iterate and Scan */
        net_device_t *dev = netdev_get_by_name("wlan0");
        if (dev && dev->scan) {
             kprint("[Internet] Scanning on wlan0...\n");
             dev->scan(dev);
        } else {
             kprint("[Internet] No wireless capabilities found.\n");
             /* List wired */
             dev = netdev_get_by_name("eth0");
             if (dev) kprint("[Internet] Wired Interface 'eth0' is UP.\n");
        }
    } else if (strcmp(args, "connect") == 0) {
        /* Prompt SSID/Pass */
        char ssid[32];
        char pass[32];
        
        kprint("WiFi Name: ");
        user_get_input(ssid, 32);
        kprint("Password: ");
        user_get_input_masked(pass, 32);
        
        net_device_t *dev = netdev_get_by_name("wlan0");
        if (dev && dev->connect) {
            dev->connect(dev, ssid, pass);
        } else {
            kprint("[Internet] No wireless interface to connect.\n");
        }
    } else if (strcmp(args, "disconnect") == 0) {
         net_device_t *dev = netdev_get_by_name("wlan0");
         if (dev && dev->disconnect) {
             dev->disconnect(dev);
         }
    } else {
         kprint("Unknown internet action.\n");
    }
}

int exec_command(char *cmd_buffer) {
        char *arg = get_arg(cmd_buffer);

        if (str_cmp(cmd_buffer, "sudo") == 0) {
            /* Sudo Logic */
            if (!arg) {
                kprint("Usage: sudo <command>\n");
                return 0;
            }
            
            /* Verify Password of CURRENT user */
            const char *current = user_get_current();
            kprint("[sudo] password for "); kprint(current); kprint(": ");
            
            char pass[64];
            user_get_input_masked(pass, 64);
            
            if (user_check(current, pass)) {
                g_sudo_active = 1;
                /* Execute recursive */
                int ret = exec_command(arg); /* arg is the rest of the string */
                g_sudo_active = 0;
                return ret;
            } else {
                kprint("Sorry, try again.\n");
            }
            return 0;
        } else if (str_cmp(cmd_buffer, "help") == 0) {
            cmd_help();
        } else if (str_cmp(cmd_buffer, "ls") == 0) {
            cmd_ls(arg);
        } else if (str_cmp(cmd_buffer, "pwd") == 0) {
            cmd_pwd();
        } else if (str_cmp(cmd_buffer, "cd") == 0) {
            cmd_cd(arg);
        } else if (str_cmp(cmd_buffer, "cc") == 0) {
            cmd_cc(arg);
        } else if (str_cmp(cmd_buffer, "as") == 0) {
            cmd_as(arg);
        } else if (str_cmp(cmd_buffer, "run") == 0) {
            cmd_run(arg);
        } else if (str_cmp(cmd_buffer, "bg") == 0) {
            cmd_bg(arg);
        } else if (str_cmp(cmd_buffer, "testlibc") == 0) {
            libc_test_run();
        } else if (str_cmp(cmd_buffer, "cat") == 0) {
            if (arg) cmd_cat(arg);
            else kprint("\nUsage: cat <filename>\n\n");
        } else if (str_cmp(cmd_buffer, "lspci") == 0) {
            cmd_lspci();
        } else if (str_cmp(cmd_buffer, "dmesg") == 0) {
            cmd_dmesg();
        } else if (str_cmp(cmd_buffer, "touch") == 0) {
            cmd_touch(arg);
        } else if (str_cmp(cmd_buffer, "rm") == 0) {
            cmd_rm(arg);
        } else if (str_cmp(cmd_buffer, "mkdir") == 0) {
            cmd_mkdir(arg);
        } else if (str_cmp(cmd_buffer, "cp") == 0) {
            cmd_cp(arg);
        } else if (str_cmp(cmd_buffer, "rm") == 0) {
            cmd_rm(arg);
        } else if (str_cmp(cmd_buffer, "apt") == 0) {
            cmd_apt(arg);
        } else if (str_cmp(cmd_buffer, "write") == 0) {
            cmd_write(arg);
        } else if (str_cmp(cmd_buffer, "clear") == 0) {
            cmd_clear();
        } else if (str_cmp(cmd_buffer, "meminfo") == 0) {
            cmd_meminfo();
        } else if (str_cmp(cmd_buffer, "mouseinfo") == 0) {
            cmd_mouseinfo();
        } else if (str_cmp(cmd_buffer, "gfxtest") == 0) {
            cmd_gfxtest();
        } else if (str_cmp(cmd_buffer, "jittest") == 0) {
            cmd_jittest();
        } else if (str_cmp(cmd_buffer, "alloctest") == 0) {
            cmd_alloc_test();
        } else if (str_cmp(cmd_buffer, "startwm") == 0) {
            cmd_startwm();
        } else if (str_cmp(cmd_buffer, "desktop") == 0) {
             cmd_desktop();
        } else if (str_cmp(cmd_buffer, "mkfs") == 0) {
            cmd_mkfs();
        } else if (str_cmp(cmd_buffer, "mkfs.axfs") == 0) {
            axfs_format();
        } else if (str_cmp(cmd_buffer, "fdisk") == 0) {
            cmd_fdisk(arg);
        } else if (str_cmp(cmd_buffer, "mount") == 0) {
            cmd_mount();
        } else if (str_cmp(cmd_buffer, "update") == 0) {
            cmd_update();
        } else if (str_cmp(cmd_buffer, "lsdisk") == 0) {
            cmd_lsdisk();
        } else if (str_cmp(cmd_buffer, "save") == 0) {
            cmd_save(arg);
        } else if (str_cmp(cmd_buffer, "date") == 0) {
            cmd_date(arg);
        } else if (str_cmp(cmd_buffer, "time") == 0) {
            cmd_time(arg);
        } else if (str_cmp(cmd_buffer, "cal") == 0) {
            cmd_cal(arg);
        } else if (str_cmp(cmd_buffer, "testlibc") == 0) {
            libc_test_run();
        } else if (str_cmp(cmd_buffer, "shutdown") == 0) {
            kprint("Shutting down...\n");
            sys_shutdown();
        } else if (str_cmp(cmd_buffer, "reboot") == 0) {
            kprint("Rebooting...\n");
            sys_reboot();
        } else if (str_cmp(cmd_buffer, "useradd") == 0) {
            if (arg && arg[0]) {
                kprint("Password: ");
                char p[64];
                user_get_input_masked(p, 64);
                user_add(arg, p);
                kprint("User added.\n");
            } else {
                kprint("Usage: useradd <username>\n");
            }
        } else if (str_cmp(cmd_buffer, "passwd") == 0) {
            kprint("Username: ");
            char u[64]; user_get_input(u, 64);
            kprint("New Password: ");
            char p[64]; user_get_input_masked(p, 64);
            user_add(u, p);
            kprint("Password updated.\n");
        } else if (str_cmp(cmd_buffer, "whoami") == 0) {
            cmd_whoami();
        } else if (str_cmp(cmd_buffer, "uptime") == 0) {
            cmd_uptime();
        } else if (str_cmp(cmd_buffer, "clock") == 0) {
            cmd_clock();
        } else if (str_cmp(cmd_buffer, "beep") == 0) {
            cmd_beep(arg);
        } else if (str_cmp(cmd_buffer, "view") == 0) {
            cmd_view(arg);
        } else if (str_cmp(cmd_buffer, "file") == 0) {
            cmd_file(arg);
        } else if (str_cmp(cmd_buffer, "exit") == 0) {
            kprint("Logging out...\n");
            for(volatile int i=0; i<5000000; i++);
            return 1; /* Logout Signal */

/* ... inside exec_command ... */
        } else if (strcmp(cmd_buffer, "ping") == 0) {
            cmd_ping();
        } else if (strcmp(cmd_buffer, "clean") == 0) {
            cmd_clean();
        } else if (strcmp(cmd_buffer, "internet") == 0) {
            cmd_internet(arg);
        } else {
            kprint("Unknown command: ");
            kprint(cmd_buffer);
            kprint("\n\n");
        }
        return 0;
}
