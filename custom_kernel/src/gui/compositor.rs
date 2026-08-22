
use alloc::vec::Vec;
use crate::gui::window::Window;
use crate::gui::graphics::{Color, Graphics};
use spin::Mutex;

const MAX_WINDOWS: usize = 16;

pub static WINDOWS: Mutex<Vec<Window>> = Mutex::new(Vec::new());

// BACKBUFFER removed to save 1.9MB of heap space

pub struct Compositor;

impl Compositor {
    pub fn add_window(window: Window) {
        let mut wins = WINDOWS.lock();
        if wins.len() < MAX_WINDOWS {
            wins.push(window);
        }
    }
    
    pub fn init() {
         // BACKBUFFER allocation removed to prevent OOM
    }

    pub fn handle_mouse(mx: isize, my: isize, buttons: u8) {
        let mut wins = WINDOWS.lock();
        let mouse_point = crate::gui::rect::Point { x: mx, y: my };
        
        let mut focused_idx: Option<usize> = None;
        let mut dragging_occurred = false;
        
        let mut to_remove: Option<usize> = None;
        let mut to_toggle_max: Option<usize> = None;
        let mut to_toggle_min: Option<usize> = None;

        for (i, win) in wins.iter_mut().enumerate().rev() {
            if win.is_minimized {
                // If minimized, check if clicked on dock? For now, we skip.
                continue;
            }
            
            let win_rect = crate::gui::rect::Rect::new(win.x, win.y, win.width, win.height);
            let title_rect = crate::gui::rect::Rect::new(win.x, win.y, win.width, 24);

            if buttons & 1 != 0 {
                // Left Click
                if !dragging_occurred && title_rect.contains(mouse_point) {
                    if win.get_close_button_rect().contains(mouse_point) {
                        to_remove = Some(i);
                    } else if win.get_max_button_rect().contains(mouse_point) {
                        to_toggle_max = Some(i);
                    } else if win.get_min_button_rect().contains(mouse_point) {
                        to_toggle_min = Some(i);
                    } else {
                        if !win.is_maximized {
                            win.dragging = true;
                        }
                    }
                    focused_idx = Some(i);
                    dragging_occurred = true;
                } else if !dragging_occurred && win_rect.contains(mouse_point) {
                    focused_idx = Some(i);
                    dragging_occurred = true; // Consume click for this frame
                }
            } else {
                win.dragging = false;
            }

            if win.dragging {
                win.x = mx - (win.width / 2) as isize;
                win.y = my - 12;
            }
        }

        if let Some(idx) = to_remove {
            wins.remove(idx);
            return;
        }

        if let Some(idx) = to_toggle_max {
            let win = &mut wins[idx];
            win.is_maximized = !win.is_maximized;
            if win.is_maximized {
                win.orig_x = win.x;
                win.orig_y = win.y;
                win.orig_width = win.width;
                win.orig_height = win.height;
                
                win.x = 0;
                win.y = 24; // Below top bar
                win.width = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock();
                win.height = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock() - 24 - 40; // minus topbar and dock
            } else {
                win.x = win.orig_x;
                win.y = win.orig_y;
                win.width = win.orig_width;
                win.height = win.orig_height;
            }
            return;
        }

        if let Some(idx) = to_toggle_min {
            wins[idx].is_minimized = true;
            return;
        }

        // Move focused window to top
        if let Some(idx) = focused_idx {
            // Only move if not already on top, though popping and pushing is fine
            if idx != wins.len() - 1 {
                let win = wins.remove(idx);
                wins.push(win);
            }
        }
    }
    
    pub fn render() {
        let h = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock();
        let w = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock();
        
        // Mouse State
        let (mx, my) = crate::drivers::mouse::get_position();
        let buttons = crate::drivers::mouse::get_buttons();
        Self::handle_mouse(mx, my, buttons);
        
        let wins = WINDOWS.lock();
        unsafe {
             let fb_addr = *crate::drivers::video::FRAMEBUFFER_ADDR.lock();
             let fb_pitch = *crate::drivers::video::FRAMEBUFFER_PITCH.lock();
             let fb_ptr = fb_addr as *mut u32;
             let stride = fb_pitch / 4;
             
             let bb = core::slice::from_raw_parts_mut(fb_ptr, stride * h);

             crate::gui::desktop::draw_desktop(bb, w, h);
             
             for win in wins.iter() {
                 win.draw(bb, stride); // pass stride instead of w
             }
             
             crate::gui::desktop::draw_overlay(bb, w, h, mx, my);
             
             // --- INDUSTRIAL SCROLLBAR (Right Side) ---
             let sb_w = 12;
             let sb_h = h - 24 - 80; // Minus top bar and dock area
             let sb_x = w - sb_w - 4;
             let sb_y = 30;
             Graphics::draw_rect_to_buffer(bb, stride, sb_x, sb_y, sb_w, sb_h, 0x40FFFFFF); // Track
             Graphics::draw_rect_to_buffer(bb, stride, sb_x + 2, sb_y + 10, sb_w - 4, 40, 0xCCFFFFFF); // Thumb
             
             // --- INDUSTRIAL ARROW CURSOR ---
             let cursor_bitmap: [u16; 16] = [
                 0b1000000000000000,
                 0b1100000000000000,
                 0b1110000000000000,
                 0b1111000000000000,
                 0b1111100000000000,
                 0b1111110000000000,
                 0b1111111000000000,
                 0b1111111100000000,
                 0b1111111110000000,
                 0b1111111111000000,
                 0b1111111111100000,
                 0b1111111100000000,
                 0b1111011110000000,
                 0b1110001110000000,
                 0b1100000111000000,
                 0b1000000011100000,
             ];
             
             for (ry, row) in cursor_bitmap.iter().enumerate() {
                 for rx in 0..16 {
                     if (row >> (15 - rx)) & 1 != 0 {
                         Graphics::plot_pixel_unchecked(bb, stride, mx as usize + rx, my as usize + ry, 0xFFFFFFFF);
                     }
                 }
             }
        }
        
        for win in wins.iter() {
            win.draw_text();
        }

        crate::gui::desktop::draw_text_overlay(w, h);
    }
}
