
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
             bb.resize(w * h, 0); 
         }
    }

    pub fn handle_mouse(mx: isize, my: isize, buttons: u8) {
        let mut wins = WINDOWS.lock();
        let mouse_point = crate::gui::rect::Point { x: mx, y: my };
        
        let mut focused_idx: Option<usize> = None;
        let mut dragging_occurred = false;

        for (i, win) in wins.iter_mut().enumerate().rev() {
            if win.is_minimized { continue; }
            
            let win_rect = crate::gui::rect::Rect::new(win.x, win.y, win.width, win.height);
            let title_rect = crate::gui::rect::Rect::new(win.x, win.y, win.width, 24);

            if buttons & 1 != 0 {
                if !dragging_occurred && title_rect.contains(mouse_point) {
                    if win.get_close_button_rect().contains(mouse_point) {
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
                win.x = mx - (win.width / 2) as isize;
                win.y = my - 12;
            }
        }

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

        let (mx, my) = crate::drivers::mouse::get_position();
        let buttons = crate::drivers::mouse::get_buttons();
        Self::handle_mouse(mx, my, buttons);

        crate::gui::desktop::draw_desktop(bb, w, h);
        
        let wins = WINDOWS.lock();
        for win in wins.iter() {
            win.draw(bb, w);
        }
        
        crate::gui::desktop::draw_overlay(bb, w, h);
        
        // Premium Cursor (Translucent Rounded Arrow)
        Graphics::draw_rounded_rect(bb, w, mx as usize, my as usize, 12, 12, 2, 0xCCFFFFFF);
        Graphics::draw_rect_to_buffer(bb, w, mx as usize + 2, my as usize + 2, 8, 8, 0xFF000000);
        
        unsafe {
             let fb_addr = *crate::drivers::video::FRAMEBUFFER_ADDR.lock();
             let fb_ptr = fb_addr as *mut u32;
             core::ptr::copy_nonoverlapping(bb.as_ptr(), fb_ptr, w * h);
        }
    }
}
