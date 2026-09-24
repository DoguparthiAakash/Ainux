use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::gui::app::App;
use crate::drivers::video;
use crate::fs::pipe::create_pipe;
use crate::fs::vfs::{ArcHandle, FileHandle};
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

static TERMINAL_CMD_QUEUE: Mutex<Option<String>> = Mutex::new(None);
static TERMINAL_READER: Mutex<Option<ArcHandle>> = Mutex::new(None);
static WORKER_SPAWNED: AtomicBool = AtomicBool::new(false);

fn terminal_worker_entry(_: u64) {
    loop {
        let cmd_opt = TERMINAL_CMD_QUEUE.lock().take();
        if let Some(cmd) = cmd_opt {
            crate::shell::execute_command(&cmd);
        } else {
            for _ in 0..100 {
                crate::process::scheduler::yield_now();
            }
        }
    }
}

pub struct TerminalApp {
    lines: Vec<String>,
    current_line: String,
    scroll_offset: usize,
}

impl TerminalApp {
    pub fn new() -> Self {
        if !WORKER_SPAWNED.swap(true, Ordering::SeqCst) {
            let (reader, writer) = create_pipe();
            *TERMINAL_READER.lock() = Some(reader as ArcHandle);
            *crate::shell::CURRENT_OUT.lock() = Some(writer as ArcHandle);
            crate::process::scheduler::spawn_kernel_task(terminal_worker_entry as u64, "terminal_worker");
        }

        let mut lines = Vec::new();
        lines.push(String::from("Ainux Terminal (GUI)"));
        lines.push(String::from("Type 'help' for commands"));
        Self {
            lines,
            current_line: String::new(),
            scroll_offset: 0,
        }
    }

    fn execute(&mut self, cmd: &str) {
        // Send command to worker
        *TERMINAL_CMD_QUEUE.lock() = Some(cmd.to_string());
    }
}

impl App for TerminalApp {
    fn update(&mut self) {
        let mut reader_guard = TERMINAL_READER.lock();
        if let Some(reader) = reader_guard.as_ref() {
            let mut buf = [0u8; 1024];
            if let Ok(n) = reader.read(&mut buf, 0) {
                if n > 0 {
                    if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                        for (i, part) in s.split('\n').enumerate() {
                            if i == 0 {
                                if let Some(last) = self.lines.last_mut() {
                                    last.push_str(part);
                                } else {
                                    self.lines.push(part.to_string());
                                }
                            } else {
                                self.lines.push(part.to_string());
                            }
                        }
                        
                        let line_height = 16;
                        let max_lines = 300 / line_height;
                        if self.lines.len() > max_lines {
                            self.scroll_offset = self.lines.len() - max_lines;
                        }
                    }
                }
            }
        }
    }

    fn draw(&mut self, buffer: &mut [u32], width: usize, height: usize) {
        // Clear background to black
        for i in 0..buffer.len() {
            buffer[i] = 0xFF000000;
        }

        let line_height = 16; // Use 16 for font_8x16
        let max_visible_lines = (height / line_height) - 1; // Leave room for current line
        
        // Ensure scroll_offset is valid
        if self.lines.len() > max_visible_lines {
            if self.scroll_offset > self.lines.len() - max_visible_lines {
                self.scroll_offset = self.lines.len() - max_visible_lines;
            }
        } else {
            self.scroll_offset = 0;
        }

        let mut y = 4;
        let start_idx = self.scroll_offset;
        let end_idx = core::cmp::min(self.lines.len(), start_idx + max_visible_lines);
        
        for i in start_idx..end_idx {
            video::draw_text_to_buffer(buffer, width as i64, height as i64, 4, y as i64, &self.lines[i], 0xFFFFFFFF);
            y += line_height;
        }
        
        if y + line_height <= height {
            let mut display_line = crate::shell::get_prompt();
            display_line.push_str(&self.current_line);
            video::draw_text_to_buffer(buffer, width as i64, height as i64, 4, y as i64, &display_line, 0xFFFFFFFF);
        }
    }

    fn on_mouse_event(&mut self, _x: i32, _y: i32, _buttons: u8) {}

    fn on_key_event(&mut self, c: char) {
        if c == '\n' {
            let cmd = self.current_line.clone();
            
            // Push prompt + cmd to screen
            let mut prompt_line = crate::shell::get_prompt();
            prompt_line.push_str(&cmd);
            self.lines.push(prompt_line);
            
            self.current_line.clear();

            if !cmd.trim().is_empty() {
                // Push an empty line so the command output starts on the next line
                self.lines.push(String::new());
                self.execute(&cmd);
            } else {
                // If empty, just show prompt on next line
                self.lines.push(crate::shell::get_prompt());
            }
        } else if c == '\x08' { // Backspace
            self.current_line.pop();
        } else if c == '\u{2191}' { // Up arrow (scroll up)
            if self.scroll_offset > 0 {
                self.scroll_offset -= 1;
            }
        } else if c == '\u{2193}' { // Down arrow (scroll down)
            let max_visible_lines = (300 / 16) - 1; // roughly
            if self.lines.len() > max_visible_lines && self.scroll_offset < self.lines.len() - max_visible_lines {
                self.scroll_offset += 1;
            }
        } else if c >= ' ' && c <= '~' {
            self.current_line.push(c);
        }
    }
}
