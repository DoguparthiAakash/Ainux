use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::gui::window::Window;
use crate::gui::compositor::Compositor;

pub fn launch_task_manager() {
    let mut win = Window::new(1, 100, 100, 300, 200, "Task Manager");
    
    let mut lines = Vec::new();
    lines.push("CPU Usage: 4%".to_string());
    lines.push("Memory: 18MB / 1024MB".to_string());
    lines.push("---------------------".to_string());
    lines.push("Processes:".to_string());
    lines.push("1. kernel_task (1%)".to_string());
    lines.push("2. compositor (3%)".to_string());
    lines.push("3. idle (96%)".to_string());
    
    win.text_lines = lines;
    Compositor::add_window(win);
}

pub fn launch_file_manager() {
    let mut win = Window::new(2, 150, 150, 400, 300, "Files");
    
    let mut lines = Vec::new();
    lines.push("/".to_string());
    lines.push("  |- boot/".to_string());
    lines.push("  |- bin/".to_string());
    lines.push("  |- etc/".to_string());
    lines.push("  |- home/".to_string());
    lines.push("      |- user/".to_string());
    lines.push("          |- documents/".to_string());
    lines.push("          |- downloads/".to_string());
    lines.push("  |- tmp/".to_string());
    lines.push("  |- var/".to_string());
    
    win.text_lines = lines;
    Compositor::add_window(win);
}
