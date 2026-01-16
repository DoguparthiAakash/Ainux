#include "nux.h"
#include <termios.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <time.h>
#include <string.h>

// Terminal State
struct termios orig_termios;
int input_key = -1;

void disableRawMode() {
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
    printf("\033[?25h"); // Show cursor
}

void enableRawMode() {
    tcgetattr(STDIN_FILENO, &orig_termios);
    atexit(disableRawMode);
    
    struct termios raw = orig_termios;
    raw.c_lflag &= ~(ECHO | ICANON); // Disable echo and canonical mode
    raw.c_cc[VMIN] = 0;
    raw.c_cc[VTIME] = 0; // Non-blocking
    
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw);
    printf("\033[?25l"); // Hide cursor
}

void nux_init() {
    enableRawMode();
    // Clear screen
    printf("\033[2J");
}

// Arrays
nux_array* Array_new(nux_int capacity) {
    nux_array* arr = malloc(sizeof(nux_array));
    arr->capacity = capacity;
    arr->length = 0;
    arr->data = malloc(sizeof(nux_int) * capacity);
    return arr;
}
void Array_set(nux_array* arr, nux_int index, nux_int val) {
    if(index < arr->capacity) arr->data[index] = val;
}
nux_int Array_get(nux_array* arr, nux_int index) {
    if(index < arr->capacity) return arr->data[index];
    return 0;
}
nux_int Array_len(nux_array* arr) { return arr->length; }

// Images
nux_image* img_alloc(nux_int width, nux_int height) {
    nux_image* img = malloc(sizeof(nux_image));
    img->width = width;
    img->height = height;
    img->pixels = malloc(sizeof(uint32_t) * width * height);
    return img;
}

void img_set(nux_image* img, nux_int x, nux_int y, nux_int color) {
    if (x >= 0 && x < img->width && y >= 0 && y < img->height) {
        img->pixels[y * img->width + x] = (uint32_t)color;
    }
}

void img_fill(nux_image* img, nux_int color) {
    for(int i=0; i<img->width*img->height; i++) {
        img->pixels[i] = (uint32_t)color;
    }
}

// ANSI Rendering
// We need to scale down? 640x480 is too big for terminal.
// Helper: Map RGB to closest ANSI color or simple threshold
void img_draw(nux_image* img, nux_int x, nux_int y) {
    // We only draw every 10th pixel or scale down?
    // Snake game uses 20x20 blocks roughly. 
    // Grid size GX=32, GY=24.
    // We can interpret the buffer pixels.
    // Or just sample.
    
    // Reset Cursor
    printf("\033[H");
    
    int scale_x = 20; // Match SZ in snake.nux
    int scale_y = 20; // Terminals chars are tall, roughly 2:1 aspect ratio vs pixels?
    
    // Grid: 32 x 24
    for(int gy = 0; gy < 24; gy++) {
        for(int gx = 0; gx < 32; gx++) {
            // Sample center of block
            int px = gx * 20 + 10;
            int py = gy * 20 + 10;
            uint32_t color = img->pixels[py * img->width + px];
            
            // Simple Color Mapping
            if ((color & 0xFFFFFF) == 0) printf("\033[40m  ");           // Black Background
            else if ((color & 0xFF0000) > 0x800000) printf("\033[41m  "); // Red
            else if ((color & 0x00FF00) > 0x8000) printf("\033[42m  ");   // Green
            else printf("\033[47m  "); // White/Else
        }
        printf("\033[0m\n"); // Reset line
    }
    fflush(stdout);
}

// Non-blocking input handling
void process_input() {
    char c;
    if (read(STDIN_FILENO, &c, 1) == 1) {
        if (c == 27) { // Escape sequence?
            char seq[2];
            if (read(STDIN_FILENO, &seq[0], 1) == 1 &&
                read(STDIN_FILENO, &seq[1], 1) == 1) {
                if (seq[0] == '[') {
                    switch (seq[1]) {
                        case 'A': input_key = 0; break; // Up
                        case 'B': input_key = 1; break; // Down
                        case 'C': input_key = 3; break; // Right
                        case 'D': input_key = 2; break; // Left
                    }
                }
            } else {
                input_key = 5; // Esc
            }
        } else {
            // Map WSAD
            if (c == 'w') input_key = 0;
            if (c == 's') input_key = 1;
            if (c == 'a') input_key = 2;
            if (c == 'd') input_key = 3;
            if (c == 'q') input_key = 5;
        }
    }
}

bool is_key_down(nux_int key_code) {
    process_input(); // Check internal buffer
    bool res = (input_key == key_code);
    // Auto-clear key after check? No, state persistent for frame?
    // Snake checks all keys.
    // Simple state machine:
    // If input_key is set, return true for that key once per frame?
    // Let's just return true if match.
    // Reset happens after frame or implicitly.
    // Actually, process_input is called multiple times.
    return res;
}

void print(const char* msg) { 
    // printf("%s\n", msg); // Messes up graphics
} 
void print_int(nux_int val) { }

void nux_sleep(int ms) {
    usleep(ms * 1000);
    input_key = -1; // Reset for next frame
}

void nux_main_loop_step() {
    input_key = -1; // Clear at start of frame logic (managed by user calling mechanism)
}
