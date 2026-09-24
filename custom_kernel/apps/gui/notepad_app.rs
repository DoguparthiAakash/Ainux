use alloc::string::String;
use crate::gui::app::App;
use crate::drivers::video;

pub struct NotepadApp {
    content: String,
}

impl NotepadApp {
    pub fn new() -> Self {
        Self {
            content: String::from("Welcome to Notepad!\nStart typing...\n"),
        }
    }
}

impl App for NotepadApp {
    fn update(&mut self) {}

    fn draw(&mut self, buffer: &mut [u32], width: usize, height: usize) {
        // Clear background to white
        for i in 0..buffer.len() {
            buffer[i] = 0xFFFFFFFF;
        }

        let mut y = 4;
        let line_height = 10;
        for line in self.content.split('\n') {
            if y + line_height > height {
                break;
            }
            video::draw_text_to_buffer(buffer, width as i64, height as i64, 4, y as i64, line, 0xFF000000);
            y += line_height;
        }
    }

    fn on_mouse_event(&mut self, _x: i32, _y: i32, _buttons: u8) {}

    fn on_key_event(&mut self, c: char) {
        if c == '\x08' {
            if !self.content.is_empty() {
                self.content.pop();
            }
        } else if c >= ' ' && c <= '~' || c == '\n' {
            self.content.push(c);
        }
    }
}
