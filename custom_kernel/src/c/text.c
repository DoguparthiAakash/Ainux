#include "gfx.h"
#include "font.h"
#include <stdint.h>
#include <stddef.h>

// Simple wrapper for drawing a character at text coordinates
// x, y are in character cells (not pixels)
void c_draw_char(int x, int y, uint32_t c, uint32_t fg_color, uint32_t bg_color) {
    if (c == '\n' || c == '\r') return;
    
    int font_index = 0;
    const uint8_t *glyph = NULL;
    uint8_t synthetic_glyph[8] = {0};

    if (c >= 32 && c <= 126) {
        font_index = c - 32;
        glyph = font_8x8[font_index];
    } else if (c == 0x2588) { // Total Block █
        for(int i=0;i<8;i++) synthetic_glyph[i] = 0xFF;
        glyph = synthetic_glyph;
    } else if (c == 0x2593) { // Dark Shade ▓
        for(int i=0;i<8;i++) synthetic_glyph[i] = (i % 2 == 0) ? 0xDD : 0x77;
        glyph = synthetic_glyph;
    } else if (c == 0x2592) { // Medium Shade ▒
        for(int i=0;i<8;i++) synthetic_glyph[i] = (i % 2 == 0) ? 0x55 : 0xAA;
        glyph = synthetic_glyph;
    } else if (c == 0x2591) { // Light Shade ░
        for(int i=0;i<8;i++) synthetic_glyph[i] = (i % 2 == 0) ? 0x11 : 0x44;
        glyph = synthetic_glyph;
    } else if (c == 0x2261) { // Congruent To ≡
        synthetic_glyph[1] = 0xFF;
        synthetic_glyph[4] = 0xFF;
        synthetic_glyph[7] = 0xFF;
        glyph = synthetic_glyph;
    } else if (c == 0x2584) { // Lower half block ▄
        for(int i=4;i<8;i++) synthetic_glyph[i] = 0xFF;
        glyph = synthetic_glyph;
    } else if (c == 0x2580) { // Upper half block ▀
        for(int i=0;i<4;i++) synthetic_glyph[i] = 0xFF;
        glyph = synthetic_glyph;
    } else if (c == 0x2554) { // Double Top-Left ╔
        synthetic_glyph[2] = 0xFC; synthetic_glyph[3] = 0x04;
        synthetic_glyph[4] = 0xFC; synthetic_glyph[5] = 0x04;
        glyph = synthetic_glyph;
    } else if (c == 0x2557) { // Double Top-Right ╗
        synthetic_glyph[2] = 0x3F; synthetic_glyph[3] = 0x20;
        synthetic_glyph[4] = 0x3F; synthetic_glyph[5] = 0x20;
        glyph = synthetic_glyph;
    } else if (c == 0x255A) { // Double Bottom-Left ╚
        synthetic_glyph[5] = 0x04; synthetic_glyph[6] = 0xFC;
        synthetic_glyph[2] = 0x04; synthetic_glyph[3] = 0xFC;
        glyph = synthetic_glyph;
    } else if (c == 0x255D) { // Double Bottom-Right ╝
        synthetic_glyph[5] = 0x20; synthetic_glyph[6] = 0x3F;
        synthetic_glyph[2] = 0x20; synthetic_glyph[3] = 0x3F;
        glyph = synthetic_glyph;
    } else if (c == 0x2550) { // Double Horizontal ═
        synthetic_glyph[2] = 0xFF; synthetic_glyph[4] = 0xFF;
        glyph = synthetic_glyph;
    } else if (c == 0x2551) { // Double Vertical ║
        for(int i=0;i<8;i++) synthetic_glyph[i] = 0x24; // 00100100
        glyph = synthetic_glyph;
    } else if (c == '=') {
        font_index = '=' - 32;
        glyph = font_8x8[font_index];
    } else if (c == '+') {
        font_index = '+' - 32;
        glyph = font_8x8[font_index];
    } else {
        font_index = '?' - 32;
        glyph = font_8x8[font_index];
    }
    
    // Convert character coordinates to pixels
    int px = x * 8;
    int py = y * 12;  // 12 pixel line height
    
    for (int dy = 0; dy < 8; dy++) {
        uint8_t row = glyph[dy];
        for (int dx = 0; dx < 8; dx++) {
            uint32_t color = (row & (1 << dx)) ? fg_color : bg_color;
            if (color != bg_color) gfx_put_pixel_safe(px + dx, py + dy, color);
            else gfx_put_pixel_safe(px + dx, py + dy, color);
        }
    }
}

// Clear screen
void c_clear_screen(uint32_t color) {
    gfx_clear(color);
}
