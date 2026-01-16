#include "nux.h"

// --- Graphics 2D Lib (Transpiled) ---
void gfx_clear(nux_image* img, nux_int color) {
    img_fill(img, color);
}

void img_set_safe(nux_image* img, nux_int x, nux_int y, nux_int color) {
    img_set(img, x, y, color);
}

void gfx_rect_fill(nux_image* handle, nux_int x, nux_int y, nux_int w, nux_int h, nux_int color) {
    for (int i = 0; i < h; i++) {
        for (int j = 0; j < w; j++) {
            img_set_safe(handle, x + j, y + i, color);
        }
    }
}

// --- Snake Game Logic (Transpiled) ---
int main() {
    nux_init(); // Boot Runtime
    
    print("Starting Snake (C-Transpiled)...");
    nux_image* screen = img_alloc(640, 480);
    
    // Grid size
    nux_int GX = 32;
    nux_int GY = 24;
    nux_int SZ = 20; 
    
    // State
    nux_int hx = 10;
    nux_int hy = 10;
    nux_int vx = 1;
    nux_int vy = 0;
    
    nux_int ax = 5;
    nux_int ay = 5;
    
    // Tail Arrays
    nux_array* tx = Array_new(100);
    nux_array* ty = Array_new(100);
    nux_int tlen = 0;
    
    nux_int score = 0;
    bool game_over = false;
    
    while (!game_over) {
        // Input
        if (is_key_down(0)) { // Up
            if (vy != 1) { vx = 0; vy = -1; }
        }
        if (is_key_down(1)) { // Down
            if (vy != -1) { vx = 0; vy = 1; }
        }
        if (is_key_down(2)) { // Left
            if (vx != 1) { vx = -1; vy = 0; }
        }
        if (is_key_down(3)) { // Right
            if (vx != -1) { vx = 1; vy = 0; }
        }
        if (is_key_down(5)) { // Esc
            game_over = true;
        }
        
        // Update Tail
        for (int i = tlen; i > 0; i--) {
            Array_set(tx, i, Array_get(tx, i - 1));
            Array_set(ty, i, Array_get(ty, i - 1));
        }
        Array_set(tx, 0, hx);
        Array_set(ty, 0, hy);
        
        // Move Head
        hx = hx + vx;
        hy = hy + vy;
        
        // Collision Wall
        if (hx < 0) { hx = GX - 1; }
        if (hx >= GX) { hx = 0; }
        if (hy < 0) { hy = GY - 1; }
        if (hy >= GY) { hy = 0; }
        
        // Collision Tail
        for (int i = 0; i < tlen; i++) {
            if (hx == Array_get(tx, i)) {
                if (hy == Array_get(ty, i)) {
                   // Die
                   score = 0;
                   tlen = 0;
                   print("Died!");
                }
            }
        }
        
        // Eat Apple
        if (hx == ax) {
            if (hy == ay) {
                score = score + 1;
                tlen = tlen + 1;
                // Respawn
                ax = (ax + 7) - ((ax + 7) / GX) * GX; 
                ay = (ay + 3) - ((ay + 3) / GY) * GY; 
                if (ax == hx) { ax = ax + 1; }
            }
        }
        
        // Draw
        gfx_clear(screen, 0x000000); // Black
        
        // Draw Apple (Red)
        gfx_rect_fill(screen, ax * SZ, ay * SZ, SZ - 2, SZ - 2, 0xFF0000);
        
        // Draw Head (Green)
        gfx_rect_fill(screen, hx * SZ, hy * SZ, SZ - 2, SZ - 2, 0x00FF00);
        
        // Draw Tail (Dark Green)
        for (int i = 0; i < tlen; i++) {
            nux_int cx = Array_get(tx, i);
            nux_int cy = Array_get(ty, i);
            gfx_rect_fill(screen, cx * SZ, cy * SZ, SZ - 2, SZ - 2, 0x008800);
        }
        
        img_draw(screen, 0, 0);
        
        nux_sleep(50); // 20 FPS
    }
    
    print("Game Over.");
    print("Final Score:");
    print_int(score);
    return 0;
}
