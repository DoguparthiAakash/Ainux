#ifndef GFX_H
#define GFX_H

#include <stdint.h>

/* Colors (0xRRGGBB) */
#define COLOR_BLACK  0x000000
#define COLOR_WHITE  0xFFFFFF
#define COLOR_RED    0xFF0000
#define COLOR_GREEN  0x00FF00
#define COLOR_BLUE   0x0000FF
#define COLOR_GRAY   0x808080
#define COLOR_DARK_GRAY 0x404040

void gfx_init(void *framebuffer_addr, uint64_t width, uint64_t height, uint64_t pitch);
void gfx_put_pixel(int x, int y, uint32_t color);
void gfx_draw_rect(int x, int y, int w, int h, uint32_t color);
void gfx_fill_rect(int x, int y, int w, int h, uint32_t color);
void gfx_draw_line(int x0, int y0, int x1, int y1, uint32_t color);
void gfx_put_pixel_safe(int x, int y, uint32_t color);
void gfx_clear(uint32_t color);

/* For Mouse Cursor */
void gfx_draw_cursor(int x, int y);

/* Get FB Info */
void gfx_get_info(uint64_t *width, uint64_t *height, uint64_t *pitch, void **addr);

#endif
