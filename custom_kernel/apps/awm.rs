use crate::gui::compositor::Compositor;
use crate::gui::window::Window;
use crate::drivers::keyboard;
use crate::drivers::video;

pub fn cmd_awm(_args: &[&str]) {
    // 1. Initialize Compositor
    Compositor::init();
    
    // 2. Clear Screen
    video::clear();
    
    // 3. Create Default Windows
    let screen_w = *video::FRAMEBUFFER_WIDTH.lock();
    let screen_h = *video::FRAMEBUFFER_HEIGHT.lock();

    // Shell Window
    let mut shell_win = Window::new(1, 50, 50, 400, 300, "System Shell");
    shell_win.fill_content(0xFF1E1E1E); // Dark Grey
    Compositor::add_window(shell_win);

    // Metrics Window
    let mut metrics_win = Window::new(2, 500, 100, 250, 150, "Kernel Metrics");
    metrics_win.fill_content(0xFF2D2D2D);
    Compositor::add_window(metrics_win);

    video::put_str("awm: Desktop Environment Started. Press ESC to exit.\n");

    // 4. Main Event Loop
    loop {
        // Redraw Desktop
        Compositor::render();
        
        // Handle Global Keys
        if let Some(c) = keyboard::pop_char() {
             if c == '\x1B' { // ESC
                 break;
             }
             // Routing to active window would happen here
        }

        // Wait for next frame (cap at ~60fps)
        for _ in 0..1_000_000 {
             core::hint::spin_loop();
        }
    }

    video::clear();
    video::put_str("awm: Desktop Environment Exited.\n");
}
