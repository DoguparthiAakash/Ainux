#include "gfx.h"
#include "font.h"
#include <stdint.h>

// Simple wrapper for drawing a character at text coordinates
// x, y are in character cells (not pixels)
void c_draw_char(int x, int y, char c, uint32_t fg_color, uint32_t bg_color) {
    if (c == '\n' || c == '\r') return;
    
    int font_index = c - 32;
    if (font_index < 0 || font_index >= 95) font_index = 0;
    
    const uint8_t *glyph = font_8x8[font_index];
    
    // Convert character coordinates to pixels
    int px = x * 8;
    int py = y * 12;  // 12 pixel line height
    
    for (int dy = 0; dy < 8; dy++) {
        uint8_t row = glyph[dy];
        for (int dx = 0; dx < 8; dx++) {
            uint32_t color = (row & (1 << dx)) ? fg_color : bg_color;
            gfx_put_pixel(px + dx, py + dy, color);
        }
    }
}

// Clear screen
void c_clear_screen(uint32_t color) {
    gfx_clear(color);
}
