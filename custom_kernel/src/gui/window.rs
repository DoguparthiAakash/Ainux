
use alloc::vec::Vec;
use alloc::string::String;
use crate::gui::graphics::{Color, Graphics};
use crate::drivers::video::THEME;

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
        let theme = THEME.lock();
        let bg_color = 0xFF000000 | (theme.bg & 0xFFFFFF);
        drop(theme);

        Window {
            id,
            x,
            y,
            width: w,
            height: h,
            title: String::from(title),
            content: alloc::vec![bg_color; w * h],
            dragging: false,
            is_maximized: false,
            is_minimized: false,
        }
    }

    pub fn get_close_button_rect(&self) -> crate::gui::rect::Rect {
        crate::gui::rect::Rect::new(self.x + 10, self.y + 8, 12, 12) // Mac style left
    }

    pub fn get_min_button_rect(&self) -> crate::gui::rect::Rect {
        crate::gui::rect::Rect::new(self.x + 30, self.y + 8, 12, 12)
    }

    pub fn get_max_button_rect(&self) -> crate::gui::rect::Rect {
        crate::gui::rect::Rect::new(self.x + 50, self.y + 8, 12, 12)
    }

    pub fn draw(&self, buffer: &mut [u32], stride: usize) {
        if self.is_minimized { return; }
        
        let theme = THEME.lock();
        let bg = 0xFF000000 | (theme.bg & 0xFFFFFF);
        let fg = 0xFF000000 | (theme.fg & 0xFFFFFF);
        let accent = 0xFF000000 | (theme.accent & 0xFFFFFF);
        drop(theme);

        // Shadow
        Graphics::draw_shadow(buffer, stride, self.x as usize, self.y as usize, self.width, self.height, 8);

        // Window Frame (Rounded Glass)
        let frame_col = (bg & 0x00FFFFFF) | 0xEE000000;
        Graphics::draw_rounded_rect(buffer, stride, self.x as usize, self.y as usize, self.width, self.height, 12, frame_col);
        
        // Titlebar Gradient
        let g1 = bg;
        let g2 = (bg & 0x00FFFFFF) | 0xFF000000; // Slightly different alpha/shade maybe? 
        // For now just use bg and a darkened version
        let g2_dark = 0xFF000000 | (((bg >> 16 & 0xFF) * 3/4) << 16) | (((bg >> 8 & 0xFF) * 3/4) << 8) | ((bg & 0xFF) * 3/4);

        Graphics::draw_gradient_h(buffer, stride, self.x as usize, self.y as usize, self.width, 28, g1, g2_dark);
        
        // Title Text (Centered)
        let title_x = self.x as usize + (self.width - self.title.len() * 8) / 2;
        Graphics::draw_text(buffer, stride, title_x, self.y as usize + 8, &self.title, fg);

        // Mac Style Buttons
        Graphics::draw_rounded_rect(buffer, stride, self.x as usize + 10, self.y as usize + 8, 12, 12, 6, 0xFFF7768E); // Close
        Graphics::draw_rounded_rect(buffer, stride, self.x as usize + 30, self.y as usize + 8, 12, 12, 6, 0xFFE0AF68); // Min
        Graphics::draw_rounded_rect(buffer, stride, self.x as usize + 50, self.y as usize + 8, 12, 12, 6, 0xFF9ECE6A); // Max

        // Content
        let content_w = self.width - 4; // Padding
        let content_h = self.height - 32;
        if self.content.len() >= content_w * content_h {
             Graphics::copy_buffer(buffer, stride, &self.content, content_w, (self.x + 2) as usize, (self.y + 30) as usize, content_w, content_h);
        }
    }

    pub fn fill_content(&mut self, color: u32) {
        for pixel in self.content.iter_mut() {
            *pixel = color;
        }
    }
}
