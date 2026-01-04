use spin::Mutex;
use crate::FRAMEBUFFER_REQUEST;

// C FFI declarations
extern "C" {
    fn gfx_init(framebuffer_addr: *mut u8, width: u64, height: u64, pitch: u64);
    fn c_draw_char(x: i32, y: i32, c: u8, fg_color: u32, bg_color: u32);
    fn c_clear_screen(color: u32);
}

static CONSOLE_X: spin::Mutex<usize> = spin::Mutex::new(0);
static CONSOLE_Y: spin::Mutex<usize> = spin::Mutex::new(0);
static CONSOLE_WIDTH: spin::Mutex<usize> = spin::Mutex::new(0);
static CONSOLE_HEIGHT: spin::Mutex<usize> = spin::Mutex::new(0);

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

            // Clear screen to black on startup
            unsafe { c_clear_screen(0x00000000); }
        }
    }
}

pub fn clear() {
    unsafe {
        c_clear_screen(0x00000000);
    }
    // Reset cursor
    *CONSOLE_X.lock() = 0;
    *CONSOLE_Y.lock() = 0;
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
            // TODO: scroll
        }
    } else if c == '\x08' {  // Backspace
        if *x > 0 {
            *x -= 1;
            unsafe {
                c_draw_char(*x as i32, *y as i32, b' ', 0xFFFFFFFF, 0x00000000);
            }
        }
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
                // TODO: scroll
            }
        }
    }
}

pub fn put_str(s: &str) {
    for c in s.chars() {
        put_char(c);
    }
}