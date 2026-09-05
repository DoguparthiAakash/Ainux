#ifndef WM_H
#define WM_H

#include <stdint.h>
#include "gfx.h"

#define MAX_WINDOWS 32
#define TITLE_BAR_HEIGHT 20
#define WINDOW_BORDER_COLOR 0x0000FF
#define WINDOW_TITLE_BG     0x000080
#define WINDOW_TITLE_FG     0xFFFFFF

#define TASKBAR_HEIGHT 40
#define ICON_SIZE 64
#define ICON_SPACING 32

typedef struct Window {
    int id;
    int x, y;
    int width, height;
    uint32_t *buffer; /* Pixel data for the window content */
    char title[32];
    int dragging;
    int data_offset_x, data_offset_y; /* For drag offset */
    int cursor_x, cursor_y; /* For Console Output */
} Window;

typedef struct DesktopIcon {
    char label[32];
    int x, y;
    void (*on_click)(void);
} DesktopIcon;

void wm_init(void);
Window* wm_create_window(int x, int y, int width, int height, const char *title);
void wm_draw_window_content(Window *win, int x, int y, uint32_t color);
void wm_fill_rect(Window *win, int x, int y, int w, int h, uint32_t color);
void wm_console_write(Window *win, const char *str);
void wm_destroy_window(Window *win);
void wm_composite(void *mouse_state_ptr); /* void* to avoid typedef dep header hell if MouseState not visible */
void wm_update_cursor(void *mouse_state_ptr);
void wm_run(void); /* Main loop */

#endif
