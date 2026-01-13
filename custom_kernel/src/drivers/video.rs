use spin::Mutex;
use crate::FRAMEBUFFER_REQUEST;

// C FFI declarations
extern "C" {
    fn gfx_init(framebuffer_addr: *mut u8, width: u64, height: u64, pitch: u64);
    fn c_draw_char(x: i32, y: i32, c: u8, fg_color: u32, bg_color: u32);
    fn c_clear_screen(color: u32);
}

pub static CONSOLE_X: spin::Mutex<usize> = spin::Mutex::new(0);
pub static CONSOLE_Y: spin::Mutex<usize> = spin::Mutex::new(0);
pub static CONSOLE_WIDTH: spin::Mutex<usize> = spin::Mutex::new(0);
pub static CONSOLE_HEIGHT: spin::Mutex<usize> = spin::Mutex::new(0);

// Raw Framebuffer Info (Public for GUI)
pub static FRAMEBUFFER_ADDR: spin::Mutex<u64> = spin::Mutex::new(0);
pub static FRAMEBUFFER_WIDTH: spin::Mutex<usize> = spin::Mutex::new(0);
pub static FRAMEBUFFER_HEIGHT: spin::Mutex<usize> = spin::Mutex::new(0);
pub static FRAMEBUFFER_PITCH: spin::Mutex<usize> = spin::Mutex::new(0);
pub static FRAMEBUFFER_BPP: spin::Mutex<usize> = spin::Mutex::new(0);


fn fast_clear(color: u32) {
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    let fb_pitch = *FRAMEBUFFER_PITCH.lock(); // bytes
    let fb_height = *FRAMEBUFFER_HEIGHT.lock();
    
    if fb_addr == 0 { return; }
    
    // Total u32 words
    // Assumes pitch is multiple of 4
    let total = (fb_pitch / 4) * fb_height;
    let ptr = fb_addr as *mut u32;
    
    unsafe {
        for i in 0..total {
            *ptr.add(i) = color;
        }
    }
}

pub fn init() {
    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
            let width = framebuffer.width() as u64;
            let height = framebuffer.height() as u64;
            let buffer = framebuffer.addr();
            let pitch = framebuffer.pitch() as u64;

            // Initialize C graphics
            unsafe {
                gfx_init(buffer as *mut u8, width, height, pitch);
            }
            
            // Store console dimensions (in characters)
            *CONSOLE_WIDTH.lock() = (width / 8) as usize;
            *CONSOLE_HEIGHT.lock() = (height / 12) as usize;
            
            // Store Raw Info
            *FRAMEBUFFER_ADDR.lock() = buffer as u64;
            *FRAMEBUFFER_WIDTH.lock() = width as usize;
            *FRAMEBUFFER_HEIGHT.lock() = height as usize;
            *FRAMEBUFFER_PITCH.lock() = pitch as usize;
            *FRAMEBUFFER_BPP.lock() = 32; // Assuming 32-bit

            // Debug Resolution
            {
               use core::fmt::Write;
               let mut serial = crate::drivers::serial::SERIAL.lock();
               let _ = write!(serial, "Video Init: W={} H={} P={}\n", width, height, pitch);
            }

            // Clear screen to black on startup
            fast_clear(0x00000000);
        }
    }
}

pub fn clear() {
    fast_clear(0x00000000);
    // Reset cursor
    *CONSOLE_X.lock() = 0;
    *CONSOLE_Y.lock() = 0;
}

fn scroll_screen() {
    let height = *FRAMEBUFFER_HEIGHT.lock();
    let pitch = *FRAMEBUFFER_PITCH.lock();
    let addr = *FRAMEBUFFER_ADDR.lock() as *mut u8;

    if addr.is_null() { return; }

    // Assume 12px font height
    let char_height = 12; 
    let row_bytes = pitch * char_height;
    let total_bytes = pitch * height;

    if total_bytes <= row_bytes { return; }

    unsafe {
        // Shift up
        // src: addr + row_bytes
        // dst: addr
        // len: total_bytes - row_bytes
        core::ptr::copy(addr.add(row_bytes), addr, total_bytes - row_bytes);
        
        // Clear bottom
        core::ptr::write_bytes(addr.add(total_bytes - row_bytes), 0, row_bytes);
    }
}

pub fn put_char(c: char) {
    // Mirror to Serial (0x3F8) for debugging
    unsafe {
        core::arch::asm!("out dx, al", in("dx") 0x3F8, in("al") c as u8, options(nomem, nostack, preserves_flags));
    }

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
        unsafe {
            c_draw_char(*x as i32, *y as i32, c as u8, 0xFFFFFFFF, 0x00000000);
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
    let draw_x = x * 8;
    let draw_y = y * 12;
    // Draw 8x12 block
    draw_rect(draw_x, draw_y, 8, 12, color);
}

pub fn put_str(s: &str) {
    for c in s.chars() {
        put_char(c);
    }
}

pub fn draw_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    let fb_width = *FRAMEBUFFER_WIDTH.lock();
    let fb_height = *FRAMEBUFFER_HEIGHT.lock();
    let fb_pitch = *FRAMEBUFFER_PITCH.lock();
    let fb_addr = *FRAMEBUFFER_ADDR.lock();

    if fb_addr == 0 { return; }

    let ptr = fb_addr as *mut u32;

    for row in 0..h {
        let draw_y = y + row;
        if draw_y >= fb_height { break; }
        
        for col in 0..w {
            let draw_x = x + col;
            if draw_x >= fb_width { break; }
            
            // Calculate offset. Assumes 32 BPP (4 bytes).
            // Pitch is in bytes.
            let offset = (draw_y * fb_pitch / 4) + draw_x;
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
    
    if x < 0 || x >= width || y < 0 || y >= height { return; }

    unsafe {
        c_draw_char(x as i32, y as i32, 0, color, 0); // Hack: using c_draw_char logic? 
        // Wait, c_draw_char draws a CHARACTER. We need direct pixel access.
        // Let's implement direct pixel access reusing the DrawRect logic but for 1 pixel?
        // Or better, just write to the pointer.
    }
    
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    let fb_pitch = *FRAMEBUFFER_PITCH.lock(); // bytes
    
    if fb_addr == 0 { return; }
    
    let offset = (y as usize * fb_pitch / 4) + x as usize;
    let ptr = fb_addr as *mut u32;
    unsafe {
        *ptr.add(offset) = color;
    }
}

pub fn fill_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    // Reusing draw_rect logic but renamed/exposed properly
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