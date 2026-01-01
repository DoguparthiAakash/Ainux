#include "wm.h"
#include "drivers/mouse.h"
#include "drivers/keyboard.h"
#include "mm/heap.h"
#include "gfx.h"
#include "cursor_icon.h"
#include "sched/sched.h"

extern void kprint(const char *msg);
extern void term_clear(void);

static Window *windows[MAX_WINDOWS];
static int window_count = 0;
static uint32_t *backbuffer = NULL;
static uint32_t *frontbuffer_addr = NULL; 
static uint64_t screen_width = 0;
static uint64_t screen_height = 0;
static uint64_t screen_pitch = 0;

/* Helper: memcpy */
static void *mem_cpy(void *dest, const void *src, size_t n) {
    uint8_t *d = (uint8_t *)dest;
    const uint8_t *s = (const uint8_t *)src;
    while (n--) *d++ = *s++;
    return dest;
}

/* Helper: memset */
static void *mem_set(void *dest, int val, size_t n) {
    uint8_t *d = (uint8_t *)dest;
    while (n--) *d++ = (uint8_t)val;
    return dest;
}

void wm_init(void) {
    /* Cleanup existing state if restarting */
    if (backbuffer) {
        kfree(backbuffer);
        backbuffer = NULL;
    }
    
    for (int i = 0; i < window_count; i++) {
        if (windows[i]) {
            if (windows[i]->buffer) kfree(windows[i]->buffer);
            kfree(windows[i]);
        }
    }
    window_count = 0;

    /* Get FB Info */
    void *addr;
    gfx_get_info(&screen_width, &screen_height, &screen_pitch, &addr);
    frontbuffer_addr = (uint32_t *)addr;

    /* Allocate Backbuffer (32-bit color) */
    /* Pitch is in bytes, so height * pitch is total bytes */
    uint64_t buffer_size = screen_height * screen_pitch;
    backbuffer = (uint32_t *)kmalloc(buffer_size);
    
    if (!backbuffer) {
        kprint("[WM] Failed to allocate backbuffer!\n");
        return;
    }
    
    mem_set(backbuffer, 0, buffer_size); /* Clear to black */
    kprint("[WM] Initialized with Backbuffer\n");
}

Window* wm_create_window(int x, int y, int width, int height, const char *title) {
    if (window_count >= MAX_WINDOWS) return NULL;
    
    Window *win = (Window *)kmalloc(sizeof(Window));
    if (!win) return NULL;
    
    win->id = window_count;
    win->x = x;
    win->y = y;
    win->width = width;
    win->height = height;
    
    /* Copy title */
    int i = 0;
    while (title[i] && i < 31) {
        win->title[i] = title[i];
        i++;
    }
    win->title[i] = '\0';
    
    /* Allocate Window Buffer */
    win->buffer = (uint32_t *)kmalloc(width * height * sizeof(uint32_t));
    if (!win->buffer) {
        kfree(win);
        return NULL;
    }
    
    /* Fill with gray by default */
    for (int j = 0; j < width * height; j++) {
        win->buffer[j] = 0xC0C0C0; 
    }
    
    win->dragging = 0;
    
    windows[window_count++] = win;
    return win;
}

extern const uint8_t font_8x8[95][8];

static void wm_draw_char(Window *win, int x, int y, char c, uint32_t color) {
    if (!win || x < 0 || y < 0 || x+8 > win->width || y+8 > win->height) return;
    
    int idx = c - 32;
    if (idx < 0 || idx >= 95) idx = 0;
    
    const uint8_t *glyph = font_8x8[idx];
    
    for(int i=0; i<8; i++) {
        uint8_t row = glyph[i];
        for(int j=0; j<8; j++) {
            if (row & (1 << j)) {
                win->buffer[(y+i) * win->width + (x+j)] = color;
            }
        }
    }
}

/* Console Rendering */
#include "font.h" 

void wm_console_write(Window *win, const char *str) {
    if (!win || !win->buffer) return;
    
    /* Sanity bounds check for window */
    if (win->width > 4096 || win->height > 4096) return;
    
    // kprint calls this, so DO NOT CALL kprint HERE (Recursion)
    
    while (*str) {
        char c = *str++;
        
        if (c == '\n') {
            win->cursor_x = 0;
            win->cursor_y += 8; /* Font H=8 */
        } else if (c == '\r') {
             win->cursor_x = 0;
        } else if (c == '\b') {
             if (win->cursor_x >= 8) win->cursor_x -= 8;
        } else if (c >= 32) {
             /* Draw Char */
             // wm_draw_char(win, win->cursor_x, win->cursor_y, c, 0xFFFFFF);
             win->cursor_x += 8;
        }
        
        /* Wrap */
        if (win->cursor_x + 8 >= win->width) {
            win->cursor_x = 0;
            win->cursor_y += 8;
        }
        
        /* Scroll */
        if (win->cursor_y + 8 >= win->height) {
            /* Shift Buffer Up */
            int scroll_h = 8;
            int total_h = win->height;
            
            uint32_t *dst = win->buffer;
            uint32_t *src = win->buffer + (scroll_h * win->width);
            uint64_t count = (total_h - scroll_h) * win->width;
            
            for(uint64_t i=0; i<count; i++) dst[i] = src[i];
            
            /* Clear Bottom */
            uint32_t *bottom = win->buffer + ((total_h - scroll_h) * win->width);
            count = scroll_h * win->width;
            for(uint64_t i=0; i<count; i++) bottom[i] = 0x000000;
            
            win->cursor_y -= 8;
        }
    }
}

/* Internal: Draw a pixel to backbuffer */
static void wm_put_pixel_bb(int x, int y, uint32_t color) {
    if (x < 0 || x >= (int)screen_width || y < 0 || y >= (int)screen_height) return;
    /* Assume 32bpp packed */
    uint64_t offset = y * (screen_pitch / 4) + x;
    backbuffer[offset] = color;
}

/* Internal: Draw Rect to BB */
static void wm_fill_rect_bb(int x, int y, int w, int h, uint32_t color) {
    for (int j=0; j<h; j++) {
        for (int i=0; i<w; i++) {
            wm_put_pixel_bb(x+i, y+j, color);
        }
    }
}

/* Desktop State */
static DesktopIcon desktop_icons[16];
static int desktop_icon_count = 0;

void wm_add_icon(const char *label, int x, int y, void (*cb)(void)) {
    if (desktop_icon_count < 16) {
        DesktopIcon *ic = &desktop_icons[desktop_icon_count++];
        int i=0;
        while(label[i] && i<31) { ic->label[i] = label[i]; i++; }
        ic->label[i] = 0;
        ic->x = x;
        ic->y = y;
        ic->on_click = cb;
    }
}

void wm_clear_icons(void) {
    desktop_icon_count = 0;
}

/* Helper: Draw Text on BB */
/* Quick hack: use wm_draw_char logic but for BB. Need font. */
extern const uint8_t font_8x8[95][8];
static void wm_draw_char_bb(int x, int y, char c, uint32_t color) {
    int idx = c - 32;
    if (idx < 0 || idx >= 95) idx = 0;
    const uint8_t *glyph = font_8x8[idx];
    for(int i=0; i<8; i++) {
        uint8_t row = glyph[i];
        for(int j=0; j<8; j++) {
            if (row & (1 << j)) wm_put_pixel_bb(x+j, y+i, color);
        }
    }
}

static void wm_draw_string_bb(int x, int y, const char *str, uint32_t color) {
    while (*str) {
        wm_draw_char_bb(x, y, *str++, color);
        x += 8;
    }
}

static void wm_composite(MouseState *mouse) {
    if (!backbuffer) return;
    
    /* 1. Draw Premium Background (Gradient) */
    /* Dark Blue (0x000020) to Purple (0x200040) */
    for (int y = 0; y < (int)screen_height; y++) {
        /* Gradient Factor */
        int r = (y * 64) / screen_height;
        int b = 64 + (y * 64) / screen_height;
        uint32_t color = ((r & 0xFF) << 16) | (0 << 8) | (b & 0xFF);
        /* Simple dither/noise optional, stick to flat gradient */
        
        /* Optimization: Memset row if color is same? No, color changes per row. */
        /* But we can fill row loop */
        uint64_t row_offset = y * (screen_pitch / 4);
        for (int x = 0; x < (int)screen_width; x++) {
            backbuffer[row_offset + x] = color;
        }
    }
    
    /* 2. Draw Desktop Icons */
    for (int i=0; i<desktop_icon_count; i++) {
         DesktopIcon *ic = &desktop_icons[i];
         /* Draw Icon Box (Glassy) */
         wm_fill_rect_bb(ic->x, ic->y, ICON_SIZE, ICON_SIZE, 0x404060);
         /* Draw Selection Highlight if mouse over? */
         if (mouse->x >= ic->x && mouse->x < ic->x+ICON_SIZE &&
             mouse->y >= ic->y && mouse->y < ic->y+ICON_SIZE) {
             wm_fill_rect_bb(ic->x, ic->y, ICON_SIZE, ICON_SIZE, 0x606080);     
         }
         
         /* Draw Label */
         wm_draw_string_bb(ic->x + 2, ic->y + ICON_SIZE + 4, ic->label, 0xFFFFFF);
         
         /* "Icon" Graphic (Simple Initials) */
         wm_draw_char_bb(ic->x + 20, ic->y + 20, ic->label[0], 0xFFFFFF);
    }
    
    /* 3. Draw Windows */
    for (int i = 0; i < window_count; i++) {
        Window *win = windows[i];
        
        /* Draw Border/Title Bar */
        /* Border + Title Height */
        int outer_x = win->x - 2;
        int outer_y = win->y - TITLE_BAR_HEIGHT - 2;
        int outer_w = win->width + 4;
        int outer_h = win->height + TITLE_BAR_HEIGHT + 4;
        
        wm_fill_rect_bb(outer_x, outer_y, outer_w, outer_h, WINDOW_BORDER_COLOR);
        
        /* Title Bar */
        wm_fill_rect_bb(win->x, win->y - TITLE_BAR_HEIGHT, win->width, TITLE_BAR_HEIGHT, WINDOW_TITLE_BG);
        
        /* Window Content */
        for (int row = 0; row < win->height; row++) {
            for (int col = 0; col < win->width; col++) {
                int screen_x = win->x + col;
                int screen_y = win->y + row;
                uint32_t color = win->buffer[row * win->width + col];
                wm_put_pixel_bb(screen_x, screen_y, color);
            }
        }
    }
    
    /* 3. Draw Mouse Cursor (Bitmap) */
    int mx = mouse->x - CURSOR_X_HOT;
    int my = mouse->y - CURSOR_Y_HOT;
    
    for (int r = 0; r < CURSOR_HEIGHT; r++) {
        for (int c = 0; c < CURSOR_WIDTH; c++) {
            int screen_x = mx + c;
            int screen_y = my + r;
            
            if (screen_x < 0 || screen_x >= (int)screen_width || 
                screen_y < 0 || screen_y >= (int)screen_height) continue;
            
            uint32_t src_pixel = cursor_icon[r * CURSOR_WIDTH + c];
            uint32_t alpha = (src_pixel >> 24) & 0xFF;
            
            if (alpha == 0) continue; /* Fully transparent */
            
            if (alpha == 255) {
                wm_put_pixel_bb(screen_x, screen_y, src_pixel & 0xFFFFFF);
            } else {
                /* Alpha Blend */
                uint64_t bg_offset = screen_y * (screen_pitch / 4) + screen_x;
                uint32_t bg_pixel = backbuffer[bg_offset];
                
                uint32_t rb = bg_pixel & 0xFF00FF;
                uint32_t g  = bg_pixel & 0x00FF00;
                
                uint32_t s_rb = src_pixel & 0xFF00FF;
                uint32_t s_g  = src_pixel & 0x00FF00;
                
                uint32_t d_rb = s_rb * alpha + rb * (255 - alpha);
                uint32_t d_g  = s_g  * alpha + g  * (255 - alpha);
                
                d_rb = (d_rb >> 8) & 0xFF00FF;
                d_g  = (d_g  >> 8) & 0x00FF00;
                
                wm_put_pixel_bb(screen_x, screen_y, d_rb | d_g);
            }
        }
    }
    
    /* 4. Flip Buffer (Copy Back -> Front) */
    mem_cpy(frontbuffer_addr, backbuffer, screen_height * screen_pitch);
}

void wm_draw_window_content(Window *win, int x, int y, uint32_t color) {
    if (!win) return;
    if (x < 0 || x >= win->width || y < 0 || y >= win->height) return;
    win->buffer[y * win->width + x] = color;
}

void wm_fill_rect(Window *win, int x, int y, int w, int h, uint32_t color) {
    if (!win) return;
    /* Clip to window bounds */
    if (x < 0) { w += x; x = 0; }
    if (y < 0) { h += y; y = 0; }
    if (x + w > win->width) w = win->width - x;
    if (y + h > win->height) h = win->height - y;
    
    if (w <= 0 || h <= 0) return;
    
    for (int row = 0; row < h; row++) {
        for (int col = 0; col < w; col++) {
             win->buffer[(y + row) * win->width + (x + col)] = color;
        }
    }
}

void wm_run(void) {
    int running = 1;
    
    int blink_cycles = 0;
    
    while (running) {
        /* Process Input */
        
        /* Keyboard: Exit on 'ESC' */
        if (keyboard_available()) {
            char c = keyboard_getchar();
            if (c == 27) { /* ESC */
                running = 0;
            }
        }
        
        /* Mouse */
        MouseState mouse = mouse_get_state();
        
        /* Window Dragging Logic */
        static int drag_win_id = -1;
        
        if (mouse.left_btn) {
            if (drag_win_id == -1) {
                /* Try to pick up a window (Front to Back for clicking) */
                for (int i = window_count - 1; i >= 0; i--) {
                    Window *win = windows[i];
                    /* Hit test title bar */
                    if (mouse.x >= win->x && mouse.x < win->x + win->width &&
                        mouse.y >= win->y - TITLE_BAR_HEIGHT && mouse.y < win->y) {
                            
                        drag_win_id = i;
                        win->data_offset_x = mouse.x - win->x;
                        win->data_offset_y = mouse.y - win->y;
                        
                        /* Move to top (simple swap with last) */
                        if (i != window_count - 1) {
                            Window *temp = windows[window_count - 1];
                            windows[window_count - 1] = win;
                            windows[i] = temp;
                            drag_win_id = window_count - 1;
                        }
                        break;
                    }
                }
            } else {
                /* Dragging */
                Window *win = windows[drag_win_id];
                win->x = mouse.x - win->data_offset_x;
                win->y = mouse.y - win->data_offset_y;
            }
        } else {
            drag_win_id = -1;
        }
        
        /* Composite & Render */
        wm_composite(&mouse);
        
        /* Yield to allow background tasks to run */
        sched_yield();
    }
    
    /* Cleanup */
    term_clear();
}
