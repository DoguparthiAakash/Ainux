use crate::gui::graphics::Graphics;
use crate::drivers::video::{self, THEME};

pub fn draw_desktop(buffer: &mut [u32], w: usize, h: usize) {
    let theme = THEME.lock();
    let bg_color = 0xFF000000 | (theme.bg & 0xFFFFFF);
    drop(theme);

    // 1. Wallpaper
    for i in 0..buffer.len() {
        buffer[i] = bg_color;
    }
    
    // Subtle Grid Pattern
    for y in (0..h).step_by(40) {
        for x in 0..w {
            Graphics::plot_pixel_unchecked(buffer, w, x, y, 0x10FFFFFF);
        }
    }
}

pub fn draw_overlay(buffer: &mut [u32], w: usize, h: usize) {
    let theme = THEME.lock();
    let bar_col = 0xCC000000 | (theme.bg & 0xFFFFFF); // Use theme bg for bar with transparency
    let accent_col = 0xFF000000 | (theme.accent & 0xFFFFFF);
    let fg_col = 0xFF000000 | (theme.fg & 0xFFFFFF);
    drop(theme);

    // 2. Top Bar
    Graphics::draw_rect_to_buffer(buffer, w, 0, 0, w, 32, bar_col);
    Graphics::draw_rect_to_buffer(buffer, w, 0, 31, w, 1, (accent_col & 0xFFFFFF) | 0x40000000); // Accent Border
    
    // Top Bar Text
    Graphics::draw_text(buffer, w, 20, 10, "Ainux OS", accent_col);
    Graphics::draw_text(buffer, w, 110, 10, "File", fg_col);
    Graphics::draw_text(buffer, w, 160, 10, "Edit", fg_col);
    Graphics::draw_text(buffer, w, 210, 10, "Terminal", fg_col);
    
    // System Clock
    Graphics::draw_text(buffer, w, w - 150, 10, "May 06 18:08", fg_col);
    
    // 3. Dock
    draw_dock(buffer, w, h);
}

fn draw_dock(buffer: &mut [u32], w: usize, h: usize) {
    let num_icons = 6;
    let icon_size = 42;
    let pad = 12;
    let dock_w = num_icons * (icon_size + pad) + pad;
    let dock_h = 60;
    
    let dock_x = (w - dock_w) / 2;
    let dock_y = h - dock_h - 15;
    
    // Glassmorphic Dock Background
    Graphics::draw_rounded_rect(buffer, w, dock_x, dock_y, dock_w, dock_h, 15, 0xAA16161E);
    Graphics::draw_rounded_rect(buffer, w, dock_x, dock_y, dock_w, dock_h, 15, 0x20FFFFFF); // Inner Glow
    
    // Draw Icons (Symbolic)
    let icons = ["T", "F", "W", "S", "M", "C"];
    let colors = [0xFF7AA2F7, 0xFFBB9AF7, 0xFF7DCFFF, 0xFF9ECE6A, 0xFFE0AF68, 0xFFF7768E];
    
    for i in 0..num_icons {
        let x = dock_x + pad + i * (icon_size + pad);
        let y = dock_y + (dock_h - icon_size) / 2;
        
        Graphics::draw_rounded_rect(buffer, w, x, y, icon_size, icon_size, 8, colors[i]);
        Graphics::draw_text(buffer, w, x + 16, y + 16, icons[i], 0xFFFFFFFF);
    }
}

pub fn draw_text_overlay() {
    // Legacy hook - now handled in backbuffer for flicker-free rendering
}
