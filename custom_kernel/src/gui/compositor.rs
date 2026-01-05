
use alloc::vec::Vec;
use crate::gui::window::Window;
use crate::gui::graphics::{Color, Graphics};
use spin::Mutex;

const MAX_WINDOWS: usize = 16;

pub static WINDOWS: Mutex<Vec<Window>> = Mutex::new(Vec::new());

pub static BACKBUFFER: Mutex<Vec<u32>> = Mutex::new(Vec::new());

pub struct Compositor;

impl Compositor {
    pub fn add_window(window: Window) {
        let mut wins = WINDOWS.lock();
        if wins.len() < MAX_WINDOWS {
            wins.push(window);
        }
    }
    
    pub fn init() {
         let w = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock();
         let h = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock();
         let mut bb = BACKBUFFER.lock();
         if w > 0 && h > 0 {
             bb.resize(w * h, 0); // Init backbuffer
         }
    }
    
    pub fn render() {
        let h = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock();
        let w = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock();
        
        // Lock Backbuffer
        let mut bb_lock = BACKBUFFER.lock();
        // Just in case it wasn't init
        if bb_lock.len() != w * h {
            bb_lock.resize(w * h, 0);
        }
        let bb = &mut *bb_lock;

        // 1. Clear / Desktop Background (Draw to BB)
        let desk_col = Color { r: 0, g: 128, b: 128, a: 255 }.to_u32();
        for pixel in bb.iter_mut() { *pixel = desk_col; }
        
        // 2. Taskbar
        // Graphics::draw_rect_to_buffer(bb, w, 0, h - 40, w, 40, 0xFFC0C0C0);
        
        // 3. Draw Windows -> BB
        let wins = WINDOWS.lock();
        for win in wins.iter() {
            win.draw(bb, w);
        }
        
        // 4. Cursor -> BB
        let (mx, my) = crate::drivers::mouse::get_position();
        Graphics::draw_rect_to_buffer(bb, w, mx as usize, my as usize, 10, 10, 0xFFFF0000);
        
        // 5. Present (Copy BB -> VRAM)
        unsafe {
             let fb_addr = *crate::drivers::video::FRAMEBUFFER_ADDR.lock();
             let fb_pitch = *crate::drivers::video::FRAMEBUFFER_PITCH.lock();
             let fb_ptr = fb_addr as *mut u32;
             
             // Optimized (or semi-optimized) copy
             // Actually pitch is in bytes. If BPP=32, stride = pitch / 4.
             let stride = fb_pitch / 4;
             
             for y in 0..h {
                 for x in 0..w {
                     let val = bb[y * w + x];
                     *fb_ptr.add(y * stride + x) = val;
                 }
             }
        }
    }
}
