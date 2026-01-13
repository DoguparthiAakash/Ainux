use crate::gui::graphics::Graphics;
use crate::drivers::video;

pub fn draw_desktop(buffer: &mut [u32], w: usize, h: usize) {
    // 1. Wallpaper (Teal)
    let teal = 0xFF008080;
    for i in 0..buffer.len() {
        buffer[i] = teal;
    }
}

pub fn draw_overlay(buffer: &mut [u32], w: usize, h: usize) {
    // 2. Top Bar (Translucent White)
    // Rect: 0, 0, w, 24
    let bar_col = 0xCCFFFFFF; 
    let stride = w;
    Graphics::draw_rect_to_buffer(buffer, stride, 0, 0, w, 24, bar_col);
    
    // Border
    Graphics::draw_rect_to_buffer(buffer, stride, 0, 24, w, 1, 0xFF808080);
    
    // 3. Dock
    draw_dock(buffer, w, h);
}

fn draw_dock(buffer: &mut [u32], w: usize, h: usize) {
    let num_icons = 6;
    let icon_size = 48;
    let pad = 16;
    let dock_w = num_icons * (icon_size + pad) + pad;
    let dock_h = 70;
    
    let dock_x = (w - dock_w) / 2;
    let dock_y = h - dock_h - 10;
    
    // Translucent White Dock Background
    let color = 0x40FFFFFF;
    Graphics::draw_rect_to_buffer(buffer, w, dock_x, dock_y, dock_w, dock_h, color);
    
    // Draw Icons (Placeholders)
    let colors = [0xFFFF0000, 0xFF00FF00, 0xFF0000FF, 0xFFFFFF00, 0xFF00FFFF, 0xFF800080];
    
    for i in 0..num_icons {
        let x = dock_x + pad + i * (icon_size + pad);
        let y = dock_y + (dock_h - icon_size) / 2;
        
        // Draw Icon Box
        Graphics::draw_rect_to_buffer(buffer, w, x, y, icon_size, icon_size, colors[i]);
    }
}

// Called after buffer flip to draw text directly to VRAM
pub fn draw_text_overlay() {
    // Top Bar Text
    let black = 0xFF000000;
    
    // "Mithl"
    draw_string_at(20, 5, "Mithl", black);
    draw_string_at(80, 5, "File", black);
    draw_string_at(130, 5, "Edit", black);
    draw_string_at(180, 5, "View", black);
}

fn draw_string_at(x: usize, y: usize, text: &str, color: u32) {
    let mut cx = x;
    for c in text.chars() {
        video::draw_char_raw(cx, y, c, color);
        cx += 8;
    }
}
