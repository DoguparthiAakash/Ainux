
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

    pub fn handle_mouse(mx: isize, my: isize, buttons: u8) {
        let mut wins = WINDOWS.lock();
        let mouse_point = crate::gui::rect::Point { x: mx, y: my };
        
        // 1. Check for Dragging/Focus (from top to bottom)
        let mut focused_idx: Option<usize> = None;
        let mut dragging_occurred = false;

        for (i, win) in wins.iter_mut().enumerate().rev() {
            if win.is_minimized { continue; }
            
            let win_rect = crate::gui::rect::Rect::new(win.x, win.y, win.width, win.height);
            let title_rect = crate::gui::rect::Rect::new(win.x, win.y, win.width, 24);

            if buttons & 1 != 0 {
                // Left Click
                if !dragging_occurred && title_rect.contains(mouse_point) {
                    // Check buttons first
                    if win.get_close_button_rect().contains(mouse_point) {
                         // Close handled in next phase? Or here.
                         continue; 
                    }
                    
                    win.dragging = true;
                    focused_idx = Some(i);
                    dragging_occurred = true;
                } else if !dragging_occurred && win_rect.contains(mouse_point) {
                    focused_idx = Some(i);
                }
            } else {
                win.dragging = false;
            }

            if win.dragging {
                // Update position (simplified dx/dy is needed, using absolute for now)
                // In a real OS you'd track the offset from mouse to window corner
                win.x = mx - (win.width / 2) as isize;
                win.y = my - 12;
            }
        }

        // 2. Move focused window to top
        if let Some(idx) = focused_idx {
            let win = wins.remove(idx);
            wins.push(win);
        }
    }
    
    pub fn render() {
        let h = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock();
        let w = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock();
        
        let mut bb_lock = BACKBUFFER.lock();
        if bb_lock.len() != w * h {
            bb_lock.resize(w * h, 0);
        }
        let bb = &mut *bb_lock;

        // Mouse State
        let (mx, my) = crate::drivers::mouse::get_position();
        let buttons = unsafe { crate::drivers::mouse::get_buttons() }; // Need to expose buttons
        Self::handle_mouse(mx, my, buttons);

        crate::gui::desktop::draw_desktop(bb, w, h);
        
        let wins = WINDOWS.lock();
        for win in wins.iter() {
            win.draw(bb, w);
        }
        
        crate::gui::desktop::draw_overlay(bb, w, h);
        
        // Modern Cursor (Translucent Arrow)
        Graphics::draw_rect_to_buffer(bb, w, mx as usize, my as usize, 8, 8, 0xFFFFFFFF);
        
        unsafe {
             let fb_addr = *crate::drivers::video::FRAMEBUFFER_ADDR.lock();
             let fb_pitch = *crate::drivers::video::FRAMEBUFFER_PITCH.lock();
             let fb_ptr = fb_addr as *mut u32;
             let stride = fb_pitch / 4;
             
             for y in 0..h {
                 for x in 0..w {
                     let val = bb[y * w + x];
                     *fb_ptr.add(y * stride + x) = val;
                 }
             }
        }

        crate::gui::desktop::draw_text_overlay();
    }
}
