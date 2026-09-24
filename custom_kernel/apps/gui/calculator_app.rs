use alloc::string::{String, ToString};
use crate::gui::app::App;
use crate::drivers::video;

pub struct CalculatorApp {
    display: String,
}

impl CalculatorApp {
    pub fn new() -> Self {
        Self {
            display: String::from("0"),
        }
    }
}

impl App for CalculatorApp {
    fn update(&mut self) {}

    fn draw(&mut self, buffer: &mut [u32], width: usize, height: usize) {
        for i in 0..buffer.len() {
            buffer[i] = 0xFFC0C0C0;
        }
        video::draw_text_to_buffer(buffer, width as i64, height as i64, 10, 10, &self.display, 0xFF000000);
        video::draw_text_to_buffer(buffer, width as i64, height as i64, 10, 40, "Press digits, +, -, *, /, =, C", 0xFF000000);
    }

    fn on_mouse_event(&mut self, _x: i32, _y: i32, _buttons: u8) {}

    fn on_key_event(&mut self, c: char) {
        if c == 'c' || c == 'C' {
            self.display = String::from("0");
        } else if c >= '0' && c <= '9' || c == '+' || c == '-' || c == '*' || c == '/' {
            if self.display == "0" {
                self.display.clear();
            }
            self.display.push(c);
        } else if c == '=' || c == '\n' {
            self.display.push_str(" = ???");
        }
    }
}
