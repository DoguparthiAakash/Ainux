use spin::Mutex;
use core::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: u32,
    pub fg: u32,
    pub accent: u32, // Head/Header color
    pub root: u32,   // root@ainux color
    pub font_size: u8,
    pub scrollbar_enabled: bool,
    pub scroll_page_wise: bool,
}

pub static THEME: Mutex<Theme> = Mutex::new(Theme {
    bg: 0x001A1B26,     // Nvix/TokyoNight Dark (Stable UI Background)
    fg: 0x00C0CAF5,     // Soft Blue-White Text
    accent: 0x007AA2F7, // Neovim Accent Blue
    root: 0x009ECE6A,   // Neovim Accent Green
    font_size: 1,
    scrollbar_enabled: true,
    scroll_page_wise: false,
});

pub fn save_theme() {
    let data = {
        let t = THEME.lock();
        alloc::format!("bg={}\nfg={}\naccent={}\nroot={}\nfont_size={}\nscrollbar_enabled={}\nscroll_page_wise={}\n", 
            t.bg, t.fg, t.accent, t.root, t.font_size, t.scrollbar_enabled, t.scroll_page_wise)
    };
    
    if let Some(root) = crate::fs::vfs::ROOT.lock().clone() {
        let file_inode = match root.lookup("theme.conf") {
            Ok(inode) => inode,
            Err(_) => match root.create("theme.conf", crate::fs::vfs::FileType::File) {
                Ok(inode) => inode,
                Err(_) => return, // Failed to create
            }
        };
        
        if let Ok(handle) = file_inode.open(0) {
            let _ = handle.truncate();
            let _ = handle.write(data.as_bytes(), 0);
        }
    }
}

pub fn load_theme() {
    if let Some(root) = crate::fs::vfs::ROOT.lock().clone() {
        if let Ok(file_inode) = root.lookup("theme.conf") {
            if let Ok(handle) = file_inode.open(0) {
                let mut buf = alloc::vec![0u8; 512];
                if let Ok(bytes_read) = handle.read(&mut buf, 0) {
                    if let Ok(s) = core::str::from_utf8(&buf[..bytes_read]) {
                        let mut t = THEME.lock();
                        for line in s.lines() {
                            let parts: alloc::vec::Vec<&str> = line.split('=').collect();
                            if parts.len() == 2 {
                                let key = parts[0];
                                let val = parts[1];
                                match key {
                                    "bg" => if let Ok(v) = val.parse::<u32>() { t.bg = v; },
                                    "fg" => if let Ok(v) = val.parse::<u32>() { t.fg = v; },
                                    "accent" => if let Ok(v) = val.parse::<u32>() { t.accent = v; },
                                    "root" => if let Ok(v) = val.parse::<u32>() { t.root = v; },
                                    "font_size" => if let Ok(v) = val.parse::<u8>() { t.font_size = v; },
                                    "scrollbar_enabled" => if let Ok(v) = val.parse::<bool>() { t.scrollbar_enabled = v; },
                                    "scroll_page_wise" => if let Ok(v) = val.parse::<bool>() { t.scroll_page_wise = v; },
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// C FFI declarations
extern "C" {
    fn gfx_init(framebuffer_addr: *mut u8, width: u64, height: u64, pitch: u64, bpp: u8);
    fn c_draw_char(x: i32, y: i32, c: u32, fg_color: u32, bg_color: u32);
    fn c_clear_screen(color: u32);
}

pub fn fast_grid_clear(width: u32, height: u32, fg: u32, bg: u32, char_c: u32) {
    for y in 0..height {
        for x in 0..width {
            unsafe { c_draw_char(x as i32, y as i32, char_c, fg, bg); }
        }
    }
}

pub fn draw_tui_shadow(x: u32, y: u32, w: u32, h: u32) {
    for i in 0..w {
        unsafe { c_draw_char((x + i + 1) as i32, (y + h) as i32, ' ' as u32, 0, 0x000000); }
    }
    for i in 0..h {
        unsafe { c_draw_char((x + w) as i32, (y + i + 1) as i32, ' ' as u32, 0, 0x000000); }
    }
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
pub static AUTO_FLUSH: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(true);

pub static mut FB_CACHE_ADDR: u64 = 0;
pub static mut FB_CACHE_WIDTH: usize = 0;
pub static mut FB_CACHE_HEIGHT: usize = 0;
pub static mut FB_CACHE_PITCH: usize = 0;
pub static mut FB_CACHE_BPP: usize = 0;

// VGA Buffer Address in Higher Half (0xFFFFFFFF80000000 + 0xB8000)
pub const VGA_HHDM_ADDR: u64 = 0xFFFFFFFF800B8000;

pub fn flush_screen() {
    // Directly drawing to framebuffer, no flush needed.
}

pub fn flush_region(x: usize, y: usize, w: usize, h: usize) {
    // Directly drawing to framebuffer, no flush needed.
}

fn fast_clear(color: u32) {
    crate::cpu::without_interrupts(|| {
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
        
        let final_color = if color == 0 { THEME.lock().bg } else { color };
        let height = *FRAMEBUFFER_HEIGHT.lock();
        let pitch = *FRAMEBUFFER_PITCH.lock();
        let total_bytes = height * pitch;
        
        unsafe {
            let ptr = fb_addr as *mut u32;
            let total_pixels = total_bytes / 4;
            for i in 0..total_pixels {
                *ptr.add(i) = final_color;
            }
        }
    });
}

pub fn init() {
    let mut fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr != 0 {
        let width = *FRAMEBUFFER_WIDTH.lock() as u64;
        let height = *FRAMEBUFFER_HEIGHT.lock() as u64;
        let pitch = *FRAMEBUFFER_PITCH.lock() as u64;
        let bpp = *FRAMEBUFFER_BPP.lock();
        
        let size_bytes = (pitch * height) as usize;
        
        // Convert to virtual address if it's still a physical address
        if fb_addr < 0xFFFF800000000000 {
            unsafe { crate::mm::vmm::remap_hhdm_pages_wc(fb_addr, size_bytes); }
            fb_addr = crate::mm::vmm::phys_to_virt(fb_addr);
            *FRAMEBUFFER_ADDR.lock() = fb_addr;
        }
        
        unsafe { 
            FB_CACHE_ADDR = fb_addr;
            FB_CACHE_WIDTH = width as usize;
            FB_CACHE_HEIGHT = height as usize;
            FB_CACHE_PITCH = pitch as usize;
            FB_CACHE_BPP = bpp as usize;
            gfx_init(fb_addr as *mut u8, width, height, pitch, bpp); 
        }
        
        *CONSOLE_WIDTH.lock() = ((width / 8) as usize).saturating_sub(2);
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

pub static HISTORY: Mutex<alloc::vec::Vec<alloc::vec::Vec<ConsoleChar>>> = Mutex::new(alloc::vec::Vec::new());
pub static SCROLL_OFFSET: AtomicU32 = AtomicU32::new(0);
pub static SCREEN_GRID: Mutex<alloc::vec::Vec<ConsoleChar>> = Mutex::new(alloc::vec::Vec::new());

#[derive(Clone, Copy, Debug)]
pub struct ConsoleChar {
    pub c: char,
    pub fg: u32,
    pub bg: u32,
}

pub fn clear() {
    let bg = THEME.lock().bg;
    fast_clear(bg);
    // Reset cursor
    *CONSOLE_X.lock() = 0;
    *CONSOLE_Y.lock() = 0;
    SCROLL_OFFSET.store(0, Ordering::SeqCst);
    
    // Clear screen grid
    let w = *CONSOLE_WIDTH.lock();
    let h = *CONSOLE_HEIGHT.lock();
    let mut grid = SCREEN_GRID.lock();
    *grid = alloc::vec![ConsoleChar { c: ' ', fg: 0, bg: bg }; w * h];
}

pub fn set_resolution(w: u16, h: u16, bpp: u16) {
    unsafe {
        // DISPI_INDEX_ENABLE -> 0x0004, DISPI_DISABLED -> 0x0000
        core::arch::asm!("out dx, ax", in("dx") 0x01CE as u16, in("ax") 0x0004 as u16);
        core::arch::asm!("out dx, ax", in("dx") 0x01CF as u16, in("ax") 0x0000 as u16);
        
        // DISPI_INDEX_XRES -> 0x0001
        core::arch::asm!("out dx, ax", in("dx") 0x01CE as u16, in("ax") 0x0001 as u16);
        core::arch::asm!("out dx, ax", in("dx") 0x01CF as u16, in("ax") w);
        
        // DISPI_INDEX_YRES -> 0x0002
        core::arch::asm!("out dx, ax", in("dx") 0x01CE as u16, in("ax") 0x0002 as u16);
        core::arch::asm!("out dx, ax", in("dx") 0x01CF as u16, in("ax") h);
        
        // DISPI_INDEX_BPP -> 0x0003
        core::arch::asm!("out dx, ax", in("dx") 0x01CE as u16, in("ax") 0x0003 as u16);
        core::arch::asm!("out dx, ax", in("dx") 0x01CF as u16, in("ax") bpp);
        
        // DISPI_INDEX_ENABLE -> 0x0004, DISPI_ENABLED | DISPI_LFB_ENABLED -> 0x41
        core::arch::asm!("out dx, ax", in("dx") 0x01CE as u16, in("ax") 0x0004 as u16);
        core::arch::asm!("out dx, ax", in("dx") 0x01CF as u16, in("ax") 0x0041 as u16);
    }
    
    *FRAMEBUFFER_WIDTH.lock() = w as usize;
    *FRAMEBUFFER_HEIGHT.lock() = h as usize;
    *FRAMEBUFFER_PITCH.lock() = (w as usize) * (bpp as usize / 8);
    *FRAMEBUFFER_BPP.lock() = bpp as u8;
    
    init(); // Re-initialize with new dimensions
}

fn scroll_screen(lines: usize) {
    if lines == 0 { return; }
    
    let width = *CONSOLE_WIDTH.lock();
    let height = *CONSOLE_HEIGHT.lock();
    let bg = THEME.lock().bg;

    // 1. Move Grid top lines to History
    {
        let mut grid = SCREEN_GRID.lock();
        if !grid.is_empty() {
            let mut history = HISTORY.lock();
            for l in 0..lines {
                if l < grid.len() / width {
                    let mut line_vec = alloc::vec::Vec::with_capacity(width);
                    for i in 0..width {
                        line_vec.push(grid[l * width + i]);
                    }
                    history.push(line_vec);
                }
            }
            
            // Limit history to 2000 lines
            if history.len() > 2000 {
                let excess = history.len() - 2000;
                // history.drain(0..excess) is better, but since it's a vec we can do this
                for _ in 0..excess {
                    history.remove(0);
                }
            }
            
            let shift_elements = core::cmp::min(lines * width, grid.len());
            // Shift grid up
            for i in 0..(grid.len() - shift_elements) {
                grid[i] = grid[i + shift_elements];
            }
            // Clear last rows
            for i in (grid.len() - shift_elements)..grid.len() {
                grid[i] = ConsoleChar { c: ' ', fg: 0, bg };
            }
        }
    }

    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr == 0 {
        // VGA Text Mode Scroll
        let vga_buffer = VGA_HHDM_ADDR as *mut u16;
        let shift = core::cmp::min(lines, 24) * 80;
        unsafe {
            core::ptr::copy(vga_buffer.offset(shift as isize), vga_buffer, (80 * 24) - shift);
            let blank = 0x0F00 | b' ' as u16;
            for i in 0..shift {
                core::ptr::write_volatile(vga_buffer.offset(((80 * 24) - shift + i) as isize), blank);
            }
        }
        return;
    }
    
    let fb_height = *FRAMEBUFFER_HEIGHT.lock();
    let pitch = *FRAMEBUFFER_PITCH.lock(); // This is in bytes!
    
    let char_height = 12; 
    let shift_rows = core::cmp::min(lines * char_height, fb_height);
    let shift_pixels = (pitch / 4) * shift_rows;
    let total_pixels = (pitch / 4) * fb_height;

    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr != 0 {
        let addr = fb_addr as *mut u32;
        unsafe {
            if total_pixels > shift_pixels {
                core::ptr::copy(addr.add(shift_pixels), addr, total_pixels - shift_pixels);
            }
            let bottom_ptr = addr.add(total_pixels - shift_pixels);
            for i in 0..shift_pixels {
                *bottom_ptr.add(i) = bg;
            }
        }
    }
    // flush_screen() not needed because we draw directly to framebuffer
}

pub fn prepare_y_for_height(height: usize) -> usize {
    let mut y = CONSOLE_Y.lock();
    let console_h = *CONSOLE_HEIGHT.lock();
    
    if *y + height > console_h {
        let overflow = (*y + height) - console_h;
        scroll_screen(overflow);
        if *y >= overflow {
            *y -= overflow;
        } else {
            *y = 0;
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
    crate::cpu::without_interrupts(|| {
        let mut x = CONSOLE_X.lock();
        let mut y = CONSOLE_Y.lock();
        let width = *CONSOLE_WIDTH.lock();
        let height = *CONSOLE_HEIGHT.lock();

        // Update Grid
        if c != '\n' && c != '\r' && c != '\x08' {
            let mut grid = SCREEN_GRID.lock();
            if grid.is_empty() {
                *grid = alloc::vec![ConsoleChar { c: ' ', fg: 0, bg: THEME.lock().bg }; width * height];
            }
            let idx = (*y * width) + *x;
            if idx < grid.len() {
                grid[idx] = ConsoleChar { c, fg, bg };
            }
        }

        if c == '\n' {
            *x = 0;
            *y += 1;
            if *y >= height {
                *y = height - 1;
                scroll_screen(1);
            }
        } else if c == '\x08' {
            if *x > 0 { *x -= 1; }
        } else if c == '\r' {
            *x = 0;
        } else {
            let old_x = *x;
            let old_y = *y;
            let fb_addr = *FRAMEBUFFER_ADDR.lock();
            if fb_addr != 0 {
                unsafe { c_draw_char(*x as i32, *y as i32, c as u32, fg, bg); }
            } else {
                let vga_buffer = VGA_HHDM_ADDR as *mut u16;
                let offset = ((*y).min(24) * 80) + (*x).min(79);
                let vga_char = (c as u16) | (0x0F << 8);
                unsafe { core::ptr::write_volatile(vga_buffer.offset(offset as isize), vga_char); }
            }

            *x += 1;
            if *x >= width {
                *x = 0;
                *y += 1;
                if *y >= height {
                    *y = height - 1;
                    scroll_screen(1);
                }
            }
            flush_region(old_x * 8, old_y * 12, 8, 12);
        }
    });
}

pub fn put_str_colored(s: &str, fg: u32, bg: u32) {
    let was_auto = AUTO_FLUSH.swap(false, core::sync::atomic::Ordering::Relaxed);
    for c in s.chars() {
        put_char_colored(c, fg, bg);
    }
    if was_auto {
        AUTO_FLUSH.store(true, core::sync::atomic::Ordering::Relaxed);
        flush_screen();
    }
}

pub fn draw_cursor(color: u32) {
    let x = *CONSOLE_X.lock();
    let y = *CONSOLE_Y.lock();
    // If the caller passes 0 (erase), use theme background — never raw black
    let actual_color = if color == 0x00000000 { THEME.lock().bg } else { color };
    draw_rect_grid(x, y, 1, 1, 0, actual_color);
}

/// Draw a character at a specific char-grid coordinate without updating console state.
pub fn put_char_at(x: usize, y: usize, c: char, fg: u32, bg: u32) {
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr != 0 {
        unsafe { c_draw_char(x as i32, y as i32, c as u32, fg, bg); }
        flush_region(x * 8, y * 12, 8, 12);
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
    let fb_width = unsafe { FB_CACHE_WIDTH } as i64;
    let fb_height = unsafe { FB_CACHE_HEIGHT } as i64;
    let fb_pitch = unsafe { FB_CACHE_PITCH };
    let fb_bpp = unsafe { FB_CACHE_BPP };
    
    let fb_addr = unsafe { FB_CACHE_ADDR };
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
    let width = unsafe { FB_CACHE_WIDTH } as i64;
    let height = unsafe { FB_CACHE_HEIGHT } as i64;
    let fb_addr = unsafe { FB_CACHE_ADDR };
    let fb_pitch = unsafe { FB_CACHE_PITCH };
    let fb_bpp = unsafe { FB_CACHE_BPP };
    
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
    let bg = THEME.lock().bg;
    unsafe {
        c_draw_char(x as i32, y as i32, c as u32, fg, bg);
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
    copy_buffer_region(buffer, 0);
}

pub fn copy_buffer_region(buffer: &[u32], offset: usize) {
    let fb_virt = *FRAMEBUFFER_ADDR.lock(); 
    if fb_virt == 0 { return; }
    
    let ptr = fb_virt as *mut u32;
    unsafe {
        core::ptr::copy_nonoverlapping(buffer.as_ptr(), ptr.add(offset), buffer.len());
    }
}

pub fn draw_char_at(x: usize, y: usize, c: u32, fg: u32) {
    if *FRAMEBUFFER_ADDR.lock() != 0 {
        let bg = THEME.lock().bg; // Use theme bg so no black box artefacts
        unsafe {
            c_draw_char(x as i32, y as i32, c, fg, bg);
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

/// EMERGENCY LOCKLESS VIDEO ACCESS
/// Used ONLY during panics to avoid deadlocks.
pub unsafe fn emergency_put_str(s: &str) {
    let mut x_off = 0;
    let mut y_off = 2; // Row 2

    for c in s.chars() {
        if c == '\n' {
            x_off = 0;
            y_off += 1;
        } else {
             let vga_buffer = VGA_HHDM_ADDR as *mut u16;
             let offset = (y_off * 80) + x_off;
             if offset < 80 * 25 {
                let vga_char = (c as u16) | (0x4F << 8); // Red background for panic
                core::ptr::write_volatile(vga_buffer.offset(offset as isize), vga_char);
             }
             x_off += 1;
             if x_off >= 80 { x_off = 0; y_off += 1; }
        }
    }
}

pub unsafe fn panic_clear() {
    let vga_buffer = VGA_HHDM_ADDR as *mut u16;
    let vga_char = 0x4F00 | b' ' as u16; // Red background
    for i in 0..(80 * 25) {
        core::ptr::write_volatile(vga_buffer.offset(i as isize), vga_char);
    }
}

// ── UI Components: Cursor & Scrollbar ────────────────────────────────────────

pub static LAST_CURSOR_X: AtomicU32 = AtomicU32::new(0);
pub static LAST_CURSOR_Y: AtomicU32 = AtomicU32::new(0);
pub static CURSOR_BACKUP: Mutex<[u32; 256]> = Mutex::new([0; 256]); // 16x16 max cursor area
pub static HAS_BACKUP: Mutex<bool> = Mutex::new(false);

pub fn draw_mouse_cursor(x: i32, y: i32) {
    let fb_addr = *FRAMEBUFFER_ADDR.lock();
    if fb_addr == 0 { return; }

    crate::cpu::without_interrupts(|| {
        // 1. Restore old background if it exists
        if *HAS_BACKUP.lock() {
            let last_x = LAST_CURSOR_X.load(Ordering::SeqCst);
            let last_y = LAST_CURSOR_Y.load(Ordering::SeqCst);
            let backup = CURSOR_BACKUP.lock();
            
            for row in 0..16 {
                for col in 0..16 {
                    draw_pixel((last_x + col as u32) as i64, (last_y + row as u32) as i64, backup[row * 16 + col]);
                }
            }
        }

        // 2. Save new background
        {
            let mut backup = CURSOR_BACKUP.lock();
            let fb_w = *FRAMEBUFFER_WIDTH.lock() as i32;
            let fb_h = *FRAMEBUFFER_HEIGHT.lock() as i32;
            let fb_pitch = *FRAMEBUFFER_PITCH.lock();
            let ptr = fb_addr as *mut u32;
            let words_per_pitch = fb_pitch / 4;

            for row in 0..16 {
                let py = y + row as i32;
                for col in 0..16 {
                    let px = x + col as i32;
                    if px >= 0 && px < fb_w && py >= 0 && py < fb_h {
                        let offset = py as usize * words_per_pitch + px as usize;
                        unsafe {
                            backup[row * 16 + col] = *ptr.add(offset);
                        }
                    }
                }
            }
            *HAS_BACKUP.lock() = true;
            LAST_CURSOR_X.store(x as u32, Ordering::SeqCst);
            LAST_CURSOR_Y.store(y as u32, Ordering::SeqCst);
        }

        // 3. Draw new cursor (Arrow shape from bitmap)
        const CURSOR_BITMAP: [u8; 256] = [
            2,2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
            2,1,2,0,0,0,0,0,0,0,0,0,0,0,0,0,
            2,1,1,2,0,0,0,0,0,0,0,0,0,0,0,0,
            2,1,1,1,2,0,0,0,0,0,0,0,0,0,0,0,
            2,1,1,1,1,2,0,0,0,0,0,0,0,0,0,0,
            2,1,1,1,1,1,2,0,0,0,0,0,0,0,0,0,
            2,1,1,1,1,1,1,2,0,0,0,0,0,0,0,0,
            2,1,1,1,1,1,1,1,2,0,0,0,0,0,0,0,
            2,1,1,1,1,1,1,1,1,2,0,0,0,0,0,0,
            2,1,1,1,1,1,1,1,1,1,2,0,0,0,0,0,
            2,1,1,1,1,1,1,1,1,1,1,2,0,0,0,0,
            2,1,1,1,1,1,1,2,2,2,2,2,2,0,0,0,
            2,1,1,1,2,2,1,1,2,0,0,0,0,0,0,0,
            2,1,2,2,0,0,2,1,1,2,0,0,0,0,0,0,
            2,2,0,0,0,0,0,2,1,1,2,0,0,0,0,0,
            0,0,0,0,0,0,0,0,2,2,2,0,0,0,0,0,
        ];
        
        let color_fill = 0x00FFFFFF; // White inner
        let color_border = 0x00000000; // Black outline
        
        for row in 0..16 {
            for col in 0..16 {
                let px = CURSOR_BITMAP[row * 16 + col];
                if px == 1 {
                    draw_pixel((x + col as i32) as i64, (y + row as i32) as i64, color_fill);
                } else if px == 2 {
                    draw_pixel((x + col as i32) as i64, (y + row as i32) as i64, color_border);
                }
            }
        }
    });
}

pub fn refresh_screen() {
    crate::cpu::without_interrupts(|| {
        let offset = SCROLL_OFFSET.load(Ordering::SeqCst) as usize;
        let width = *CONSOLE_WIDTH.lock();
        let height = *CONSOLE_HEIGHT.lock();
        let theme = THEME.lock();
        let bg_color = theme.bg;
        drop(theme);
        
        fast_clear(bg_color);
        
        let history = HISTORY.lock();
        let total_hist = history.len();
        
        for y in 0..height {
            let line_to_draw = if offset > 0 {
                // History viewing
                let hist_idx = (total_hist as isize - offset as isize) + y as isize;
                if hist_idx >= 0 && (hist_idx as usize) < total_hist {
                    Some(&history[hist_idx as usize])
                } else {
                    None
                }
            } else {
                // Normal view (grid)
                None
            };

            if let Some(line) = line_to_draw {
                for x in 0..width {
                    let cell = line[x];
                    if cell.c != ' ' {
                        unsafe { c_draw_char(x as i32, y as i32, cell.c as u32, cell.fg, cell.bg); }
                    }
                }
            } else if offset == 0 || (total_hist as isize - offset as isize + y as isize) >= total_hist as isize {
                // Draw from current screen grid
                let grid = SCREEN_GRID.lock();
                let grid_y = if offset > 0 {
                    (y as isize - (total_hist as isize - offset as isize + height as isize - total_hist as isize)) as usize
                } else {
                    y
                };
                
                // Simplified: if offset is 0, just use y
                let actual_grid_y = if offset == 0 { y } else {
                    let rel_y = (total_hist as isize - offset as isize) + y as isize;
                    if rel_y >= total_hist as isize {
                        (rel_y - total_hist as isize) as usize
                    } else {
                        9999 // skip
                    }
                };

                if actual_grid_y < height {
                    for x in 0..width {
                        let cell = grid[actual_grid_y * width + x];
                        if cell.c != ' ' {
                            unsafe { c_draw_char(x as i32, y as i32, cell.c as u32, cell.fg, cell.bg); }
                        }
                    }
                }
            }
        }
    });
}

pub fn draw_scrollbar() {
    let theme = THEME.lock();
    if !theme.scrollbar_enabled { return; }
    
    let fb_w = *FRAMEBUFFER_WIDTH.lock();
    let fb_h = *FRAMEBUFFER_HEIGHT.lock();
    if fb_w == 0 { return; }

    let sb_w = 12;
    let sb_x = (fb_w - sb_w) as i64;
    
    let bg = 0x002A2B36; // Industrial Dark Track
    let thumb_color = theme.accent;
    drop(theme);

    // Track
    fill_rect(sb_x, 0, sb_w as i64, fb_h as i64, bg);
    
    // Calculate thumb position
    let history_len = HISTORY.lock().len();
    if history_len == 0 {
         // Full thumb if no history
         fill_rect(sb_x + 2, 2, sb_w as i64 - 4, fb_h as i64 - 4, thumb_color);
         return;
    }

    let total_lines = history_len + *CONSOLE_HEIGHT.lock();
    let thumb_h = ((fb_h as usize * *CONSOLE_HEIGHT.lock()) / total_lines).max(30);
    
    let scroll_pos = SCROLL_OFFSET.load(Ordering::SeqCst) as usize;
    let max_scroll = history_len;
    
    // Invert scroll_pos for top-down scrollbar: offset 0 means bottom (end of history)
    let inverted_pos = max_scroll - scroll_pos;
    let thumb_y = (inverted_pos * (fb_h - thumb_h)) / max_scroll;

    fill_rect(sb_x + 2, thumb_y as i64, sb_w as i64 - 4, thumb_h as i64, thumb_color);
}