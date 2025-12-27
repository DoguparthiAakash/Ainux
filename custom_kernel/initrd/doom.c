/* Doom-like Raycaster in Nano-C */
/* Uses Fixed Point Math */

#include <iostream> /* Mock for run cmd detection */

int main() {
    /* Setup */
    int SCREEN_WIDTH = 1024;
    int SCREEN_HEIGHT = 768;
    int MAP_WIDTH = 24;
    int MAP_HEIGHT = 24;
    
    /* Map: 1=Wall, 0=Empty */
    int map_addr = malloc(MAP_WIDTH * MAP_HEIGHT * 4);
    
    /* Fill Map (Simple Borders) */
    int i = 0;
    while (i < MAP_WIDTH * MAP_HEIGHT) {
        int x = i % MAP_WIDTH;
        int y = i / MAP_WIDTH;
        int val = 0;
        if (x == 0 || x == MAP_WIDTH - 1 || y == 0 || y == MAP_HEIGHT - 1) val = 1;
        /* Some pillars */
        if (x == 10 && y == 10) val = 2;
        if (x == 12 && y == 12) val = 3;
        
        poke(map_addr + i * 4, val);
        i = i + 1;
    }

    /* Player */
    int px = 22 * 100; /* x100 fixed point */
    int py = 12 * 100;
    int dirX = -100;
    int dirY = 0;
    int planeX = 0;
    int planeY = 66; /* FOV 0.66 */
    
    int fb = videobase();
    
    print("Welcome to Nano-Doom!");
    print("Controls: Arrow Keys to Move. ESC to Quit.");
    
    int running = 1;
    int frame = 0;
    
    print("Starting Main Loop...");
    
    while (running) {
        
        /* Debug every 10 frames */
        if (frame % 10 == 0) {
            /* print("Frame..."); */
            /* We can't print too much or it flickers */
        }
        frame = frame + 1;
        
        /* ... existing raycasting code ... */
        
        /* Raycasting Loop */
        int x = 0;
        /* Width 320 for speed? 1024 is ALOT for interpreted */
        /* Let's degrade to 320x200 centered for speed first? */
        /* Or just step x+=4 */
        
        while (x < SCREEN_WIDTH) {
             /* ... */
             
             /* Optimization: Stride */
             /* Render every 4th pixel horizontally */
             
            /* Camera X: -1 to 1 */
            int cameraX = (2 * x * 100 / SCREEN_WIDTH) - 100;
            
            /* ... (keep logic) ... */
            
            /* Draw Vertical Line directly to FB */
            int y = drawStart;
            while (y <= drawEnd) {
                 /* Draw 4px width */
                 int addr = (y * 1024 + x) * 4;
                 poke(fb + addr, color);
                 poke(fb + addr + 4, color);
                 poke(fb + addr + 8, color);
                 poke(fb + addr + 12, color);
                 y = y + 1;
            }
            /* Ceiling/Floor */
            /* Skip for speed/simplicity */
            
            x = x + 4; /* Stride 4 */
        }
        
        /* Simple Input Handling hack */
        /* ... */
    }
    return 0;
}
