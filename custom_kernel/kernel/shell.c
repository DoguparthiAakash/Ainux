#include "shell.h"
#include "drivers/keyboard.h"
#include "drivers/mouse.h"
#include "fs/initrd.h"
#include "mm/pmm.h"
#include "mm/heap.h"
#include "nano_c.h"
#include "gfx.h"
#include "fs/fat32.h"
#include "fs/mbr.h"
#include "wm.h"

extern void libc_test_run(void);

extern void kprint(const char *msg);
extern void kprint_buf(const char *buf, uint64_t len);
extern void term_clear(void);
extern void term_set_cursor(int x, int y);
extern void term_draw_cursor(void); 
extern void term_erase_cursor(void);

/* Current Working Directory */
static char cwd[256] = "/";
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
                    initrd_delete_file(editor_filename);
                    initrd_create_file(editor_filename, editor_buffer, editor_buf_len);
                    term_set_cursor(0, 47); kprint("Saved!       ");
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
    
    /* Find file in InitRD */
    /* This assumes filename is relative to CWD or absolute InitRD path */
    /* We reuse resolving logic or just search simple */
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

static void cmd_help(void) {
    kprint("\nAvailable commands:\n");
    kprint("  help      - Show this help\n");
    kprint("  ls        - List files\n");
    kprint("  pwd       - Show current directory\n");
    kprint("  cd DIR    - Change directory\n");
    kprint("  cat FILE  - Display file\n");
    kprint("  touch F   - Create file\n");
    kprint("  mkdir D   - Create dir\n");
    kprint("  cp S D    - Copy file\n");
    kprint("  rm F      - Remove file\n");
    kprint("  write F   - Text Editor\n");
    kprint("  cc F      - Compile C Script\n");
    kprint("  cc F      - Compile C Script\n");
    kprint("  fdisk [args]- Disk Manager (map, new <MB>, reset)\n");
    kprint("  mkfs      - Format Partition 1 (FAT32)\n");
    kprint("  mount     - Mount Disk\n");
    kprint("  lsdisk    - List Disk Files\n");
    kprint("  save F    - Save InitRD file to Disk\n");
    kprint("  testlibc  - Run libc tests\n");
    kprint("\n");
}

/* Command: fdisk */
static void cmd_fdisk(char *arg) {
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
        kprint("\n");
    } else {
        kprint("\nDirectory not found: ");
        kprint(target);
        kprint("\n\n");
    }
}

/* Command: ls */
static void cmd_ls(char *arg) {
    kprint("\n");
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
    kprint("\n");
}

/* Command: cat */
static void cmd_cat(const char *filename) {
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
    char full_path[256];
    if (name[0] == '/') str_cpy(full_path, name);
    else {
        str_cpy(full_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(full_path, "/");
        str_cat(full_path, name);
    }
    
    if (initrd_create_file(full_path, NULL, 0) == 0) {
        kprint("File created.\n");
    } else {
        kprint("Failed to create file.\n");
    }
}

static void cmd_mkdir(char *name) {
    if (!name || name[0] == '\0') {
        kprint("Usage: mkdir <dirname>\n");
        return;
    }
    char full_path[256];
    if (name[0] == '/') str_cpy(full_path, name);
    else {
        str_cpy(full_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(full_path, "/");
        str_cat(full_path, name);
    }
    if (initrd_create_dir(full_path) == 0) {
        kprint("Directory created.\n");
    } else {
        kprint("Failed to create dir.\n");
    }
}

static void cmd_cp(char *args) {
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
    
    char src_path[256];
    if (src[0] == '/') str_cpy(src_path, src);
    else {
        str_cpy(src_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(src_path, "/");
        str_cat(src_path, src);
    }
    struct initrd_file *sfile = initrd_find_file(src_path);
    if (!sfile && src_path[0]=='/') sfile = initrd_find_file(src_path+1);
    
    if (!sfile) {
        kprint("Source not found.\n");
        return;
    }
    
    char dest_path[256];
    if (dest[0] == '/') str_cpy(dest_path, dest);
    else {
        str_cpy(dest_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(dest_path, "/");
        str_cat(dest_path, dest);
    }
    if (initrd_create_file(dest_path, (char*)sfile->data, sfile->size) == 0) {
        kprint("File copied.\n");
    } else {
        kprint("Copy failed.\n");
    }
}

static void cmd_rm(char *name) {
    if (!name || name[0] == '\0') {
        kprint("Usage: rm <filename>\n");
        return;
    }
    char full_path[256];
    if (name[0] == '/') str_cpy(full_path, name);
    else {
        str_cpy(full_path, cwd);
        if (str_cmp(cwd, "/") != 0) str_cat(full_path, "/");
        str_cat(full_path, name);
    }
    if (initrd_delete_file(full_path) == 0) {
        kprint("File deleted.\n");
    } else {
        kprint("Delete failed (not found?)\n");
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

static void cmd_startwm(void) {
    kprint("Starting Window Manager...\n");
    
    wm_init();
    
    Window *w1 = wm_create_window(50, 50, 300, 200, "Window 1");
    if (w1) {
        /* Draw something in w1 */
        for (int y=0; y<200; y++) {
            for (int x=0; x<300; x++) {
                wm_draw_window_content(w1, x, y, 0xFF0000); /* Red */
            }
        }
    }
    
    Window *w2 = wm_create_window(400, 100, 250, 250, "Window 2");
    if (w2) {
        /* Draw something in w2 */
        for (int y=0; y<250; y++) {
            for (int x=0; x<250; x++) {
                wm_draw_window_content(w2, x, y, 0x00FF00); /* Green */
            }
        }
    }
    
    Window *w3 = wm_create_window(100, 300, 200, 150, "Small Win");
    if (w3) {
         for (int y=0; y<150; y++) {
            for (int x=0; x<200; x++) {
                wm_draw_window_content(w3, x, y, 0x0000FF); /* Blue */
            }
        }
    }
    
    wm_run(); /* Blocks until exit */
    
    /* Restore Text Mode (kind of, we just clear and reset cursor) */
    term_clear();
    kprint("Window Manager Exited.\n");
}


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
    
    if (file) {
        /* Ensure null-terminated string for compiler */
        char *src = (char *)kmalloc(file->size + 1);
        mem_cpy(src, file->data, file->size);
        src[file->size] = '\0';
        
        nano_c_run(src);
        
        kfree(src);
    } else {
        kprint("File not found: ");
        kprint(filename);
        kprint("\n");
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

extern int serial_received(void);
extern char serial_read(void);

void shell_run(void) {
    char *cmd_buffer = (char *)kmalloc(256);
    if (!cmd_buffer) {
        kprint("Failed to allocate command buffer!\n");
        return;
    }

    kprint("\n=== Ainux Shell ===\n");
    kprint("Type 'help' for commands\n\n");

    while (1) {
        kprint(cwd);
        kprint(" $ ");
        
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
                } else if (c == '\b') {
                    if (pos > 0) {
                        term_erase_cursor(); /* Erase current block */
                        pos--;
                        kprint("\b \b");
                        term_draw_cursor(); /* Draw at new pos */
                    }
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

        char *arg = get_arg(cmd_buffer);

        if (str_cmp(cmd_buffer, "help") == 0) {
            cmd_help();
        } else if (str_cmp(cmd_buffer, "ls") == 0) {
            cmd_ls(arg);
        } else if (str_cmp(cmd_buffer, "pwd") == 0) {
            cmd_pwd();
        } else if (str_cmp(cmd_buffer, "cd") == 0) {
            cmd_cd(arg);
        } else if (str_cmp(cmd_buffer, "cc") == 0) {
            cmd_cc(arg);
        } else if (str_cmp(cmd_buffer, "testlibc") == 0) {
            libc_test_run();
        } else if (str_cmp(cmd_buffer, "cat") == 0) {
            if (arg) cmd_cat(arg);
            else kprint("\nUsage: cat <filename>\n\n");
        } else if (str_cmp(cmd_buffer, "touch") == 0) {
            cmd_touch(arg);
        } else if (str_cmp(cmd_buffer, "mkdir") == 0) {
            cmd_mkdir(arg);
        } else if (str_cmp(cmd_buffer, "cp") == 0) {
            cmd_cp(arg);
        } else if (str_cmp(cmd_buffer, "rm") == 0) {
            cmd_rm(arg);
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
        } else if (str_cmp(cmd_buffer, "mkfs") == 0) {
            cmd_mkfs();
        } else if (str_starts_with(cmd_buffer, "fdisk")) {
            char *arg = get_arg(cmd_buffer);
            cmd_fdisk(arg);
        } else if (str_cmp(cmd_buffer, "mount") == 0) {
            cmd_mount();
        } else if (str_cmp(cmd_buffer, "lsdisk") == 0) {
            cmd_lsdisk();
        } else if (str_cmp(cmd_buffer, "save") == 0) {
            cmd_save(arg);
        } else {
            kprint("Unknown command: ");
            kprint(cmd_buffer);
            kprint("\n\n");
        }
    }
}
