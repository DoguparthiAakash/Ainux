#include "wm.h"
#include <libc/string.h>
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

/* Cached Background Buffer */
static uint32_t *bg_buffer = NULL;

/* Dirty Logic */
static int wm_dirty = 1;
void wm_mark_dirty(void) { wm_dirty = 1; }

void wm_enable_acceleration(void) {
    if (!frontbuffer_addr) return;
    extern void mtrr_set_wc(uint64_t base, uint64_t size);
    mtrr_set_wc((uint64_t)frontbuffer_addr, screen_height * screen_pitch);
}

/* Optimized Helpers */
static void *mem_cpy_opt(void *dest, const void *src, size_t n) {
    /* Use rep movsq for 8-byte blocks */
    size_t qwords = n / 8;
    size_t remainder = n % 8;
    
    /* Inline ASM for Speed */
    __asm__ volatile (
        "rep movsq\n"
        : "+D"(dest), "+S"(src), "+c"(qwords)
        : 
        : "memory"
    );
    
    /* Handle remainder */
    if (remainder) {
        __asm__ volatile (
            "rep movsb\n"
            : "+D"(dest), "+S"(src), "+c"(remainder)
            : 
            : "memory"
        );
    }
    
    return dest;
}

static void *mem_set_opt(void *dest, int val, size_t n) {
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
    gfx_get_info(&screen_width, &screen_height, &screen_pitch, &addr);
    frontbuffer_addr = (uint32_t *)addr;
    
    /* Enable Hardware Acceleration (Write Combining) */
    /* Disabled by default to prevent VirtualBox GPF. Enable via 'mtrr on' cmd. */
    /* if (addr) mtrr_set_wc((uint64_t)addr, screen_height * screen_pitch); */

    /* Allocate Backbuffer (32-bit color) */
    /* Pitch is in bytes, so height * pitch is total bytes */
    uint64_t buffer_size = screen_height * screen_pitch;
    backbuffer = (uint32_t *)kmalloc(buffer_size);
    
    if (!backbuffer) {
        kprint("[WM] Failed to allocate backbuffer!\n");
        return;
    }
    
    if (bg_buffer) { kfree(bg_buffer); bg_buffer = NULL; }

    mem_set_opt(backbuffer, 0, buffer_size); /* Clear to black */
    
    /* Allocate BG cache */
    bg_buffer = (uint32_t *)kmalloc(buffer_size);
    if (!bg_buffer) { kprint("[WM] Failed BG alloc\n"); return; }
    
    /* Pre-render Gradient */
    for (int y = 0; y < (int)screen_height; y++) {
        int r = (y * 64) / screen_height;
        int b = 64 + (y * 64) / screen_height;
        uint32_t color = ((r & 0xFF) << 16) | (0 << 8) | (b & 0xFF);
        
        uint64_t row_offset = y * (screen_pitch / 4);
        for (int x = 0; x < (int)screen_width; x++) {
            bg_buffer[row_offset + x] = color;
        }
    }
    
    kprint("[WM] Initialized with Backbuffer + Cache\n");
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

void wm_destroy_window(Window *win) {
    if (!win) return;
    
    int index = -1;
    for (int i = 0; i < window_count; i++) {
        if (windows[i] == win) {
            index = i;
            break;
        }
    }
    
    if (index != -1) {
        if (win->buffer) kfree(win->buffer);
        kfree(win);
        
        /* Shift */
        for (int i = index; i < window_count - 1; i++) {
            windows[i] = windows[i+1];
        }
        window_count--;
        windows[window_count] = NULL;
    }
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


/* Moved to Global for Login Screen Access */
static int prev_mx = -1, prev_my = -1;
static int prev_btn = -1;

void wm_composite(void *ctx) {
    MouseState *mouse = (MouseState*)ctx;
    
    /* Check for changes */
    /* Disabled Dirty Check for Debugging */
    /* if (!wm_dirty && mouse->x == prev_mx && mouse->y == prev_my && mouse->left_btn == prev_btn) {
        return; 
    } */
    
    if (!backbuffer) return;
    
    /* 1. Fast Background Copy or Fallback */
    if (bg_buffer) {
        mem_cpy_opt(backbuffer, bg_buffer, screen_height * screen_pitch);
    } else {
        /* Fallback: Recalculate Gradient */
        for (int y = 0; y < (int)screen_height; y++) {
            int r = (y * 64) / screen_height;
            int b = 64 + (y * 64) / screen_height;
            uint32_t color = ((r & 0xFF) << 16) | (0 << 8) | (b & 0xFF);
            uint64_t row_offset = y * (screen_pitch / 4);
            for (int x = 0; x < (int)screen_width; x++) {
                backbuffer[row_offset + x] = color;
            }
        }
    }
    
    /* 2. Desktop Icons */
    for (int i=0; i<desktop_icon_count; i++) {
         DesktopIcon *ic = &desktop_icons[i];
         /* Simple logic: assume we need to redraw them. Optimization: pre-render icons to bg_buffer? No, dynamic selection */
         wm_fill_rect_bb(ic->x, ic->y, ICON_SIZE, ICON_SIZE, 0x404060);
         if (mouse->x >= ic->x && mouse->x < ic->x+ICON_SIZE &&
             mouse->y >= ic->y && mouse->y < ic->y+ICON_SIZE) {
             wm_fill_rect_bb(ic->x, ic->y, ICON_SIZE, ICON_SIZE, 0x606080);     
         }
         wm_draw_string_bb(ic->x + 2, ic->y + ICON_SIZE + 4, ic->label, 0xFFFFFF);
         wm_draw_char_bb(ic->x + 20, ic->y + 20, ic->label[0], 0xFFFFFF);
    }

    /* 3. Draw Windows */
    for (int i = 0; i < window_count; i++) {
        Window *win = windows[i];
        
        /* Drop Shadow Optimization: Simple Darkening is expensive per pixel. Disable for "Speed"? 
           Or optimize loop. Let's keep it but optimize memory access if possible */
           
        /* Draw Border/Title */
        int outer_x = win->x - 2;
        int outer_y = win->y - TITLE_BAR_HEIGHT - 2;
        int outer_w = win->width + 4;
        int outer_h = win->height + TITLE_BAR_HEIGHT + 4;
        
        wm_fill_rect_bb(outer_x, outer_y, outer_w, outer_h, 0x202020);
        wm_fill_rect_bb(win->x, win->y - TITLE_BAR_HEIGHT, win->width, TITLE_BAR_HEIGHT, 0x404040);
        wm_draw_string_bb(win->x + 8, win->y - TITLE_BAR_HEIGHT + 6, win->title, 0xFFFFFF);
        wm_draw_string_bb(win->x + win->width - 16, win->y - TITLE_BAR_HEIGHT + 6, "X", 0xFF5555);

        /* Content Blit Optimization: Row-wise Copy instead of put_pixel */
        /* Only valid if screen_pitch/4 matches window width? No. */
        /* Use memcpy per row */
        for (int row = 0; row < win->height; row++) {
            int screen_y = win->y + row;
            if (screen_y < 0 || screen_y >= (int)screen_height) continue;
            
            /* Bounds X */
            int win_start_col = 0;
            int screen_x_start = win->x;
            int copy_w = win->width;
            
            if (screen_x_start < 0) { win_start_col = -screen_x_start; copy_w += screen_x_start; screen_x_start = 0; }
            if (screen_x_start + copy_w > (int)screen_width) copy_w = (int)screen_width - screen_x_start;
            
            if (copy_w > 0) {
                 uint32_t *dest_row = backbuffer + (screen_y * (screen_pitch/4) + screen_x_start);
                 uint32_t *src_row = win->buffer + (row * win->width + win_start_col);
                 /* memcpy (bytes) */
                 mem_cpy_opt(dest_row, src_row, copy_w * 4);
            }
        }
    }
    
    /* 4. Mouse (Custom Icon) */
    int mx = mouse->x - CURSOR_X_HOT;
    int my = mouse->y - CURSOR_Y_HOT;
    
    for (int r = 0; r < CURSOR_HEIGHT; r++) {
        for (int c = 0; c < CURSOR_WIDTH; c++) {
            int screen_x = mx + c;
            int screen_y = my + r;
            
            /* Bounds check */
            if (screen_x < 0 || screen_x >= (int)screen_width || 
                screen_y < 0 || screen_y >= (int)screen_height) continue;
            
            uint32_t src_pixel = cursor_icon[r * CURSOR_WIDTH + c];
            uint32_t alpha = (src_pixel >> 24) & 0xFF; // ARGB
            
            if (alpha == 0) continue; // Fully transparent
            
            if (alpha == 255) {
                wm_put_pixel_bb(screen_x, screen_y, src_pixel & 0xFFFFFF);
            } else {
                /* Alpha Blend */
                uint64_t off = screen_y * (screen_pitch / 4) + screen_x;
                uint32_t dest = backbuffer[off];
                
                uint32_t s_r = (src_pixel >> 16) & 0xFF;
                uint32_t s_g = (src_pixel >> 8) & 0xFF;
                uint32_t s_b = (src_pixel) & 0xFF;
                
                uint32_t d_r = (dest >> 16) & 0xFF;
                uint32_t d_g = (dest >> 8) & 0xFF;
                uint32_t d_b = (dest) & 0xFF;
                
                uint32_t r_out = (s_r * alpha + d_r * (255 - alpha)) / 255;
                uint32_t g_out = (s_g * alpha + d_g * (255 - alpha)) / 255;
                uint32_t b_out = (s_b * alpha + d_b * (255 - alpha)) / 255;
                
                backbuffer[off] = (r_out << 16) | (g_out << 8) | b_out;
            }
        }
    }
    
    /* 5. Flip */
    mem_cpy_opt(frontbuffer_addr, backbuffer, screen_height * screen_pitch);
    
    prev_mx = mouse->x; prev_my = mouse->y; prev_btn = mouse->left_btn;
    wm_dirty = 0;
}

/* --- GUI Login System --- */
#include "ke/user.h" /* Assume include path is correct or adjust Makefile include */
/* Actually user.h functions are linked, we just need prototypes if header fails */
extern int user_check(const char *username, const char *password);
extern int user_recover_check(const char *email, const char *mobile, char *out_user);

/* Simple Text Input Widget Helper */
static void draw_input_box(int x, int y, int w, int h, const char *label, const char *value, int active, int masked) {
    /* Label */
    wm_draw_string_bb(x, y - 20, label, 0xCCCCCC);
    
    /* Box */
    uint32_t bg = active ? 0x303040 : 0x202020;
    uint32_t border = active ? 0x00AAFF : 0x555555;
    
    wm_fill_rect_bb(x, y, w, h, bg);
    
    /* Border (Top/Bot/Left/Right) */
    wm_fill_rect_bb(x, y, w, 1, border);
    wm_fill_rect_bb(x, y+h-1, w, 1, border);
    wm_fill_rect_bb(x, y, 1, h, border);
    wm_fill_rect_bb(x+w-1, y, 1, h, border);
    
    /* Text */
    char display_text[64];
    if (masked) {
        int len = 0; while(value[len] && len < 63) { display_text[len] = '*'; len++; }
        display_text[len] = 0;
    } else {
        strcpy(display_text, value);
    }
    
    wm_draw_string_bb(x + 5, y + 8, display_text, 0xFFFFFF);
    
    /* Cursor if active */
    if (active) {
        int len = 0; while(value[len]) len++;
        int cx = x + 5 + len * 8; /* Approx 8px per char */
        if(((mouse_get_state().x) >> 4) & 1) /* Blink via mouse/timer hack? Use dirty frame count */ 
             wm_fill_rect_bb(cx, y+5, 2, h-10, 0xFFFFFF);
    }
}

int wm_login_screen(void) {
    char user_buf[64] = "";
    char pass_buf[64] = "";
    char recover_buf[64] = ""; /* For Email/Mobile input */
    
    int focus = 0; /* 0=User, 1=Pass, 2=LoginBtn, 3=ForgotBtn, 4=RecoverInput, 5=SubmitRecover */
    int mode = 0;  /* 0=Login, 1=Recovery */
    
    int win_w = 400;
    int win_h = 300;
    int win_x = (screen_width - win_w) / 2;
    int win_y = (screen_height - win_h) / 2;
    
    while(1) {
        /* Input Handling Strategy: 
           Since we are in a loop, we need to poll keyboard. 
           But keyboard driver is IRQ driven or Polled. 
           We need a non-blocking `keyboard_get_last_char` or similar.
           Assuming `keyboard_getchar` is blocking, we check `keyboard_available`.
        */
        
        if (keyboard_available()) {
            char c = keyboard_getchar();
            
            if (c == '\t') { /* Tab */
                if (mode == 0) focus = (focus + 1) % 4;
                else focus = 4 + ((focus - 3) % 2); /* 4->5->4... */
            } else if (c == '\n') { /* Enter */
                if (mode == 0) {
                     if (focus == 0 && user_buf[0]) {
                         focus = 1; /* Switch to Password */
                     } else if (focus == 1 || (focus == 0 && !user_buf[0])) { /* Attempt Login */
                         kprint("[Login] Attempting with User='"); kprint(user_buf); kprint("' Pass='"); kprint(pass_buf); kprint("'\n");
                         
                         if (user_check(user_buf, pass_buf)) {
                             kprint("[Login] Success!\n");
                             user_set_current(user_buf); /* Set Session */
                             extern void user_set_authenticated(int status);
                             user_set_authenticated(1);
                             return 1; /* Success */
                         }
                         kprint("[Login] Failed.\n");
                         /* Else Fail Animation/Mark? Clear pass */
                         pass_buf[0] = 0;
                         if (!user_buf[0]) focus = 0; /* Refocus user if empty */
                     }
                } else {
                     /* Recovery Logic */
                     char found_user[64];
                     if (user_recover_check(recover_buf, recover_buf, found_user)) {
                          /* Found! Simulate OTP */
                          term_clear(); /* Force text mode for critical info? No, overlay. */
                          /* Just auto-fill user and reset pass? */
                          strcpy(user_buf, found_user);
                          mode = 0; focus = 1; /* Go to password */
                          /* Print valid OTP to serial/log for "Simulation" */
                          kprint("\n[RECOVERY] OTP for "); kprint(found_user); kprint(": 1234\n");
                     } else {
                          recover_buf[0] = 0; /* Fail */
                     }
                }
            } else if (c == '\b') {
                char *target = NULL;
                if (mode == 0) {
                    if (focus == 0) target = user_buf;
                    if (focus == 1) target = pass_buf;
                } else {
                    if (focus == 4) target = recover_buf;
                }
                
                if (target) {
                    int len = 0; while(target[len]) len++;
                    if (len > 0) target[len-1] = 0;
                }
            } else if (c >= 32 && c <= 126) {
                char *target = NULL;
                if (mode == 0) {
                    if (focus == 0) target = user_buf;
                    if (focus == 1) target = pass_buf;
                } else {
                    if (focus == 4) target = recover_buf;
                }
                
                if (target) {
                    int len = 0; while(target[len]) len++;
                    if (len < 63) { target[len] = c; target[len+1] = 0; }
                }
            }
            wm_mark_dirty();
        }
        
        /* Render Loop (Reuse wm_composite logic or custom minimal) */
        /* To utilize full double buffering, we assume wm_composite calls us or we takeover?
           Ideally 'wm_run' calls 'wm_login_screen' which replaces the "Desktop" phase.
           
           For this implementation, let's just Draw to Backbuffer and Flip manually 
           since we are blocking execution.
        */
        
        /* Clear Screen (Gradient - Deep MacOS Big Sur style) */
        for (int y = 0; y < (int)screen_height; y++) {
             /* Gradient: Top #e55d87 -> Bot #5fc3e4 (Pink/Blue) or just Blue */
             /* Let's do a smooth Purple-Blue: #2E3192 to #1BFFFF */
             /* Interpolate R, G, B */
             int factor = (y * 255) / screen_height;
             
             /* Start Color: 0x2E3192 (46, 49, 146) */
             /* End Color:   0x1BFFFF (27, 255, 255) */
             
             int r = (46 * (255 - factor) + 27 * factor) / 255;
             int g = (49 * (255 - factor) + 255 * factor) / 255;
             int b = (146 * (255 - factor) + 255 * factor) / 255;
             
             uint32_t color = (r << 16) | (g << 8) | b;
             
             /* Optimization: Set row */
             /* Cannot use memset because it's 32-bit color. Use loop. */
             uint32_t *row = backbuffer + y * (screen_pitch/4);
             for(int x=0; x < (int)screen_width; x++) row[x] = color;
        }
        
        /* Draw Window - "Glass" Panel (Rounded Corners simulated by not drawing corners) */
        int radius = 10;
        uint32_t panel_color = 0x88FFFFFF; /* White with transparency simulation? No, just solid light grey for now. */
        /* Actually "Glass" involves alpha blending with BG. Too slow. Use Solid Light Grey/White. */
        panel_color = 0xF0F0F0;
        
        /* Draw Rounded Rect Manual */
        for(int y=0; y<win_h; y++) {
            for(int x=0; x<win_w; x++) {
                 /* Apply simple corner masking */
                 if ((x < radius && y < radius && (radius-x)*(radius-x)+(radius-y)*(radius-y) > radius*radius) || // TL
                     (x > win_w-radius && y < radius && (x-(win_w-radius))*(x-(win_w-radius))+(radius-y)*(radius-y) > radius*radius) || // TR
                     (x < radius && y > win_h-radius && (radius-x)*(radius-x)+(y-(win_h-radius))*(y-(win_h-radius)) > radius*radius) || // BL
                     (x > win_w-radius && y > win_h-radius && (x-(win_w-radius))*(x-(win_w-radius))+(y-(win_h-radius))*(y-(win_h-radius)) > radius*radius) // BR
                    ) {
                     continue; /* Skip corner pixel */
                 }
                 wm_put_pixel_bb(win_x + x, win_y + y, panel_color);
            }
        }
        
        /* Avatar Circle (Placeholder) */
        int av_r = 30;
        int av_cx = win_x + win_w/2;
        int av_cy = win_y + 50;
        for(int y=-av_r; y<=av_r; y++) {
            for(int x=-av_r; x<=av_r; x++) {
                if (x*x + y*y <= av_r*av_r) {
                    wm_put_pixel_bb(av_cx + x, av_cy + y, 0xCCCCCC);
                }
            }
        }
        
        if (mode == 0) {
             /* Centered Text */
             /* Font width 8px */
             char *title = "Mithl OS";
             int t_w = strlen(title) * 8;
             wm_draw_string_bb(win_x + (win_w - t_w)/2, av_cy + 40, title, 0x333333);
             
             draw_input_box(win_x + 50, win_y + 110, 300, 30, "Username", user_buf, focus==0, 0);
             draw_input_box(win_x + 50, win_y + 160, 300, 30, "Password", pass_buf, focus==1, 1);
             
             /* Action Buttons */
             uint32_t login_bg = (focus == 2) ? 0x00CC00 : 0x008800;
             wm_fill_rect_bb(win_x + 40, win_y + 200, 150, 40, login_bg);
             wm_draw_string_bb(win_x + 90, win_y + 212, "Login", 0xFFFFFF);
             
             uint32_t forgot_bg = (focus == 3) ? 0xAA4400 : 0x882200;
             wm_fill_rect_bb(win_x + 210, win_y + 200, 150, 40, forgot_bg);
             wm_draw_string_bb(win_x + 240, win_y + 212, "Forgot?", 0xFFFFFF);
        } else {
             wm_draw_string_bb(win_x + 20, win_y + 20, "Recover Account", 0xFFFFFF);
             wm_draw_string_bb(win_x + 20, win_y + 50, "Enter Email or Mobile:", 0xAAAAAA);
             
             draw_input_box(win_x + 40, win_y + 100, 320, 30, "Recovery Contact", recover_buf, focus==4, 0);
             
             uint32_t sub_bg = (focus == 5) ? 0x0088CC : 0x0066AA;
             wm_fill_rect_bb(win_x + 40, win_y + 160, 320, 40, sub_bg);
             wm_draw_string_bb(win_x + 160, win_y + 172, "Send OTP", 0xFFFFFF);
             
             wm_draw_string_bb(win_x + 40, win_y + 250, "Press [Tab] to switch, [Enter] to submit.", 0x888888);
        }
        
        /* Mouse Cursor (High Quality) */
        MouseState ms = mouse_get_state();
        int mx = ms.x - CURSOR_X_HOT;
        int my = ms.y - CURSOR_Y_HOT;
        
        for (int r = 0; r < CURSOR_HEIGHT; r++) {
            for (int c = 0; c < CURSOR_WIDTH; c++) {
                int screen_x = mx + c;
                int screen_y = my + r;
                
                if (screen_x < 0 || screen_x >= (int)screen_width || 
                    screen_y < 0 || screen_y >= (int)screen_height) continue;
                
                uint32_t src_pixel = cursor_icon[r * CURSOR_WIDTH + c];
                uint32_t alpha = (src_pixel >> 24) & 0xFF; // ARGB
                
                if (alpha == 0) continue; 
                
                if (alpha == 255) {
                    wm_put_pixel_bb(screen_x, screen_y, src_pixel & 0xFFFFFF);
                } else {
                    /* Alpha Blend */
                    uint64_t off = screen_y * (screen_pitch / 4) + screen_x;
                    uint32_t dest = backbuffer[off];
                    
                    uint32_t s_r = (src_pixel >> 16) & 0xFF;
                    uint32_t s_g = (src_pixel >> 8) & 0xFF;
                    uint32_t s_b = (src_pixel) & 0xFF;
                    
                    uint32_t d_r = (dest >> 16) & 0xFF;
                    uint32_t d_g = (dest >> 8) & 0xFF;
                    uint32_t d_b = (dest) & 0xFF;
                    
                    uint32_t r_out = (s_r * alpha + d_r * (255 - alpha)) / 255;
                    uint32_t g_out = (s_g * alpha + d_g * (255 - alpha)) / 255;
                    uint32_t b_out = (s_b * alpha + d_b * (255 - alpha)) / 255;
                    
                    backbuffer[off] = (r_out << 16) | (g_out << 8) | b_out;
                }
            }
        }
        
        /* Flip */
        mem_cpy_opt(frontbuffer_addr, backbuffer, screen_height * screen_pitch);
        
        /* Check Click */
        if (ms.left_btn && !prev_btn) {
             /* Check Buttons Hit Test */
             if (mode == 0) {
                 /* Check Login Button */
                 if (ms.x >= win_x+40 && ms.x < win_x+190 && ms.y >= win_y+200 && ms.y < win_y+240) {
                      if (user_check(user_buf, pass_buf)) return 1;
                      pass_buf[0] = 0;
                 }
                 /* Check Forgot Button */
                 if (ms.x >= win_x+210 && ms.x < win_x+360 && ms.y >= win_y+200 && ms.y < win_y+240) {
                      mode = 1; focus = 4;
                 }
                 /* Inputs focus */
                 if (ms.y >= win_y+110 && ms.y < win_y+140) focus = 0; /* Updated Y for input box 1 */
                 if (ms.y >= win_y+160 && ms.y < win_y+190) focus = 1; /* Updated Y for input box 2 */
             } else {
                 if (ms.x >= win_x+40 && ms.x < win_x+360 && ms.y >= win_y+160 && ms.y < win_y+200) {
                      /* Copy paste logic from Enter */
                      /* ... */
                 }
             }
        }
        prev_mx = ms.x; prev_my = ms.y; prev_btn = ms.left_btn;
        
        /* Sleep */
        /* asm("hlt"); - Interrupts enabled? Yes. */
    }
}

void wm_draw_window_content(Window *win, int x, int y, uint32_t color) {
    if (!win) return;
    if (x < 0 || x >= win->width || y < 0 || y >= win->height) return;
    win->buffer[y * win->width + x] = color;
    extern void wm_mark_dirty(void);
    wm_mark_dirty();
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
        
        /* Process Logic */
        wm_update_cursor(&mouse);
        
        /* Composite & Render */
        wm_composite(&mouse);
        
        /* Yield to allow background tasks to run */
        sched_yield();
    }
    
    /* Cleanup */
    term_clear();
}

/* Global Drag State */
static int drag_win_id = -1;

void wm_update_cursor(void *ctx) {
         MouseState *mouse = (MouseState*)ctx;
         /* Window Dragging Logic */
         if (mouse->left_btn) {
            if (drag_win_id == -1) {
                /* Try to pick up a window (Front to Back for clicking) */
                for (int i = window_count - 1; i >= 0; i--) {
                    Window *win = windows[i];
                    /* Hit test title bar */
                    if (mouse->x >= win->x && mouse->x < win->x + win->width &&
                        mouse->y >= win->y - TITLE_BAR_HEIGHT && mouse->y < win->y) {
                            
                        drag_win_id = i;
                        win->data_offset_x = mouse->x - win->x;
                        win->data_offset_y = mouse->y - win->y;
                        
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
                win->x = mouse->x - win->data_offset_x;
                win->y = mouse->y - win->data_offset_y;
            }
        } else {
            drag_win_id = -1;
        }
}
