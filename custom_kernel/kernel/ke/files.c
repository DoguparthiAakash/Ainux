#include "shell.h"
#include "../drivers/gfx/wm.h"
#include "../drivers/gfx/gfx.h"
#include "../drivers/mouse.h"
#include "../drivers/keyboard.h"
#include "../libc/string.h"
#include "../libc/stdio.h"
#include "../log.h"
#include "../ex/io/vfs.h"
#include "../ex/mm/heap.h"

/* State */
Window *fm_win = NULL;
static char current_path[256];
static int selected_index = -1;

#define FM_ICON_SIZE 40
#define PADDING 20
#define COLS 6

/* File Entry Cache */
typedef struct {
    char name[64];
    int is_dir;
    int size;
} file_entry_t;

static file_entry_t files[64];
static int file_count = 0;

/* Helper to get node from path */
/* Exposed from vfs? No, internal. We walk from fs_root */
static vfs_node_t *get_node_from_path(const char *path) {
    if (strcmp(path, "/") == 0) return fs_root;
    
    /* Naive: only support 1 level for now or specific mounts */
    /* TODO: Full path parser */
    
    /* Temp: If path is /initrd, return initrd mount */
    /* fs_root should be FAT32. check its children? */
    /* vfs_finddir expects name. */
    
    /* Let's try to walk the path tokens */
    vfs_node_t *curr = fs_root;
    char temp[256];
    strcpy(temp, path);
    
    char *p = temp;
    if (*p == '/') p++; /* Skip leading / */
    
    char *token = strtok(p, "/");
    while (token) {
        vfs_node_t *next = vfs_finddir(curr, token);
        if (!next) return NULL;
        curr = next;
        token = strtok(NULL, "/");
    }
    return curr;
}

static void load_files(const char *path) {
    file_count = 0;
    vfs_node_t *dir = get_node_from_path(path);
    if (!dir) return;
    
    /* vfs_readdir protocol: returns struct w_dirent*, takes index */
    struct w_dirent *entry;
    int i = 0;
    while ((entry = vfs_readdir(dir, i)) != NULL) {
        if (file_count < 64) {
            strcpy(files[file_count].name, entry->name);
            /* Type check? w_dirent has no type? */
            /* vfs_finddir to get node and check flags? Expensive? */
            /* Checking dirent name conventions or separate stat? */
            /* Let's lookup to check directory flag */
            vfs_node_t *node = vfs_finddir(dir, entry->name);
            files[file_count].is_dir = (node && (node->flags & VFS_DIRECTORY));
            
            file_count++;
        }
        i++;
    }
}

/* Need Font Helper */
#include "../drivers/gfx/font.h"
static void fm_draw_char(Window *win, char c, int x, int y, uint32_t color) {
    if (c < 32) return;
    int idx = c - 32;
    const uint8_t *glyph = font_8x8[idx];
    for(int dy=0; dy<8; dy++) {
        for(int dx=0; dx<8; dx++) {
            if (glyph[dy] & (1<<dx)) {
                wm_draw_window_content(win, x+dx, y+dy, color);
            }
        }
    }
}

static void fm_draw_string(Window *win, char *s, int x, int y, uint32_t color) {
    while(*s) {
        fm_draw_char(win, *s++, x, y, color);
        x += 8;
    }
}

static void draw_icon(int index, int x, int y) {
    if (index >= file_count) return;
    
    file_entry_t *f = &files[index];
    
    /* Icon Rect */
    uint32_t color = f->is_dir ? 0xFFA500 : 0xFFFFFF; // Orange vs White
    
    /* Draw Box */
    for(int dy=0; dy<FM_ICON_SIZE; dy++) {
        for(int dx=0; dx<FM_ICON_SIZE; dx++) {
             wm_draw_window_content(fm_win, x+dx, y+dy, color);
        }
    }
}

static void fman_paint(void) {
    /* Clear Background */
    /* WM handles window abstraction, we just draw content */
    /* Fill Grey */
    for(int y=0; y<fm_win->height; y++) {
        for(int x=0; x<fm_win->width; x++) {
             wm_draw_window_content(fm_win, x, y, 0x303030);
        }
    }
    
    /* Draw Path */
    fm_draw_string(fm_win, current_path, 10, 5, 0xFFFFFF);
    
    /* Draw Icons */
    int start_y = 30;
    for(int i=0; i<file_count; i++) {
        int col = i % COLS;
        int row = i / COLS;
        
        int x = PADDING + col * (FM_ICON_SIZE + PADDING);
        int y = start_y + row * (FM_ICON_SIZE + PADDING + 20); // 20 for label
        
        draw_icon(i, x, y);
        fm_draw_string(fm_win, files[i].name, x, y + FM_ICON_SIZE + 2, 0xCCCCCC);
    }
    
    /* wm_update(fm_win); - WM usually double buffered? Loop composites */
}

static void fman_click(int mx, int my) {
    /* Check collision with icons */
     int start_y = 30;
    for(int i=0; i<file_count; i++) {
        int col = i % COLS;
        int row = i / COLS;
        
        int x = PADDING + col * (FM_ICON_SIZE + PADDING);
        int y = start_y + row * (FM_ICON_SIZE + PADDING + 20);
        
        if (mx >= x && mx <= x + FM_ICON_SIZE && my >= y && my <= y + FM_ICON_SIZE) {
            /* Clicked! */
            file_entry_t *f = &files[i];
            if (f->is_dir) {
                /* Change Dir */
                /* Basic concatenation logic */
                char new_path[256];
                strcpy(new_path, current_path);
                if (new_path[strlen(new_path)-1] != '/') strcat(new_path, "/");
                strcat(new_path, f->name);
                
                strcpy(current_path, new_path);
                load_files(current_path);
                fman_paint(); /* Repaint immediately */
                
            } else {
                /* Run / Open */
                kprint("[FM] Clicked File: "); kprint(f->name); kprint("\n");
            }
            return;
        }
    }
}

void cmd_files(char *arg) {
    (void)arg;
    strcpy(current_path, "/");
    load_files(current_path);
    
    fm_win = wm_create_window(100, 100, 400, 300, "File Manager");
    if (!fm_win) return;
    
    fman_paint();
    
    /* Loop */
    while(1) {
        /* Poll Mouse */
        MouseState m = mouse_get_state();
        
        /* Check Exit (Esc) */
        if (keyboard_available()) {
            char c = keyboard_getchar();
            if (c == 27) break; /* Esc */
        }
        
        /* Check WM events (Click) */
        if (m.left_btn) {
             /* Global Mouse to Window Local? */
             /* Very crude */
             int wx = m.x - fm_win->x;
             int wy = m.y - fm_win->y;
             if (wx >= 0 && wx < fm_win->width && wy >= 0 && wy < fm_win->height) { // Inside
                 fman_click(wx, wy);
                 /* Debounce */
                 for(volatile int k=0; k<5000000; k++);
             }
        }
        
        /* Render GUI */
        kprint("[Files] Compositing...\n");
        wm_composite(&m);
        kprint("[Files] Done.\n");
        
        /* Yield is crucial? Shell task? */
        /* sched_yield(); usually implies we give up time. */
        /* If we loop tight, we freeze system? No, loop has polls. */
        /* But we should yield to allow WM composite! */
        /* Important: Add yield or sleep */
        for(volatile int k=0; k<10000000; k++); // Large delay (slow FPS)
    }
    
    wm_destroy_window(fm_win);
    /* Clear screen on exit to restore text console look */
    /* Access term_clear? it's in init.c or wm.c */
    /* wm.c has extern term_clear or implements it? */
    /* wm_run() calls term_clear() at end. */
    /* Let's try to find it. wm.c implies it's extern. */
    /* It is in init.c. Not exposed in header? */
    /* wm.h doesn't have it. */
    /* Just clearing icon/window state is enough, console will overwrite. */
}
