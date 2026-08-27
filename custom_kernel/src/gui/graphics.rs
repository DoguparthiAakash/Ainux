
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
    // Alpha Blending Helper
    #[inline(always)]
    pub fn blend_colors(bg: u32, fg: u32) -> u32 {
        let alpha = (fg >> 24) & 0xFF;
        if alpha == 255 { return fg; }
        if alpha == 0 { return bg; }

        let inv_alpha = 255 - alpha;

        let r_bg = (bg >> 16) & 0xFF;
        let g_bg = (bg >> 8) & 0xFF;
        let b_bg = bg & 0xFF;

        let r_fg = (fg >> 16) & 0xFF;
        let g_fg = (fg >> 8) & 0xFF;
        let b_fg = fg & 0xFF;

        let r = (r_fg * alpha + r_bg * inv_alpha) / 255;
        let g = (g_fg * alpha + g_bg * inv_alpha) / 255;
        let b = (b_fg * alpha + b_bg * inv_alpha) / 255;

        (0xFF << 24) | (r << 16) | (g << 8) | b
    }

    // Generic plot to a slice (for backbuffer)
    #[inline(always)]
    pub fn plot_pixel_unchecked(buffer: &mut [u32], width: usize, x: usize, y: usize, color: u32) {
        if x < width && y * width + x < buffer.len() {
            let idx = y * width + x;
            // Check for alpha
            if (color >> 24) != 0xFF {
                buffer[idx] = Self::blend_colors(buffer[idx], color);
            } else {
                buffer[idx] = color;
            }
        }
    }

    // Standard VRAM Plot (Legacy/Direct)
    pub fn plot_pixel(x: usize, y: usize, color: Color) {
        crate::drivers::video::draw_pixel(x as i64, y as i64, color.to_u32());
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
        crate::drivers::video::fill_rect(x as i64, y as i64, w as i64, h as i64, color.to_u32());
    }
    
    // Fast Rect for clearing screen or solid windows
    pub fn fill_screen(color: Color) {
        let w = *video::FRAMEBUFFER_WIDTH.lock();
        let h = *video::FRAMEBUFFER_HEIGHT.lock();
        crate::drivers::video::fill_rect(0, 0, w as i64, h as i64, color.to_u32());
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
        crate::drivers::video::blit_buffer(src, x as i32, y as i32, w as i32, h as i32, w as i32);
    }
}
