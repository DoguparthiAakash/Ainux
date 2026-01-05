
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
        }
    }

    pub fn draw(&self, buffer: &mut [u32], stride: usize) {
        // Draw Frame/Titlebar
        // Titlebar
        Graphics::draw_rect_to_buffer(buffer, stride, self.x as usize, self.y as usize, self.width, 20, Color::GRAY.to_u32());
        
        // Border: Skipped simple lines for now, just main rects
        
        // Content
        // Ensure content size matches
        let content_w = self.width;
        let content_h = self.height - 20;
        
        // Safety check
        if self.content.len() >= content_w * content_h {
             Graphics::copy_buffer(buffer, stride, &self.content, content_w, self.x as usize, (self.y + 20) as usize, content_w, content_h);
        }
    }

    // Simple paint helper for content
    pub fn fill_content(&mut self, color: u32) {
        for pixel in self.content.iter_mut() {
            *pixel = color;
        }
    }
}
