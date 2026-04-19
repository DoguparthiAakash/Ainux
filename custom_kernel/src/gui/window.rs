
use alloc::vec::Vec;
use alloc::string::String;
use crate::gui::graphics::{Color, Graphics};

#[derive(Clone)]
pub struct Window {
    pub id: usize,
    pub x: isize,
    pub y: isize,
    pub width: usize,
    pub height: usize,
    pub title: String,
    pub content: Vec<u32>, // Backbuffer (ARGB)
    pub dragging: bool,
    pub is_maximized: bool,
    pub is_minimized: bool,
}

impl Window {
    pub fn new(id: usize, x: isize, y: isize, w: usize, h: usize, title: &str) -> Window {
        Window {
            id,
            x,
            y,
            width: w,
            height: h,
            title: String::from(title),
            content: alloc::vec![0xFFFFFFFF; w * h], // White background
            dragging: false,
            is_maximized: false,
            is_minimized: false,
        }
    }

    pub fn get_close_button_rect(&self) -> crate::gui::rect::Rect {
        crate::gui::rect::Rect::new(self.x + self.width as isize - 24, self.y + 4, 16, 16)
    }

    pub fn get_max_button_rect(&self) -> crate::gui::rect::Rect {
        crate::gui::rect::Rect::new(self.x + self.width as isize - 44, self.y + 4, 16, 16)
    }

    pub fn get_min_button_rect(&self) -> crate::gui::rect::Rect {
        crate::gui::rect::Rect::new(self.x + self.width as isize - 64, self.y + 4, 16, 16)
    }

    pub fn draw(&self, buffer: &mut [u32], stride: usize) {
        if self.is_minimized { return; }
        
        // 1. Draw Titlebar (Modern Gradient/Rounded look)
        let title_color = 0xFF5555FF; // Modern Blue
        Graphics::draw_rect_to_buffer(buffer, stride, self.x as usize, self.y as usize, self.width, 24, title_color);
        
        // 2. Draw Window Content Shadow/Border
        Graphics::draw_rect_to_buffer(buffer, stride, self.x as usize, (self.y + 24) as usize, self.width, self.height - 24, 0xFFEEEEEE);

        // 3. Draw Buttons
        // Close [X] - Red
        let close_rect = self.get_close_button_rect();
        Graphics::draw_rect_to_buffer(buffer, stride, close_rect.x as usize, close_rect.y as usize, close_rect.w, close_rect.h, 0xFFFF4444);
        
        // Max [ ] - Green
        let max_rect = self.get_max_button_rect();
        Graphics::draw_rect_to_buffer(buffer, stride, max_rect.x as usize, max_rect.y as usize, max_rect.w, max_rect.h, 0xFF44FF44);

        // Min [-] - Yellow
        let min_rect = self.get_min_button_rect();
        Graphics::draw_rect_to_buffer(buffer, stride, min_rect.x as usize, min_rect.y as usize, min_rect.w, min_rect.h, 0xFFFFFF44);

        // 4. Content
        let content_w = self.width;
        let content_h = self.height - 24;
        if self.content.len() >= content_w * content_h {
             Graphics::copy_buffer(buffer, stride, &self.content, content_w, self.x as usize, (self.y + 24) as usize, content_w, content_h);
        }
    }

    // Simple paint helper for content
    pub fn fill_content(&mut self, color: u32) {
        for pixel in self.content.iter_mut() {
            *pixel = color;
        }
    }
}
