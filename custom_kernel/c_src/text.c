#include "gfx.h"
#include "font.h"
#include <stdint.h>
#include <stddef.h>

// Simple wrapper for drawing a character at text coordinates
// x, y are in character cells (not pixels)
void c_draw_char(int x, int y, uint32_t c, uint32_t fg_color, uint32_t bg_color) {
    if (c == '\n' || c == '\r') return;
    
    const uint8_t *glyph = NULL;
    uint8_t synth[16] = {0};
    int is_synth = 0;

    // 1. Character Categorization & Glyph Selection
    if (c >= 0 && c < 256) {
        glyph = font_8x16[c];
    } else {
        is_synth = 1;
        switch(c) {
            case 0x2588: // Full Block █
                for(int i=0; i<12; i++) { synth[i] = 0xFF; } break;
            case 0x2593: // Dark Shade ▓
                for(int i=0; i<12; i++) { synth[i] = (i % 2 == 0) ? 0xDD : 0x77; } break;
            case 0x2592: // Medium Shade ▒
                for(int i=0; i<12; i++) { synth[i] = (i % 2 == 0) ? 0x55 : 0xAA; } break;
            case 0x2591: // Light Shade ░
                for(int i=0; i<12; i++) { synth[i] = (i % 2 == 0) ? 0x11 : 0x22; } break;
            case 0x2500: // Horizontal line ─
                synth[5] = 0xFF; synth[6] = 0xFF; break;
            case 0x2502: // Vertical line │
                for(int i=0; i<12; i++) { synth[i] = 0x18; } break;
            case 0x250C: // Top-Left ┌
                synth[5] = 0xF0; synth[6] = 0xF0;
                for(int i=7; i<12; i++) { synth[i] = 0x10; } break;
            case 0x2510: // Top-Right ┐
                synth[5] = 0x0F; synth[6] = 0x0F;
                for(int i=7; i<12; i++) { synth[i] = 0x08; } break;
            case 0x2514: // Bottom-Left └
                for(int i=0; i<5; i++) { synth[i] = 0x10; }
                synth[5] = 0xF0; synth[6] = 0xF0; break;
            case 0x2518: // Bottom-Right ┘
                for(int i=0; i<5; i++) { synth[i] = 0x08; }
                synth[5] = 0x0F; synth[6] = 0x0F; break;
            case 0x2550: // Double Horizontal ═
                synth[4] = 0xFF; synth[7] = 0xFF; break;
            case 0x2551: // Double Vertical ║
                for(int i=0; i<12; i++) { synth[i] = 0x24; } break;
            case 0x2554: // Double Top-Left ╔
                synth[4] = 0xFC; synth[5] = 0x04; synth[6] = 0x04; synth[7] = 0xFC;
                for(int i=8; i<12; i++) { synth[i] = 0x24; } break;
            case 0x2557: // Double Top-Right ╗
                synth[4] = 0x3F; synth[5] = 0x20; synth[6] = 0x20; synth[7] = 0x3F;
                for(int i=8; i<12; i++) { synth[i] = 0x24; } break;
            case 0x255A: // Double Bottom-Left ╚
                for(int i=0; i<4; i++) { synth[i] = 0x24; }
                synth[4] = 0xFC; synth[5] = 0x04; synth[6] = 0x04; synth[7] = 0xFC; break;
            case 0x255D: // Double Bottom-Right ╝
                for(int i=0; i<4; i++) { synth[i] = 0x24; }
                synth[4] = 0x3F; synth[5] = 0x20; synth[6] = 0x20; synth[7] = 0x3F; break;
            case 0x2580: // Upper block ▀
                for(int i=0; i<6; i++) { synth[i] = 0xFF; } break;
            case 0x203E: // Overline ‾
                synth[0] = 0xFF; synth[1] = 0xFF; break;
            case 0x2261: // Congruent ≡
                synth[2] = 0xFF; synth[5] = 0xFF; synth[8] = 0xFF; break;
            default:
                is_synth = 0; glyph = font_8x16['?']; break;
        }
    }

    int px = x * 8;
    int py = y * 16;

    if (is_synth) {
        gfx_draw_char_fast(px, py, fg_color, bg_color, synth, 0);
    } else {
        gfx_draw_char_fast(px, py, fg_color, bg_color, glyph, 0);
    }
}

// Clear screen
void c_clear_screen(uint32_t color) {
    gfx_clear(color);
}
