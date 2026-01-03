
print("[Script] Starting Boot Menu...");

/* Constants */
int SCREEN_W = 800;
int SCREEN_H = 600;
int WHITE = 16777215;   /* 0xFFFFFF */
int GRAY  = 2236962;    /* 0x222222 */
int BLUE  = 4474026;    /* 0x4444AA */
int BLACK = 0;

/* Paint Background */
rect(0, 0, SCREEN_W, SCREEN_H, 2105376); /* Dark Blue-ish */

while(1) {
    /* Main Panel */
    rect(200, 150, 400, 300, 3158064); /* 0x303030 */
    rect(200, 150, 400, 40, BLUE);
    
    text(330, 165, "AINUX SCRIPT UI", WHITE);
    
    /* Menu Items */
    text(250, 220, "1. Start Kernel", WHITE);
    text(250, 250, "2. Diagnostics", 11184810); /* Light Gray */
    text(250, 280, "3. Reboot", 11184810);
    
    text(250, 400, "Press [1] to Boot...", WHITE);
    
    /* Input Loop */
    print("Waiting for input...");
    /* We don't have non-blocking check exposed yet in script, so 'input' will block */
    /* This effectively 'freezes' the text/rect updates until keypress */
    /* But that's fine for a boot menu */
    
    int key = input();
    
    if (key == 49) { /* '1' */
        print("Booting...");
        exit(0); /* Exit script to continue kernel boot */
    }
}
