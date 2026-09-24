use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::gui::window::Window;
use crate::gui::compositor::Compositor;
use crate::gui::app::AppType;
use crate::gui::terminal_app::TerminalApp;
use crate::gui::notepad_app::NotepadApp;
use crate::drivers::video;

// Helper to draw text into a window's buffer
fn draw_text_to_window(win: &mut Window, text: &str, x: i32, y: i32) {
    let mut curr_x = x;
    for c in text.chars() {
        if c as u32 >= 32 && c as u32 <= 126 {
            let fg = 0x000000; // Black text
            // We use video::draw_char_to_buffer using the window's buffer dimensions
            video::draw_char_to_buffer(&mut win.buffer, win.width as usize, win.height as usize, curr_x as usize, y as usize, c, fg);
        }
        curr_x += 8;
    }
}

pub fn launch_task_manager(comp: &mut Compositor) {
    let mut win = Window::new(1, "Task Manager", 100, 100, 300, 200);
    
    draw_text_to_window(&mut win, "CPU Usage: 4%", 10, 10);
    draw_text_to_window(&mut win, "Memory: 18MB / 1024MB", 10, 30);
    draw_text_to_window(&mut win, "---------------------", 10, 50);
    draw_text_to_window(&mut win, "Processes:", 10, 70);
    draw_text_to_window(&mut win, "1. kernel_task (1%)", 10, 90);
    draw_text_to_window(&mut win, "2. compositor (3%)", 10, 110);
    draw_text_to_window(&mut win, "3. idle (96%)", 10, 130);
    
    comp.add_window(win);
}

pub fn launch_file_manager(comp: &mut Compositor) {
    let mut win = Window::new(2, "Files", 150, 150, 400, 300);
    
    let lines = [
        "/",
        "  |- boot/",
        "  |- bin/",
        "  |- etc/",
        "  |- home/",
        "      |- user/",
        "          |- documents/",
        "          |- downloads/",
        "  |- tmp/",
        "  |- var/",
    ];
    
    for (i, line) in lines.iter().enumerate() {
        draw_text_to_window(&mut win, line, 10, 10 + (i as i32 * 20));
    }
    
    comp.add_window(win);
}

pub fn launch_terminal(comp: &mut Compositor) {
    let mut win = Window::new(3, "Terminal", 200, 200, 500, 350);
    win.app = AppType::Terminal(TerminalApp::new());
    comp.add_window(win);
}

pub fn launch_notepad(comp: &mut Compositor) {
    let mut win = Window::new(4, "Notepad", 300, 300, 400, 300);
    win.app = AppType::Notepad(NotepadApp::new());
    comp.add_window(win);
}
