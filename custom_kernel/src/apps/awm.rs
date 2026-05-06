
use alloc::vec::Vec;
use crate::gui::compositor::Compositor;
use crate::gui::window::Window;
use crate::gui::graphics::Graphics;

pub fn cmd_awm(_args: &[&str]) {
    // 1. Initialize Compositor
    Compositor::init();

    // 2. Create Premium Windows
    let mut win1 = Window::new(1, 100, 100, 600, 400, "Ainux System Dashboard");
    // Draw some mock content for win1
    let w1 = win1.width - 2;
    let h1 = win1.height - 35;
    // Draw a "CPU Usage" bar
    Graphics::draw_text(&mut win1.content, w1, 20, 20, "CPU Usage: [||||||||||      ] 50%", 0xFF9ECE6A);
    Graphics::draw_text(&mut win1.content, w1, 20, 40, "Memory:    [||||||          ] 30%", 0xFF7AA2F7);
    Graphics::draw_text(&mut win1.content, w1, 20, 80, "Welcome to the Silk Titanium Desktop.", 0xFFC0CAF5);

    let mut win2 = Window::new(2, 450, 150, 400, 300, "Terminal");
    let w2 = win2.width - 2;
    Graphics::draw_text(&mut win2.content, w2, 10, 10, "ainux@kernel:~$ _", 0xFFFFFFFF);

    let mut win3 = Window::new(3, 200, 300, 300, 200, "Network");
    let w3 = win3.width - 2;
    Graphics::draw_text(&mut win3.content, w3, 10, 10, "Interface: eth0", 0xFF7DCFFF);
    Graphics::draw_text(&mut win3.content, w3, 10, 30, "Status: Connected", 0xFF9ECE6A);

    Compositor::add_window(win1);
    Compositor::add_window(win2);
    Compositor::add_window(win3);

    // 3. Render Loop
    loop {
        // Redraw Desktop
        Compositor::render();

        // 4. Update Loop Delay (approx 60 FPS)
        for _ in 0..500_000 {
             core::hint::spin_loop();
        }
    }
}
