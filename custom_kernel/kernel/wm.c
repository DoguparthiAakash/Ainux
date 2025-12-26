#include "wm.h"
#include "drivers/mouse.h"
#include "drivers/keyboard.h"
#include "mm/heap.h"
#include "gfx.h"
#include "cursor_icon.h"

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

static void wm_composite(MouseState *mouse) {
    if (!backbuffer) return;
    
    /* 1. Clear Background (Teal) */
    uint64_t total_pixels = screen_height * (screen_pitch / 4);
    for (uint64_t i = 0; i < total_pixels; i++) {
        backbuffer[i] = 0x008080; /* Teal background */
    }
    
    /* 2. Draw Windows */
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
        
        /* Yield / Delay */
        /* Currently single threaded, busy loop implies 100% CPU usage */
        /* Add small delay? */
        /* for(volatile int i=0; i<1000; i++); */ 
    }
    
    /* Cleanup */
    term_clear();
}
