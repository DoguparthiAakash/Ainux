
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
    pub orig_x: isize,
    pub orig_y: isize,
    pub orig_width: usize,
    pub orig_height: usize,
    pub text_lines: Vec<String>,
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
            content: Vec::new(), // Do not preallocate huge buffers by default
            dragging: false,
            is_maximized: false,
            is_minimized: false,
            orig_x: x,
            orig_y: y,
            orig_width: w,
            orig_height: h,
            text_lines: Vec::new(),
        }
    }

    pub fn allocate_content(&mut self) {
        if self.content.is_empty() {
            self.content = alloc::vec![0xFFFFFFFF; self.width * self.height];
        }
    }

    pub fn get_close_button_rect(&self) -> crate::gui::rect::Rect {
        crate::gui::rect::Rect::new(self.x + 8, self.y + 6, 12, 12)
    }

    pub fn get_min_button_rect(&self) -> crate::gui::rect::Rect {
        crate::gui::rect::Rect::new(self.x + 24, self.y + 6, 12, 12)
    }

    pub fn get_max_button_rect(&self) -> crate::gui::rect::Rect {
        crate::gui::rect::Rect::new(self.x + 40, self.y + 6, 12, 12)
    }

    pub fn draw(&self, buffer: &mut [u32], stride: usize) {
        if self.is_minimized { return; }
        
        // 1. Draw Titlebar (Adwaita Dark Headerbar)
        let header_h = 36;
        let title_color = 0xFF303030; 
        Graphics::draw_rect_to_buffer(buffer, stride, self.x as usize, self.y as usize, self.width, header_h, title_color);
        
        // 2. Draw Window Content Background (Adwaita Dark Mode)
        Graphics::draw_rect_to_buffer(buffer, stride, self.x as usize, (self.y + header_h as isize) as usize, self.width, self.height - header_h, 0xFF1E1E1E);

        // 3. Draw Border (Subtle)
        let border_col = 0xFF242424;
        Graphics::draw_rect_to_buffer(buffer, stride, self.x as usize, self.y as usize, self.width, 1, border_col); // Top
        Graphics::draw_rect_to_buffer(buffer, stride, self.x as usize, self.y as usize, 1, self.height, border_col); // Left
        Graphics::draw_rect_to_buffer(buffer, stride, (self.x + self.width as isize - 1) as usize, self.y as usize, 1, self.height, border_col); // Right
        Graphics::draw_rect_to_buffer(buffer, stride, self.x as usize, (self.y + self.height as isize - 1) as usize, self.width, 1, border_col); // Bottom

        // 4. Draw GNOME Window Controls
        let btn_bg = 0xFF4A4A4A;
        let close_rect = self.get_close_button_rect();
        Graphics::draw_rect_to_buffer(buffer, stride, close_rect.x as usize, close_rect.y as usize, close_rect.w, close_rect.h, 0xFFE04343); // Close is red on hover, we keep red for now
        
        let min_rect = self.get_min_button_rect();
        Graphics::draw_rect_to_buffer(buffer, stride, min_rect.x as usize, min_rect.y as usize, min_rect.w, min_rect.h, btn_bg);

        let max_rect = self.get_max_button_rect();
        Graphics::draw_rect_to_buffer(buffer, stride, max_rect.x as usize, max_rect.y as usize, max_rect.w, max_rect.h, btn_bg);

        // 5. Content
        let content_w = self.width;
        let content_h = self.height - header_h;
        if self.content.len() >= content_w * content_h {
             Graphics::copy_buffer(buffer, stride, &self.content, content_w, self.x as usize, (self.y + header_h as isize) as usize, content_w, content_h);
        }
    }

    pub fn draw_text(&self) {
        if self.is_minimized { return; }
        
        let header_h = 36;
        
        // Title text centered (GNOME style)
        let text_len = self.title.len() * 8;
        let title_x = if self.width > text_len { self.x as usize + (self.width - text_len) / 2 } else { self.x as usize + 60 };
        let cy = self.y as usize + (header_h - 12) / 2; // Center vertically in header
        
        let mut cx = title_x;
        for c in self.title.chars() {
            crate::drivers::video::draw_char_raw(cx, cy, c, 0xFFFFFFFF); // White text for dark titlebar
            cx += 8;
        }
        
        let mut text_y = self.y as usize + header_h + 8;
        for line in &self.text_lines {
            let mut text_x = self.x as usize + 8;
            for c in line.chars() {
                crate::drivers::video::draw_char_raw(text_x, text_y, c, 0xFFDDDDDD); // Light text for dark content
                text_x += 8;
            }
            text_y += 16;
        }
    }

    // Simple paint helper for content
    pub fn fill_content(&mut self, color: u32) {
        for pixel in self.content.iter_mut() {
            *pixel = color;
        }
    }
}
