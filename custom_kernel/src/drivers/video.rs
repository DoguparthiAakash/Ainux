use spin::Mutex;
use core::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: u32,
    pub fg: u32,
    pub accent: u32, // Head/Header color
    pub root: u32,   // root@ainux color
    pub font_size: u8,
}

pub static THEME: Mutex<Theme> = Mutex::new(Theme {
    bg: 0x00000000,
    fg: 0xFFFFFFFF,
    accent: 0x00AAAAFF, // Sky blue header
    root: 0x00FF5555,   // Soft red root
    font_size: 1,
});

// C FFI declarations
extern "C" {
    fn gfx_init(framebuffer_addr: *mut u8, width: u64, height: u64, pitch: u64, bpp: u8);
    fn c_draw_char(x: i32, y: i32, c: u32, fg_color: u32, bg_color: u32);
    fn c_clear_screen(color: u32);
    
    // Zig-based TUI Core
    pub fn fast_grid_clear(width: u32, height: u32, fg: u32, bg: u32, char: u32);
    pub fn draw_tui_shadow(x: u32, y: u32, w: u32, h: u32);
}

pub static CONSOLE_X: spin::Mutex<usize> = spin::Mutex::new(0);
pub static CONSOLE_Y: spin::Mutex<usize> = spin::Mutex::new(0);
pub static CONSOLE_WIDTH: spin::Mutex<usize> = spin::Mutex::new(80);
pub static CONSOLE_HEIGHT: spin::Mutex<usize> = spin::Mutex::new(25);

// Raw Framebuffer Info (Public for GUI)
pub static FRAMEBUFFER_ADDR: spin::Mutex<u64> = spin::Mutex::new(0);
pub static FRAMEBUFFER_WIDTH: spin::Mutex<usize> = spin::Mutex::new(0);
pub static FRAMEBUFFER_HEIGHT: spin::Mutex<usize> = spin::Mutex::new(0);
pub static FRAMEBUFFER_PITCH: spin::Mutex<usize> = spin::Mutex::new(0);
pub static FRAMEBUFFER_BPP: spin::Mutex<u8> = spin::Mutex::new(0);
pub static FRAMEBUFFER_TYPE: spin::Mutex<u8> = spin::Mutex::new(0);

// VGA Buffer Address in Higher Half (0xFFFFFFFF80000000 + 0xB8000)
pub const VGA_HHDM_ADDR: u64 = 0xFFFFFFFF800B8000;

fn fast_clear(color: u32) {
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr == 0 {
        // VGA Text mode clear
        let vga_buffer = VGA_HHDM_ADDR as *mut u16;
        let vga_char = 0x0F00 | b' ' as u16; // Black background, white space
        unsafe {
            for i in 0..(80 * 25) {
                core::ptr::write_volatile(vga_buffer.offset(i as isize), vga_char);
            }
        }
        return;
    }
    
    let fb_pitch = *FRAMEBUFFER_PITCH.lock(); // bytes
    let fb_height = *FRAMEBUFFER_HEIGHT.lock();
    
    // Fallback to theme BG if color is 0 (assuming default clear)
    let final_color = if color == 0 { THEME.lock().bg } else { color };

    // Total u32 words
    let total = (fb_pitch / 4) * fb_height;
    let ptr = fb_addr as *mut u32;
    
    unsafe {
        for i in 0..total {
            *ptr.add(i) = final_color;
        }
    }
}

pub fn init() {
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr != 0 {
        let width = *FRAMEBUFFER_WIDTH.lock() as u64;
        let height = *FRAMEBUFFER_HEIGHT.lock() as u64;
        let pitch = *FRAMEBUFFER_PITCH.lock() as u64;
        let bpp = *FRAMEBUFFER_BPP.lock();
        
        unsafe { gfx_init(fb_addr as *mut u8, width, height, pitch, bpp); }
        *CONSOLE_WIDTH.lock() = (width / 8) as usize;
        *CONSOLE_HEIGHT.lock() = (height / 12) as usize;
        fast_clear(0x00000000);
    } else {
        // VGA Text mode dimensions
        *CONSOLE_WIDTH.lock() = 80;
        *CONSOLE_HEIGHT.lock() = 25;
        fast_clear(0);
    }
}

/// Return (width, height) of the framebuffer, or (0, 0) if text mode.
pub fn get_resolution() -> (usize, usize) {
    let w = *FRAMEBUFFER_WIDTH.lock();
    let h = *FRAMEBUFFER_HEIGHT.lock();
    (w, h)
}

pub fn clear() {
    let bg = THEME.lock().bg;
    fast_clear(bg);
    // Reset cursor
    *CONSOLE_X.lock() = 0;
    *CONSOLE_Y.lock() = 0;
}

fn scroll_screen() {
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr == 0 {
        // VGA Text Mode Scroll
        let vga_buffer = VGA_HHDM_ADDR as *mut u16;
        unsafe {
            // Shift up by 1 line (80 chars)
            core::ptr::copy(vga_buffer.offset(80), vga_buffer, 80 * 24);
            // Clear last line
            let blank = 0x0F00 | b' ' as u16;
            for i in 0..80 {
                core::ptr::write_volatile(vga_buffer.offset((80 * 24) + i), blank);
            }
        }
        return;
    }
    
    let height = *FRAMEBUFFER_HEIGHT.lock();
    let pitch = *FRAMEBUFFER_PITCH.lock();
    let addr = fb_addr as *mut u8;

    let char_height = 12; 
    let row_bytes = pitch * char_height;
    let total_bytes = pitch * height;

    if total_bytes <= row_bytes { return; }

    unsafe {
        core::ptr::copy(addr.add(row_bytes), addr, total_bytes - row_bytes);
        core::ptr::write_bytes(addr.add(total_bytes - row_bytes), 0, row_bytes);
    }
}

pub fn prepare_y_for_height(height: usize) -> usize {
    let mut y = CONSOLE_Y.lock();
    let console_h = *CONSOLE_HEIGHT.lock();
    
    if *y + height > console_h {
        let overflow = (*y + height) - console_h;
        for _ in 0..overflow {
            scroll_screen();
            if *y > 0 {
                *y -= 1;
            }
        }
    }
    *y
}

pub fn put_char(c: char) {
    let theme = THEME.lock();
    let fg = theme.fg;
    let bg = theme.bg;
    drop(theme);
    put_char_colored(c, fg, bg);
}

pub fn put_char_colored(c: char, fg: u32, bg: u32) {
    let mut x = CONSOLE_X.lock();
    let mut y = CONSOLE_Y.lock();
    let width = *CONSOLE_WIDTH.lock();
    let height = *CONSOLE_HEIGHT.lock();

    if c == '\n' {
        *x = 0;
        *y += 1;
        if *y >= height {
            *y = height - 1;
            scroll_screen();
        }
    } else if c == '\x08' {
        if *x > 0 { *x -= 1; }
    } else if c == '\r' {
        *x = 0;
    } else {
        let fb_addr = *FRAMEBUFFER_ADDR.lock();
        if fb_addr != 0 {
            unsafe { c_draw_char(*x as i32, *y as i32, c as u32, fg, bg); }
        } else {
            let vga_buffer = VGA_HHDM_ADDR as *mut u16;
            let offset = ((*y).min(24) * 80) + (*x).min(79);
            // Default white for VGA for now
            let vga_char = (c as u16) | (0x0F << 8);
            unsafe { core::ptr::write_volatile(vga_buffer.offset(offset as isize), vga_char); }
        }

        *x += 1;
        if *x >= width {
            *x = 0;
            *y += 1;
            if *y >= height {
                *y = height - 1;
                scroll_screen();
            }
        }
    }
}

pub fn put_str_colored(s: &str, fg: u32, bg: u32) {
    for c in s.chars() {
        put_char_colored(c, fg, bg);
    }
}

pub fn draw_cursor(color: u32) {
    let x = *CONSOLE_X.lock();
    let y = *CONSOLE_Y.lock();
    // draw a small rectangle at the cursor position
    draw_rect_grid(x, y, 1, 1, 0, color);
}

/// Draw a character at a specific char-grid coordinate without updating console state.
pub fn put_char_at(x: usize, y: usize, c: char, fg: u32, bg: u32) {
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr != 0 {
        unsafe { c_draw_char(x as i32, y as i32, c as u32, fg, bg); }
    } else {
        let vga_buffer = VGA_HHDM_ADDR as *mut u16;
        let offset = (y.min(24) * 80) + x.min(79);
        let vga_char = (c as u16) | (0x0F << 8);
        unsafe { core::ptr::write_volatile(vga_buffer.offset(offset as isize), vga_char); }
    }
}

/// Draw a filled/outlined rectangle in the character grid.
pub fn draw_rect_grid(x: usize, y: usize, w: usize, h: usize, fg: u32, bg: u32) {
    for cy in y..(y + h) {
        for cx in x..(x + w) {
            put_char_at(cx, cy, ' ', fg, bg);
        }
    }
}

pub fn put_str_at(x: usize, y: usize, s: &str, fg: u32, bg: u32) {
    for (i, c) in s.chars().enumerate() {
        put_char_at(x + i, y, c, fg, bg);
    }
}

pub fn put_str(s: &str) {
    for c in s.chars() {
        put_char(c);
    }
}

pub fn draw_rect(x: i64, y: i64, w: i64, h: i64, color: u32) {
    let fb_width = *FRAMEBUFFER_WIDTH.lock() as i64;
    let fb_height = *FRAMEBUFFER_HEIGHT.lock() as i64;
    let fb_pitch = *FRAMEBUFFER_PITCH.lock();
    let fb_bpp = *FRAMEBUFFER_BPP.lock() as usize;
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr == 0 { return; }

    let ptr = fb_addr as *mut u8;
    let bytes_per_pixel = fb_bpp / 8;

    for row in 0..h {
        let draw_y = y + row;
        if draw_y < 0 || draw_y >= fb_height { continue; }
        
        for col in 0..w {
            let draw_x = x + col;
            if draw_x < 0 || draw_x >= fb_width { continue; }
            
            let offset = (draw_y as usize * fb_pitch) + (draw_x as usize * bytes_per_pixel);
            unsafe {
                if fb_bpp == 32 {
                    *(ptr.add(offset) as *mut u32) = color;
                } else if fb_bpp == 24 {
                    *ptr.add(offset) = (color & 0xFF) as u8;
                    *ptr.add(offset + 1) = ((color >> 8) & 0xFF) as u8;
                    *ptr.add(offset + 2) = ((color >> 16) & 0xFF) as u8;
                }
            }
        }
    }
}


pub fn put_int(mut val: usize) {
    if val == 0 {
        put_char('0');
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 0;
    while val > 0 {
        buf[i] = (val % 10) as u8 + b'0';
        val /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        put_char(buf[i] as char);
    }
}

pub fn draw_pixel(x: i64, y: i64, color: u32) {
    let width = *FRAMEBUFFER_WIDTH.lock() as i64;
    let height = *FRAMEBUFFER_HEIGHT.lock() as i64;
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    let fb_pitch = *FRAMEBUFFER_PITCH.lock();
    let fb_bpp = *FRAMEBUFFER_BPP.lock() as usize;
    
    if fb_addr == 0 { return; }
    if x < 0 || x >= width || y < 0 || y >= height { return; }

    let bytes_per_pixel = fb_bpp / 8;
    let offset = (y as usize * fb_pitch) + (x as usize * bytes_per_pixel);
    let ptr = fb_addr as *mut u8;
    unsafe {
        if fb_bpp == 32 {
            *(ptr.add(offset) as *mut u32) = color;
        } else if fb_bpp == 24 {
            *ptr.add(offset) = (color & 0xFF) as u8;
            *ptr.add(offset + 1) = ((color >> 8) & 0xFF) as u8;
            *ptr.add(offset + 2) = ((color >> 16) & 0xFF) as u8;
        }
    }
}

pub fn fill_rect(x: i64, y: i64, w: i64, h: i64, color: u32) {
    draw_rect(x, y, w, h, color);
}

pub fn draw_char_raw(x: usize, y: usize, c: char, fg: u32) {
    unsafe {
        c_draw_char(x as i32, y as i32, c as u32, fg, 0); // Transparent BG? Assume 0 is transparent/ignored or we don't care
    }
}

pub fn draw_line(x0: i64, y0: i64, x1: i64, y1: i64, color: u32) {
    let mut x0 = x0;
    let mut y0 = y0;
    let dx = if x1 > x0 { x1 - x0 } else { x0 - x1 };
    let dy = if y1 > y0 { y1 - y0 } else { y0 - y1 };
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = (if dx > dy { dx } else { -dy }) / 2;
    let mut e2;

    loop {
        draw_pixel(x0, y0, color);
        if x0 == x1 && y0 == y1 { break; }
        e2 = err;
        if e2 > -dx { err -= dy; x0 += sx; }
        if e2 < dy { err += dx; y0 += sy; }
    }
}

pub fn draw_circle(cx: i64, cy: i64, r: i64, color: u32) {
    let mut x = 0;
    let mut y = r;
    let mut d = 3 - 2 * r;

    draw_circle_octants(cx, cy, x, y, color);
    while y >= x {
        x += 1;
        if d > 0 {
            y -= 1;
            d = d + 4 * (x - y) + 10;
        } else {
            d = d + 4 * x + 6;
        }
        draw_circle_octants(cx, cy, x, y, color);
    }
}

fn draw_circle_octants(cx: i64, cy: i64, x: i64, y: i64, color: u32) {
    draw_pixel(cx + x, cy + y, color);
    draw_pixel(cx - x, cy + y, color);
    draw_pixel(cx + x, cy - y, color);
    draw_pixel(cx - x, cy - y, color);
    draw_pixel(cx + y, cy + x, color);
    draw_pixel(cx - y, cy + x, color);
    draw_pixel(cx + y, cy - x, color);
    draw_pixel(cx - y, cy - x, color);
}

pub fn copy_buffer(buffer: &[u32]) {
    let fb_addr = *FRAMEBUFFER_ADDR.lock(); 
    if fb_addr == 0 { return; }
    
    // Safety: We assume buffer size matches framebuffer size for full screen updates.
    // Or we should check bounds. For max speed, we trust the caller in this kernel context.
    let ptr = fb_addr as *mut u32;
    unsafe {
        core::ptr::copy_nonoverlapping(buffer.as_ptr(), ptr, buffer.len());
    }
}

pub fn draw_char_at(x: usize, y: usize, c: u32, fg: u32) {
    if *FRAMEBUFFER_ADDR.lock() != 0 {
        unsafe {
            c_draw_char(x as i32, y as i32, c, fg, 0x00000000);
        }
    }
}

pub fn draw_tui_box(x: usize, y: usize, w: usize, h: usize, fg: u32) {
    if w < 2 || h < 2 { return; }
    
    // Corners
    draw_char_at(x, y, 0x2554, fg); // ╔
    draw_char_at(x + w - 1, y, 0x2557, fg); // ╗
    draw_char_at(x, y + h - 1, 0x255A, fg); // ╚
    draw_char_at(x + w - 1, y + h - 1, 0x255D, fg); // ╝
    
    // Horizontal lines
    for i in 1..(w - 1) {
        draw_char_at(x + i, y, 0x2550, fg); // ═
        draw_char_at(x + i, y + h - 1, 0x2550, fg); // ═
    }
    
    // Vertical lines
    for j in 1..(h - 1) {
        draw_char_at(x, y + j, 0x2551, fg); // ║
        draw_char_at(x + w - 1, y + j, 0x2551, fg); // ║
    }
}

pub fn draw_tui_title_box(x: usize, y: usize, w: usize, h: usize, title: &str, fg: u32) {
    draw_tui_box(x, y, w, h, fg);
    if title.len() > 0 && title.len() < w - 2 {
        let start_x = x + (w - title.len()) / 2;
        for (i, c) in title.chars().enumerate() {
            draw_char_at(start_x + i, y, c as u32, fg);
        }
    }
}