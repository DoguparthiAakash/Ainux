use crate::gui::graphics::Graphics;
use crate::drivers::video;

pub fn draw_desktop(buffer: &mut [u32], w: usize, h: usize) {
    // GNOME Default Wallpaper (Solid Dark Grey/Blue)
    let gnome_bg = 0xFF243447;
    for i in 0..buffer.len() {
        buffer[i] = gnome_bg;
    }
}

pub fn draw_overlay(buffer: &mut [u32], w: usize, h: usize, mx: isize, my: isize) {
    // 1. GNOME Top Bar (Solid Black/Dark Grey)
    let topbar_col = 0xFF1E1E1E; 
    let stride = w;
    Graphics::draw_rect_to_buffer(buffer, stride, 0, 0, w, 28, topbar_col);
    
    // Top Bar Bottom Border (Subtle highlight)
    Graphics::draw_rect_to_buffer(buffer, stride, 0, 28, w, 1, 0xFF333333);
    
    // 2. GNOME Dash (Centered at bottom)
    draw_dock(buffer, w, h, mx, my);
}

fn draw_dock(buffer: &mut [u32], w: usize, h: usize, mx: isize, my: isize) {
    let num_icons = 4; // GNOME Dash apps
    let icon_size = 48;
    let pad = 12;
    let dock_w = num_icons * (icon_size + pad) + pad;
    let dock_h = icon_size + (pad * 2);
    
    let dock_x = (w - dock_w) / 2;
    let dock_y = h - dock_h - 16;
    
    // GNOME Dash Background (Dark rounded rect)
    let dock_bg = 0xAA000000;
    Graphics::draw_rect_to_buffer(buffer, w, dock_x, dock_y, dock_w, dock_h, dock_bg);
    
    // Draw Icons (Terminal, Files, Settings, Web)
    let colors = [
        0xFF333333, // Terminal (Dark)
        0xFF1A73E8, // Files (Blue)
        0xFF808080, // Settings (Grey)
        0xFFE34F26, // Web/Firefox (Orange)
    ];
    
    for i in 0..num_icons {
        let x = dock_x + pad + i * (icon_size + pad);
        let y = dock_y + pad;
        
        let mut color = colors[i];
        
        // Hover highlight
        if mx >= x as isize && mx < (x + icon_size) as isize && 
           my >= y as isize && my < (y + icon_size) as isize {
            color = Graphics::blend_colors(color, 0x40FFFFFF);
        }
        
        Graphics::draw_rect_to_buffer(buffer, w, x, y, icon_size, icon_size, color);
    }
}

// Called after buffer flip to draw text directly to VRAM
pub fn draw_text_overlay(w: usize, h: usize) {
    let text_col = 0xFFFFFFFF;
    
    // Left: Activities
    draw_string_at(20, 6, "Activities", text_col);
    
    // Center: Date & Time
    let t = crate::drivers::rtc::read_time();
    let month_names = ["", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let m_str = if t.month as usize <= 12 && t.month > 0 { month_names[t.month as usize] } else { "??" };
    let am_pm = if t.hours >= 12 { "PM" } else { "AM" };
    let hour_12 = if t.hours == 0 { 12 } else if t.hours > 12 { t.hours - 12 } else { t.hours };
    let date_str = alloc::format!("{} {:02}  {:02}:{:02} {}", m_str, t.day, hour_12, t.minutes, am_pm);
    
    let date_x = (w - (date_str.len() * 8)) / 2;
    draw_string_at(date_x, 6, &date_str, text_col);
    
    // Right: Status (Battery, Network, Power)
    let batt = crate::cpu::acpi::get_battery_percentage();
    let temp = crate::cpu::acpi::get_temperature();
    let status_str = alloc::format!("[WIFI] [{}°C] [{}%] [*]", temp, batt);
    let status_x = w - (status_str.len() * 8) - 20;
    draw_string_at(status_x, 6, &status_str, text_col);
}

fn draw_string_at(x: usize, y: usize, text: &str, color: u32) {
    let mut cx = x;
    for c in text.chars() {
        video::draw_char_raw(cx, y, c, color);
        cx += 8;
    }
}

pub fn run() {
    crate::gui::compositor::Compositor::init();
    
    // Launch default apps
    crate::apps::gui_apps::launch_task_manager();
    crate::apps::gui_apps::launch_file_manager();
    
    loop {
        crate::gui::compositor::Compositor::render();
        unsafe { core::arch::asm!("hlt"); }
    }
}
