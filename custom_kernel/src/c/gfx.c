#include "gfx.h"

static uint32_t *fb_addr = 0;
static uint64_t fb_width = 0;
static uint64_t fb_height = 0;
static uint64_t fb_pitch = 0;

void gfx_get_info(uint64_t *width, uint64_t *height, uint64_t *pitch, void **addr) {
    if (width) *width = fb_width;
    if (height) *height = fb_height;
    if (pitch) *pitch = fb_pitch;
    if (addr) *addr = fb_addr;
}

void gfx_init(void *framebuffer_addr, uint64_t width, uint64_t height, uint64_t pitch) {
    fb_addr = (uint32_t *)framebuffer_addr;
    fb_width = width;
    fb_height = height;
    fb_pitch = pitch;
}

void gfx_put_pixel_safe(int x, int y, uint32_t color) {
    if (!fb_addr) return;
    uint64_t offset = y * (fb_pitch / 4) + x;
    fb_addr[offset] = color;
}

void gfx_put_pixel(int x, int y, uint32_t color) {
    if (x < 0 || x >= (int)fb_width || y < 0 || y >= (int)fb_height) return;
    
    /* Legacy Math: pitch is in bytes, divide by 4 for u32 index */
    uint64_t offset = y * (fb_pitch / 4) + x;
    fb_addr[offset] = color;
}

void gfx_draw_rect(int x, int y, int w, int h, uint32_t color) {
    for (int i = 0; i < w; i++) {
        gfx_put_pixel(x + i, y, color);
        gfx_put_pixel(x + i, y + h - 1, color);
    }
    for (int i = 0; i < h; i++) {
        gfx_put_pixel(x, y + i, color);
        gfx_put_pixel(x + w - 1, y + i, color);
    }
}

void gfx_fill_rect(int x, int y, int w, int h, uint32_t color) {
    for (int j = 0; j < h; j++) {
        for (int i = 0; i < w; i++) {
            gfx_put_pixel(x + i, y + j, color);
        }
    }
}

void gfx_draw_line(int x0, int y0, int x1, int y1, uint32_t color) {
    int dx = (x1 > x0) ? (x1 - x0) : (x0 - x1);
    int dy = (y1 > y0) ? (y1 - y0) : (y0 - y1);
    int sx = (x0 < x1) ? 1 : -1;
    int sy = (y0 < y1) ? 1 : -1;
    int err = (dx > dy ? dx : -dy) / 2;
    int e2;

    for (;;) {
        gfx_put_pixel(x0, y0, color);
        if (x0 == x1 && y0 == y1) break;
        e2 = err;
        if (e2 > -dx) { err -= dy; x0 += sx; }
        if (e2 < dy) { err += dx; y0 += sy; }
    }
}

void gfx_clear(uint32_t color) {
    uint64_t total = fb_height * (fb_pitch / 4);
    for (uint64_t i = 0; i < total; i++) {
        fb_addr[i] = color;
    }
}

void gfx_draw_cursor(int x, int y) {
    gfx_draw_rect(x, y, 10, 10, COLOR_BLACK);
    gfx_fill_rect(x+1, y+1, 8, 8, COLOR_WHITE);
}

// Scroll disabled to remove string.h/memcpy dependency
void gfx_scroll_up(int pixels) {
    (void)pixels;
}
