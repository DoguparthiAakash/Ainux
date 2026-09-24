#include "gfx.h"

static uint8_t *fb_addr = 0;
static uint64_t fb_width = 0;
static uint64_t fb_height = 0;
static uint64_t fb_pitch = 0;
static uint8_t fb_bpp = 32;

void gfx_get_info(uint64_t *width, uint64_t *height, uint64_t *pitch, void **addr) {
    if (width) *width = fb_width;
    if (height) *height = fb_height;
    if (pitch) *pitch = fb_pitch;
    if (addr) *addr = fb_addr;
}

void gfx_init(void *framebuffer_addr, uint64_t width, uint64_t height, uint64_t pitch, uint8_t bpp) {
    fb_addr = (uint8_t *)framebuffer_addr;
    fb_width = width;
    fb_height = height;
    fb_pitch = pitch;
    fb_bpp = bpp;
}

void gfx_put_pixel_safe(int x, int y, uint32_t color) {
    if (!fb_addr) return;
    uint64_t offset = y * fb_pitch + x * (fb_bpp / 8);
    if (fb_bpp == 32) {
        *((uint32_t*)(fb_addr + offset)) = color;
    } else if (fb_bpp == 24) {
        fb_addr[offset] = color & 0xFF;
        fb_addr[offset + 1] = (color >> 8) & 0xFF;
        fb_addr[offset + 2] = (color >> 16) & 0xFF;
    } else if (fb_bpp == 16) {
        uint8_t r = (color >> 16) & 0xFF;
        uint8_t g = (color >> 8) & 0xFF;
        uint8_t b = color & 0xFF;
        uint16_t color16 = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
        *((uint16_t*)(fb_addr + offset)) = color16;
    }
}

void gfx_put_pixel(int x, int y, uint32_t color) {
    if (x < 0 || x >= (int)fb_width || y < 0 || y >= (int)fb_height) return;
    
    uint64_t offset = y * fb_pitch + x * (fb_bpp / 8);
    if (fb_bpp == 32) {
        *((uint32_t*)(fb_addr + offset)) = color;
    } else if (fb_bpp == 24) {
        fb_addr[offset] = color & 0xFF;
        fb_addr[offset + 1] = (color >> 8) & 0xFF;
        fb_addr[offset + 2] = (color >> 16) & 0xFF;
    } else if (fb_bpp == 16) {
        uint8_t r = (color >> 16) & 0xFF;
        uint8_t g = (color >> 8) & 0xFF;
        uint8_t b = color & 0xFF;
        uint16_t color16 = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
        *((uint16_t*)(fb_addr + offset)) = color16;
    }
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
    if (!fb_addr) return;
    if (fb_bpp == 32) {
        uint32_t *dest = (uint32_t *)fb_addr;
        uint64_t pixels = (fb_height * fb_pitch) / 4;
        for (uint64_t i = 0; i < pixels; i++) {
            dest[i] = color;
        }
    } else if (fb_bpp == 24) {
        uint64_t total = fb_height * fb_pitch;
        for (uint64_t i = 0; i < total; i += 3) {
            fb_addr[i] = color & 0xFF;
            fb_addr[i + 1] = (color >> 8) & 0xFF;
            fb_addr[i + 2] = (color >> 16) & 0xFF;
        }
    } else if (fb_bpp == 16) {
        uint16_t *dest = (uint16_t *)fb_addr;
        uint64_t pixels = (fb_height * fb_pitch) / 2;
        uint8_t r = (color >> 16) & 0xFF;
        uint8_t g = (color >> 8) & 0xFF;
        uint8_t b = color & 0xFF;
        uint16_t color16 = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
        for (uint64_t i = 0; i < pixels; i++) {
            dest[i] = color16;
        }
    }
}

void gfx_draw_char_fast(int x, int y, uint32_t fg, uint32_t bg, const uint8_t *bitmap, int transparent) {
    if (!fb_addr) return;
    if (x < 0 || y < 0 || x + 8 > (int)fb_width || y + 16 > (int)fb_height) return;
    
    if (fb_bpp == 32) {
        for (int j = 0; j < 16; j++) {
            uint32_t *row_ptr = (uint32_t *)(fb_addr + (y + j) * fb_pitch + x * 4);
            uint8_t row = bitmap[j];
            for (int i = 0; i < 8; i++) {
                if ((row >> i) & 1) {
                    row_ptr[i] = fg;
                } else if (!transparent) {
                    row_ptr[i] = bg;
                }
            }
        }
    } else if (fb_bpp == 24) {
        for (int j = 0; j < 16; j++) {
            uint8_t *row_ptr = fb_addr + (y + j) * fb_pitch + x * 3;
            uint8_t row = bitmap[j];
            for (int i = 0; i < 8; i++) {
                if ((row >> i) & 1) {
                    row_ptr[i * 3] = fg & 0xFF;
                    row_ptr[i * 3 + 1] = (fg >> 8) & 0xFF;
                    row_ptr[i * 3 + 2] = (fg >> 16) & 0xFF;
                } else if (!transparent) {
                    row_ptr[i * 3] = bg & 0xFF;
                    row_ptr[i * 3 + 1] = (bg >> 8) & 0xFF;
                    row_ptr[i * 3 + 2] = (bg >> 16) & 0xFF;
                }
            }
        }
    } else if (fb_bpp == 16) {
        uint8_t fg_r = (fg >> 16) & 0xFF;
        uint8_t fg_g = (fg >> 8) & 0xFF;
        uint8_t fg_b = fg & 0xFF;
        uint16_t fg16 = ((fg_r & 0xF8) << 8) | ((fg_g & 0xFC) << 3) | (fg_b >> 3);
        
        uint8_t bg_r = (bg >> 16) & 0xFF;
        uint8_t bg_g = (bg >> 8) & 0xFF;
        uint8_t bg_b = bg & 0xFF;
        uint16_t bg16 = ((bg_r & 0xF8) << 8) | ((bg_g & 0xFC) << 3) | (bg_b >> 3);

        for (int j = 0; j < 16; j++) {
            uint16_t *row_ptr = (uint16_t *)(fb_addr + (y + j) * fb_pitch + x * 2);
            uint8_t row = bitmap[j];
            for (int i = 0; i < 8; i++) {
                if ((row >> i) & 1) {
                    row_ptr[i] = fg16;
                } else if (!transparent) {
                    row_ptr[i] = bg16;
                }
            }
        }
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

void gfx_blit_buffer(const uint32_t *src, int x, int y, int w, int h, int src_stride) {
    if (!fb_addr) return;
    
    // Bounds check to avoid drawing outside the framebuffer
    int start_x = (x < 0) ? -x : 0;
    int start_y = (y < 0) ? -y : 0;
    int end_x = (x + w > (int)fb_width) ? (int)fb_width - x : w;
    int end_y = (y + h > (int)fb_height) ? (int)fb_height - y : h;
    
    for (int j = start_y; j < end_y; j++) {
        uint64_t dest_offset = (y + j) * fb_pitch + (x + start_x) * (fb_bpp / 8);
        int src_offset = j * src_stride + start_x;
        
        for (int i = start_x; i < end_x; i++) {
            uint32_t color = src[src_offset];
            
            // Handle simple alpha testing (only draw if not fully transparent)
            if ((color >> 24) != 0) {
                if (fb_bpp == 32) {
                    *((uint32_t*)(fb_addr + dest_offset)) = color;
                } else if (fb_bpp == 24) {
                    fb_addr[dest_offset] = color & 0xFF;
                    fb_addr[dest_offset + 1] = (color >> 8) & 0xFF;
                    fb_addr[dest_offset + 2] = (color >> 16) & 0xFF;
                } else if (fb_bpp == 16) {
                    uint8_t r = (color >> 16) & 0xFF;
                    uint8_t g = (color >> 8) & 0xFF;
                    uint8_t b = color & 0xFF;
                    uint16_t color16 = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
                    *((uint16_t*)(fb_addr + dest_offset)) = color16;
                }
            }
            
            dest_offset += (fb_bpp / 8);
            src_offset++;
        }
    }
}

// Fast path for opaque buffers (like game backbuffers), avoiding branching for WC optimizations
void gfx_blit_buffer_opaque(const uint32_t *src, int x, int y, int w, int h, int src_stride) {
    if (!fb_addr) return;
    
    // Bounds check to avoid drawing outside the framebuffer
    int start_x = (x < 0) ? -x : 0;
    int start_y = (y < 0) ? -y : 0;
    int end_x = (x + w > (int)fb_width) ? (int)fb_width - x : w;
    int end_y = (y + h > (int)fb_height) ? (int)fb_height - y : h;
    
    int draw_w = end_x - start_x;
    if (draw_w <= 0) return;
    
    for (int j = start_y; j < end_y; j++) {
        uint64_t dest_offset = (y + j) * fb_pitch + (x + start_x) * (fb_bpp / 8);
        int src_offset = j * src_stride + start_x;
        
        if (fb_bpp == 32) {
            // Direct memory copy (allows CPU to use ERMS and proper Write-Combining bursts)
            for (int i = 0; i < draw_w; i++) {
                ((uint32_t*)(fb_addr + dest_offset))[i] = src[src_offset + i];
            }
        } else if (fb_bpp == 24) {
            for (int i = 0; i < draw_w; i++) {
                uint32_t color = src[src_offset + i];
                fb_addr[dest_offset] = color & 0xFF;
                fb_addr[dest_offset + 1] = (color >> 8) & 0xFF;
                fb_addr[dest_offset + 2] = (color >> 16) & 0xFF;
                dest_offset += 3;
            }
        } else if (fb_bpp == 16) {
            for (int i = 0; i < draw_w; i++) {
                uint32_t color = src[src_offset + i];
                uint8_t r = (color >> 16) & 0xFF;
                uint8_t g = (color >> 8) & 0xFF;
                uint8_t b = color & 0xFF;
                uint16_t color16 = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
                *((uint16_t*)(fb_addr + dest_offset)) = color16;
                dest_offset += 2;
            }
        }
    }
}
