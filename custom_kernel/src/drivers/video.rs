use spin::Mutex;

// C FFI declarations
extern "C" {
    fn gfx_init(framebuffer_addr: *mut u8, width: u64, height: u64, pitch: u64);
    fn c_draw_char(x: i32, y: i32, c: u8, fg_color: u32, bg_color: u32);
    fn c_clear_screen(color: u32);
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
pub static FRAMEBUFFER_BPP: spin::Mutex<usize> = spin::Mutex::new(0);

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
    
    // Total u32 words
    let total = (fb_pitch / 4) * fb_height;
    let ptr = fb_addr as *mut u32;
    
    unsafe {
        for i in 0..total {
            *ptr.add(i) = color;
        }
    }
}

pub fn init() {
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr != 0 {
        let width = *FRAMEBUFFER_WIDTH.lock() as u64;
        let height = *FRAMEBUFFER_HEIGHT.lock() as u64;
        let pitch = *FRAMEBUFFER_PITCH.lock() as u64;
        
        unsafe { gfx_init(fb_addr as *mut u8, width, height, pitch); }
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

pub fn clear() {
    fast_clear(0x00000000);
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

pub fn put_char(c: char) {
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
    } else if c == '\x08' {  // Backspace (Non-destructive move left)
        if *x > 0 {
            *x -= 1;
        }
    } else if c == '\r' {
        *x = 0;
    } else {
        let fb_addr = *FRAMEBUFFER_ADDR.lock();
        if fb_addr != 0 {
            // Pixel Graphics Mode
            unsafe {
                c_draw_char(*x as i32, *y as i32, c as u8, 0xFFFFFFFF, 0x00000000);
            }
        } else {
            // Legacy VGA Text Mode Fallback
            let vga_buffer = VGA_HHDM_ADDR as *mut u16;
            // Ensure bounds are safe
            let safe_x = (*x).min(79);
            let safe_y = (*y).min(24);
            let offset = (safe_y * 80) + safe_x;
            let vga_char = (c as u16) | (0x0F << 8); // White on black
            unsafe {
                core::ptr::write_volatile(vga_buffer.offset(offset as isize), vga_char);
            }
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

pub fn draw_cursor(color: u32) {
    let x = *CONSOLE_X.lock();
    let y = *CONSOLE_Y.lock();
    // 8x12 font
    let draw_x = (x * 8) as i64;
    let draw_y = (y * 12) as i64;
    // Draw 8x12 block
    draw_rect(draw_x, draw_y, 8, 12, color);
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
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr == 0 { return; }

    let ptr = fb_addr as *mut u32;

    for row in 0..h {
        let draw_y = y + row;
        if draw_y < 0 || draw_y >= fb_height { continue; }
        
        for col in 0..w {
            let draw_x = x + col;
            if draw_x < 0 || draw_x >= fb_width { continue; }
            
            let offset = (draw_y as usize * fb_pitch / 4) + draw_x as usize;
            unsafe {
                *ptr.add(offset) = color;
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
    
    if fb_addr == 0 { return; }
    if x < 0 || x >= width || y < 0 || y >= height { return; }

    let offset = (y as usize * fb_pitch / 4) + x as usize;
    let ptr = fb_addr as *mut u32;
    unsafe {
        *ptr.add(offset) = color;
    }
}

pub fn fill_rect(x: i64, y: i64, w: i64, h: i64, color: u32) {
    draw_rect(x, y, w, h, color);
}

pub fn draw_char_raw(x: usize, y: usize, c: char, fg: u32) {
    unsafe {
        c_draw_char(x as i32, y as i32, c as u8, fg, 0); // Transparent BG? Assume 0 is transparent/ignored or we don't care
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