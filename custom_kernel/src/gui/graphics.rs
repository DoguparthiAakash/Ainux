
use crate::drivers::video::{self, FRAMEBUFFER_ADDR, FRAMEBUFFER_WIDTH, FRAMEBUFFER_HEIGHT, FRAMEBUFFER_PITCH, FRAMEBUFFER_BPP};

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const RED: Color = Color { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0, a: 255 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255, a: 255 };
    pub const GRAY: Color = Color { r: 128, g: 128, b: 128, a: 255 };

    pub fn to_u32(self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
    
    pub fn from_u32(val: u32) -> Self {
        Self {
            a: (val >> 24) as u8,
            r: (val >> 16) as u8,
            g: (val >> 8) as u8,
            b: val as u8,
        }
    }
    
    pub fn from_hex(hex: u32) -> Color {
        Color {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }
}

pub struct Graphics;

impl Graphics {
    // Generic plot to a slice (for backbuffer)
    #[inline(always)]
    pub fn plot_pixel_unchecked(buffer: &mut [u32], width: usize, x: usize, y: usize, color: u32) {
        if x < width && y * width + x < buffer.len() {
            buffer[y * width + x] = color;
        }
    }

    // Standard VRAM Plot (Legacy/Direct)
    pub fn plot_pixel(x: usize, y: usize, color: Color) {
        unsafe {
            let fb_addr = *video::FRAMEBUFFER_ADDR.lock();
            let fb_width = *video::FRAMEBUFFER_WIDTH.lock();
            let fb_height = *video::FRAMEBUFFER_HEIGHT.lock();
            let fb_pitch = *video::FRAMEBUFFER_PITCH.lock();
            
            if x >= fb_width || y >= fb_height { return; }
            let offset = y * fb_pitch + x * 4;
            let ptr = (fb_addr + offset as u64) as *mut u32;
            *ptr = color.to_u32();
        }
    }

    pub fn draw_rect_to_buffer(buffer: &mut [u32], stride: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for i in 0..h {
            for j in 0..w {
                 Self::plot_pixel_unchecked(buffer, stride, x + j, y + i, color);
            }
        }
    }
    
    pub fn copy_buffer(target: &mut [u32], t_stride: usize, src: &[u32], s_stride: usize, dx: usize, dy: usize, w: usize, h: usize) {
        for i in 0..h {
            for j in 0..w {
                if i * s_stride + j < src.len() {
                     let col = src[i * s_stride + j];
                     // Alpha check
                     if (col >> 24) != 0 {
                        Self::plot_pixel_unchecked(target, t_stride, dx + j, dy + i, col);
                     }
                }
            }
        }
    }

    pub fn fill_rect(x: usize, y: usize, w: usize, h: usize, color: Color) {
        for i in 0..h {
            for j in 0..w {
                Self::plot_pixel(x + j, y + i, color);
            }
        }
    }
    
    // Fast Rect for clearing screen or solid windows
    pub fn fill_screen(color: Color) {
        let w = *video::FRAMEBUFFER_WIDTH.lock();
        let h = *video::FRAMEBUFFER_HEIGHT.lock();
        Self::fill_rect(0, 0, w, h, color);
    }
    
    pub fn draw_line(x0: isize, y0: isize, x1: isize, y1: isize, color: Color) {
        let mut x0 = x0;
        let mut y0 = y0;
        let dx = if x1 > x0 { x1 - x0 } else { x0 - x1 };
        let dy = if y1 > y0 { y0 - y1 } else { y1 - y0 };
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                 Self::plot_pixel(x0 as usize, y0 as usize, color);
            }
            if x0 == x1 && y0 == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
    
    // Blit (Texture Copy) - For Cards!
    // src: array of u32 (ARGB)
    pub fn blit(x: usize, y: usize, w: usize, h: usize, src: &[u32]) {
        for i in 0..h {
            for j in 0..w {
                if i * w + j >= src.len() { break; }
                let color_val = src[i * w + j];
                // Check Alpha (Simple check: if not fully transparent)
                if (color_val >> 24) != 0 {
                    let color = Color::from_hex(color_val);
                    Self::plot_pixel(x + j, y + i, color);
                }
            }
        }
    }
}
