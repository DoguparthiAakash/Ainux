use alloc::vec::Vec;
use crate::drivers::video;
use crate::drivers::mouse;
use crate::drivers::keyboard;
use alloc::string::{String, ToString};
use super::window::{Window, WindowState};


// Utility to draw strings to a generic buffer
fn draw_string_to_buffer(buf: &mut [u32], buf_w: usize, buf_h: usize, x: i32, y: i32, s: &str, fg: u32, clip: Option<&crate::gui::clip::Rect>) {
    let mut curr_x = x;
    for c in s.chars() {
        if curr_x >= 0 && (curr_x + 8) <= buf_w as i32 && y >= 0 && (y + 16) <= buf_h as i32 {
            if let Some(cl) = clip {
                if curr_x >= cl.x + cl.w || curr_x + 8 <= cl.x || y >= cl.y + cl.h || y + 16 <= cl.y {
                    curr_x += 8;
                    continue;
                }
            }
            video::draw_char_to_buffer(buf, buf_w, buf_h, curr_x as usize, y as usize, c, fg);
        }
        curr_x += 8;
    }
}

fn fill_rect_buffer(buf: &mut [u32], buf_w: usize, buf_h: usize, rx: i32, ry: i32, rw: i32, rh: i32, color: u32, clip: Option<&crate::gui::clip::Rect>) {
    let mut x1 = rx.max(0);
    let mut y1 = ry.max(0);
    let mut x2 = (rx + rw).min(buf_w as i32);
    let mut y2 = (ry + rh).min(buf_h as i32);

    if let Some(c) = clip {
        x1 = x1.max(c.x);
        y1 = y1.max(c.y);
        x2 = x2.min(c.x + c.w);
        y2 = y2.min(c.y + c.h);
    }
    
    if x1 >= x2 || y1 >= y2 { return; }
    
    for sy in y1..y2 {
        for sx in x1..x2 {
            buf[(sy * buf_w as i32 + sx) as usize] = color;
        }
    }
}

pub struct Compositor {
    pub dirty_rects: Vec<crate::gui::clip::Rect>,
    pub full_redraw: bool,
    pub backbuffer: Vec<u32>,
    pub width: usize,
    pub height: usize,
    pub windows: Vec<Window>,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_left_down: bool,
    pub mouse_right_down: bool,
    pub dragged_window: Option<usize>,
    pub start_menu_open: bool,
    pub active_submenu: Option<u32>,
    pub resized_window: Option<usize>,
    pub desktop_icons: alloc::vec::Vec<DesktopIcon>,
    pub dragged_icon: Option<usize>,
    pub icon_drag_offset_x: i32,
    pub icon_drag_offset_y: i32,
    pub last_clicked_icon: Option<usize>,
    pub last_click_ticks: u64,
    pub context_menu_open: bool,
    pub context_menu_x: i32,
    pub context_menu_y: i32,
    pub context_menu_target: Option<usize>,
    pub btn_close: crate::gui::bmp::BmpImage,
    pub btn_min: crate::gui::bmp::BmpImage,
    pub btn_max: crate::gui::bmp::BmpImage,
    pub active_edge: Option<u8>,
    pub last_redraw_ticks: u64,
    pub last_frame_rects: alloc::vec::Vec<crate::gui::clip::Rect>,
}

#[derive(Clone)]
pub struct DesktopIcon {
    pub name: alloc::string::String,
    pub x: i32,
    pub y: i32,
    pub bmp: crate::gui::bmp::BmpImage,
}

impl Compositor {
    pub fn new(w: usize, h: usize) -> Self {
        let mut desktop_icons = alloc::vec::Vec::new();
        
        let mut add_icon = |name: &str, raw_data: &[u8]| {
            if let Some(bmp) = crate::gui::bmp::BmpImage::parse(raw_data) {
                let count = desktop_icons.len() as i32;
                let col = count / 6; // Max 6 icons per column
                let row = count % 6;
                desktop_icons.push(DesktopIcon {
                    name: alloc::string::String::from(name),
                    x: 20 + col * 80,
                    y: 20 + row * 80,
                    bmp,
                });
            }
        };
        
        add_icon("Terminal", include_bytes!("../../icons/terminal.bmp"));
        add_icon("Settings", include_bytes!("../../icons/settings.bmp"));
        add_icon("File Mgr", include_bytes!("../../icons/fileexplorer.bmp"));
        add_icon("Games", include_bytes!("../../icons/games.bmp"));
        add_icon("Task Mgr", include_bytes!("../../icons/discmanagement.bmp"));
        add_icon("Sys Mon", include_bytes!("../../icons/controlcenter.bmp"));
        add_icon("Paint", include_bytes!("../../icons/images.bmp"));
        add_icon("Clock", include_bytes!("../../icons/clock.bmp"));
        add_icon("Calendar", include_bytes!("../../icons/calander.bmp"));
        add_icon("Tetris", include_bytes!("../../icons/tetris.bmp"));
        add_icon("Pong", include_bytes!("../../icons/pong.bmp"));
        add_icon("Mines", include_bytes!("../../icons/mine.bmp"));
        add_icon("Browser", include_bytes!("../../icons/bookmark.bmp"));
        
        Self {
            dirty_rects: alloc::vec::Vec::new(),
            full_redraw: true,
            backbuffer: alloc::vec![0; w * h],
            width: w,
            height: h,
            windows: Vec::new(),
            mouse_x: w as i32 / 2,
            mouse_y: h as i32 / 2,
            mouse_left_down: false,
            mouse_right_down: false,
            dragged_window: None,
            start_menu_open: false,
            active_submenu: None,
            resized_window: None,
            desktop_icons,
            dragged_icon: None,
            icon_drag_offset_x: 0,
            icon_drag_offset_y: 0,
            last_clicked_icon: None,
            last_click_ticks: 0,
            context_menu_open: false,
            context_menu_x: 0,
            context_menu_y: 0,
            context_menu_target: None,
            btn_close: crate::gui::bmp::BmpImage::parse(include_bytes!("../../icons/close.bmp")).unwrap_or_else(|| crate::gui::bmp::BmpImage { width: 20, height: 20, data: alloc::vec![0; 400] }),
            btn_min: crate::gui::bmp::BmpImage::parse(include_bytes!("../../icons/minimize.bmp")).unwrap_or_else(|| crate::gui::bmp::BmpImage { width: 20, height: 20, data: alloc::vec![0; 400] }),
            btn_max: crate::gui::bmp::BmpImage::parse(include_bytes!("../../icons/fullscreen.bmp")).unwrap_or_else(|| crate::gui::bmp::BmpImage { width: 20, height: 20, data: alloc::vec![0; 400] }),
            active_edge: None,
            last_redraw_ticks: 0,
            last_frame_rects: alloc::vec::Vec::new(),
        }
    }

    pub fn add_window(&mut self, w: Window) {
        self.windows.push(w);
    }

    pub fn invalidate(&mut self, rect: crate::gui::clip::Rect) {
        self.dirty_rects.push(rect);
    }
    
    pub fn invalidate_all(&mut self) {
        self.full_redraw = true;
    }

    pub fn draw(&mut self) {
        if self.full_redraw {
            self.full_redraw = false;
            self.dirty_rects.clear();
            self.dirty_rects.push(crate::gui::clip::Rect { x: 0, y: 0, w: self.width as i32, h: self.height as i32 });
        } else if self.dirty_rects.is_empty() {
            return;
        }

        let mut final_rects: alloc::vec::Vec<crate::gui::clip::Rect> = alloc::vec::Vec::new();
        for r in &self.dirty_rects {
            let mut new_pieces = alloc::vec![*r];
            for existing in &final_rects {
                let mut next_pieces = alloc::vec::Vec::new();
                for p in new_pieces {
                    next_pieces.extend(p.subtract(existing));
                }
                new_pieces = next_pieces;
            }
            final_rects.extend(new_pieces);
        }
        self.dirty_rects.clear();
        
        // 1. Draw scene to clean backbuffer for each dirty rect
        for r in &final_rects {
            let r_x = r.x.max(0);
            let r_y = r.y.max(0);
            let r_w = r.w.min(self.width as i32 - r_x);
            let r_h = r.h.min(self.height as i32 - r_y);
            if r_w > 0 && r_h > 0 {
                self.draw_scene(Some(&crate::gui::clip::Rect { x: r_x, y: r_y, w: r_w, h: r_h }));
            }
        }

        // 2. Temporarily draw cursor into backbuffer
        let cursor_x = self.mouse_x;
        let cursor_y = self.mouse_y;
        let mut cursor_backup = alloc::vec![0u32; 32 * 32];
        for row in 0..32 {
            let py = cursor_y + row;
            if py < 0 || py >= self.height as i32 { continue; }
            for col in 0..32 {
                let px = cursor_x + col;
                if px < 0 || px >= self.width as i32 { continue; }
                cursor_backup[(row * 32 + col) as usize] = self.backbuffer[(py * self.width as i32 + px) as usize];
            }
        }

        // Get cursor bitmap and blend
        let state = *crate::drivers::video::CURRENT_CURSOR.lock();
        let bitmap = match state {
            crate::drivers::video::CursorState::Normal => &crate::drivers::cursors::CURSOR_PTR,
            crate::drivers::video::CursorState::ResizeH => &crate::drivers::cursors::CURSOR_RESIZE_H,
            crate::drivers::video::CursorState::ResizeV => &crate::drivers::cursors::CURSOR_RESIZE_V,
            crate::drivers::video::CursorState::Wait => &crate::drivers::cursors::CURSOR_WAIT,
            crate::drivers::video::CursorState::Text => &crate::drivers::cursors::CURSOR_TEXT,
        };

        for row in 0..32 {
            let py = cursor_y + row;
            if py < 0 || py >= self.height as i32 { continue; }
            for col in 0..32 {
                let px = cursor_x + col;
                if px < 0 || px >= self.width as i32 { continue; }
                
                let pixel = bitmap[(row * 32 + col) as usize];
                let alpha = (pixel >> 24) & 0xFF;
                
                if alpha > 0 {
                    let idx = (py * self.width as i32 + px) as usize;
                    if alpha == 255 {
                        self.backbuffer[idx] = pixel & 0x00FFFFFF;
                    } else {
                        let bg = self.backbuffer[idx];
                        let bg_r = (bg >> 16) & 0xFF;
                        let bg_g = (bg >> 8) & 0xFF;
                        let bg_b = bg & 0xFF;
                        
                        let fg_r = (pixel >> 16) & 0xFF;
                        let fg_g = (pixel >> 8) & 0xFF;
                        let fg_b = pixel & 0xFF;
                        
                        let inv_alpha = 255 - alpha;
                        let out_r = ((fg_r * alpha) + (bg_r * inv_alpha)) / 255;
                        let out_g = ((fg_g * alpha) + (bg_g * inv_alpha)) / 255;
                        let out_b = ((fg_b * alpha) + (bg_b * inv_alpha)) / 255;
                        
                        self.backbuffer[idx] = (out_r << 16) | (out_g << 8) | out_b;
                    }
                }
            }
        }

        // 3. Blit all dirty rects from backbuffer to hardware framebuffer
        for r in &final_rects {
            let r_x = r.x.max(0);
            let r_y = r.y.max(0);
            let r_w = r.w.min(self.width as i32 - r_x);
            let r_h = r.h.min(self.height as i32 - r_y);
            if r_w > 0 && r_h > 0 {
                let offset = r_y as usize * self.width + r_x as usize;
                video::blit_buffer_opaque(&self.backbuffer[offset..], r_x, r_y, r_w, r_h, self.width as i32);
            }
        }

        // 4. Restore backbuffer clean pixels
        for row in 0..32 {
            let py = cursor_y + row;
            if py < 0 || py >= self.height as i32 { continue; }
            for col in 0..32 {
                let px = cursor_x + col;
                if px < 0 || px >= self.width as i32 { continue; }
                self.backbuffer[(py * self.width as i32 + px) as usize] = cursor_backup[(row * 32 + col) as usize];
            }
        }
    }

    fn draw_scene(&mut self, clip_opt: Option<&crate::gui::clip::Rect>) {


        // 1. Clear background (Desktop Wallpaper)
        let bg_color = *crate::gui::settings_app::WALLPAPER_COLOR.lock();
        // Fill entire screen
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 0, 0, self.width as i32, self.height as i32, bg_color | 0xFF000000, clip_opt);

        // 1.5 Draw Desktop Icons
        for icon in &self.desktop_icons {
            // Draw BMP (scaled to 48x48 max)
            let draw_w = core::cmp::min(icon.bmp.width, 48);
            let draw_h = core::cmp::min(icon.bmp.height, 48);
            let scale_x_mul = (icon.bmp.width * 1024) / draw_w;
            let scale_y_mul = (icon.bmp.height * 1024) / draw_h;

            for dy in 0..draw_h {
                let sy = icon.y + dy as i32;
                if sy < 0 || sy >= self.height as i32 { continue; }
                let src_y = (dy * scale_y_mul) / 1024;
                
                for dx in 0..draw_w {
                    let sx = icon.x + dx as i32;
                    if sx < 0 || sx >= self.width as i32 { continue; }
                    let src_x = (dx * scale_x_mul) / 1024;
                    
                    let color = icon.bmp.data[src_y * icon.bmp.width + src_x];
                    if color & 0xFF000000 != 0 && color != 0xFFFFFFFF {
                        if let Some(c) = clip_opt {
                            if sx >= c.x && sx < c.x + c.w && sy >= c.y && sy < c.y + c.h {
                                self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                            }
                        } else {
                            self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                        }
                    }
                }
            }
            // Draw text
            let text_x = icon.x + (draw_w as i32 / 2) - (icon.name.len() as i32 * 4);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, text_x, icon.y + draw_h as i32 + 5, &icon.name, 0xFFFFFF, clip_opt);
        }

        // 2. Draw Windows from bottom to top
        for w in &mut self.windows {
            if w.state == WindowState::Minimized { continue; }
            
            let border = 1;
            let title_h = 24;
            
            // Draw window frame outline (SkyOS Blue/Silver Theme)
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 
                w.x - border, w.y - title_h - border, w.width + border * 2, w.height + title_h + border * 2, 0x1A477A, clip_opt);
                
            // Draw title bar (SkyOS Gradient Blue)
            for dy in 0..title_h {
                let r = 0x5B - ((0x5B - 0x1A) * dy / title_h);
                let g = 0x9B - ((0x9B - 0x47) * dy / title_h);
                let b = 0xD5 - ((0xD5 - 0x7A) * dy / title_h);
                let color = (r << 16) | (g << 8) | b;
                fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 
                    w.x, w.y - title_h + dy, w.width, 1, color as u32, clip_opt);
            }
                
            // Draw title string with drop shadow effect
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, w.x + 9, w.y - title_h + 7, &w.title, 0x000000, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, w.x + 8, w.y - title_h + 6, &w.title, 0xFFFFFF, clip_opt);

            // Close Button
            let close_bx = w.x + w.width - 22;
            let close_by = w.y - 22;
            
            let mut draw_btn = |bmp: &crate::gui::bmp::BmpImage, bx: i32, by: i32| {
                let draw_w = core::cmp::min(bmp.width, 20);
                let draw_h = core::cmp::min(bmp.height, 20);
                let scale_x_mul = (bmp.width * 1024) / draw_w;
                let scale_y_mul = (bmp.height * 1024) / draw_h;

                for dy in 0..draw_h {
                    let sy = by + dy as i32;
                    let src_y = (dy * scale_y_mul) / 1024;
                    for dx in 0..draw_w {
                        let sx = bx + dx as i32;
                        let src_x = (dx * scale_x_mul) / 1024;
                        let color = bmp.data[src_y * bmp.width + src_x];
                        if color & 0xFF000000 != 0 && color != 0xFFFFFFFF { 
                            if sy >= 0 && sy < self.height as i32 && sx >= 0 && sx < self.width as i32 {
                                if let Some(c) = clip_opt {
                                    if sx >= c.x && sx < c.x + c.w && sy >= c.y && sy < c.y + c.h {
                                        self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                                    }
                                } else {
                                    self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                                }
                            }
                        }
                    }
                }
            };
            
            draw_btn(&self.btn_close, close_bx, close_by);

            // Maximize Button
            let max_bx = w.x + w.width - 44;
            let max_by = w.y - 22;
            draw_btn(&self.btn_max, max_bx, max_by);

            // Minimize Button
            let min_bx = w.x + w.width - 66;
            let min_by = w.y - 22;
            draw_btn(&self.btn_min, min_bx, min_by);
            
            // Removed hardcoded w.app.draw(...)

            
            // Draw Resize Handle (bottom right)
            let res_x = w.x + w.width - 12;
            let res_y = w.y + w.height - 12;
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, res_x, res_y, 12, 12, 0x808080, clip_opt);
            
            // Draw window content (blit its buffer)
            let mut bx1 = w.x.max(0);
            let mut by1 = w.y.max(0);
            let mut bx2 = (w.x + w.width).min(self.width as i32);
            let mut by2 = (w.y + w.height).min(self.height as i32);
            if let Some(c) = clip_opt {
                bx1 = bx1.max(c.x);
                by1 = by1.max(c.y);
                bx2 = bx2.min(c.x + c.w);
                by2 = by2.min(c.y + c.h);
            }
            if bx1 < bx2 && by1 < by2 {
                for sy in by1..by2 {
                    let dy = sy - w.y;
                    for sx in bx1..bx2 {
                        let dx = sx - w.x;
                        let src_idx = (dy * w.width + dx) as usize;
                        let dst_idx = (sy * self.width as i32 + sx) as usize;
                        self.backbuffer[dst_idx] = w.buffer[src_idx];
                    }
                }
            }
        }
        
        // 3. Draw Dock
        let dock_pos = *crate::gui::settings_app::DOCK_POSITION.lock();
        
        let num_items = self.windows.len() as i32 + 1; // 1 for start button
        let item_w = 120;
        let item_h = 30;
        let padding = 4;
        let v_item_size = 40;
        
        let mut dock_w;
        let mut dock_h;
        let mut dock_x;
        let mut dock_y;
        let dx;
        let dy;

        if dock_pos == 0 || dock_pos == 1 { // Top / Bottom
            dock_w = num_items * (item_w + padding) + padding;
            dock_h = item_h + padding * 2;
            dock_x = (self.width as i32 - dock_w) / 2;
            if dock_x < 0 { dock_x = 0; }
            dock_y = if dock_pos == 0 { 10 } else { self.height as i32 - dock_h - 10 };
            dx = item_w + padding;
            dy = 0;
        } else { // Left / Right
            dock_w = v_item_size + padding * 2;
            dock_h = num_items * (v_item_size + padding) + padding;
            dock_y = (self.height as i32 - dock_h) / 2;
            if dock_y < 0 { dock_y = 0; }
            dock_x = if dock_pos == 2 { 10 } else { self.width as i32 - dock_w - 10 };
            dx = 0;
            dy = v_item_size + padding;
        }

        // Draw Dock Background (SkyOS Silver/Glossy)
        for dy in 0..dock_h {
            let color = 0xE0E0E0 - ((0xE0 - 0xC0) * dy / dock_h);
            let rgb = (color << 16) | (color << 8) | color;
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, dock_x, dock_y + dy, dock_w, 1, rgb as u32, clip_opt);
        }
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, dock_x-1, dock_y-1, dock_w+2, 1, 0xA0A0A0, clip_opt);
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, dock_x-1, dock_y+dock_h, dock_w+2, 1, 0x707070, clip_opt);
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, dock_x-1, dock_y-1, 1, dock_h+2, 0xA0A0A0, clip_opt);
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, dock_x+dock_w, dock_y-1, 1, dock_h+2, 0x707070, clip_opt);
        
        let mut curr_x = dock_x + padding;
        let mut curr_y = dock_y + padding;
        
        // Start Button
        let start_w = if dock_pos <= 1 { item_w } else { v_item_size };
        let start_h = if dock_pos <= 1 { item_h } else { v_item_size };
        let start_color = if self.start_menu_open { 0x4090E0 } else { 0x2A73C8 }; // Sky blue start button
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, curr_x, curr_y, start_w, start_h, start_color, clip_opt);
        draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, curr_x + (start_w/2) - 16, curr_y + (start_h/2) - 4, "Start", 0xFFFFFF, clip_opt);
        curr_x += dx;
        curr_y += dy;

        for (i, w) in self.windows.iter().enumerate() {
            let iw = if dock_pos <= 1 { item_w } else { v_item_size };
            let ih = if dock_pos <= 1 { item_h } else { v_item_size };
            let color = if i == self.windows.len() - 1 && w.state != WindowState::Minimized { 0xC0C0C0 } else { 0xDFE3E8 };
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, curr_x, curr_y, iw, ih, color, clip_opt);
            
            // Draw Icon
            if let Some(icon) = self.desktop_icons.iter().find(|ic| ic.name == w.title) {
                let draw_w = 16;
                let draw_h = 16;
                let scale_x_mul = (icon.bmp.width * 1024) / draw_w;
                let scale_y_mul = (icon.bmp.height * 1024) / draw_h;
                let icon_x = if dock_pos <= 1 { curr_x + 6 } else { curr_x + (v_item_size/2) - (draw_w as i32/2) };
                let icon_y = if dock_pos <= 1 { curr_y + 7 } else { curr_y + (v_item_size/2) - (draw_h as i32/2) };

                for dy_bmp in 0..draw_h {
                    let sy = icon_y + dy_bmp as i32;
                    if sy < 0 || sy >= self.height as i32 { continue; }
                    let src_y = (dy_bmp * scale_y_mul) / 1024;
                    for dx_bmp in 0..draw_w {
                        let sx = icon_x + dx_bmp as i32;
                        if sx < 0 || sx >= self.width as i32 { continue; }
                        let src_x = (dx_bmp * scale_x_mul) / 1024;
                        let c = icon.bmp.data[src_y * icon.bmp.width + src_x];
                        if c & 0xFF000000 != 0 && c != 0xFFFFFFFF { 
                            if let Some(cl) = clip_opt {
                                if sx >= cl.x && sx < cl.x + cl.w && sy >= cl.y && sy < cl.y + cl.h {
                                    self.backbuffer[(sy * self.width as i32 + sx) as usize] = c;
                                }
                            } else {
                                self.backbuffer[(sy * self.width as i32 + sx) as usize] = c;
                            }
                        }
                    }
                }
            }

            if dock_pos <= 1 {
                let mut title_disp = alloc::string::String::new();
                for (j, c) in w.title.chars().enumerate() { if j > 10 { break; } title_disp.push(c); }
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, curr_x + 28, curr_y + 11, &title_disp, 0x000000, clip_opt);
            }
            
            curr_x += dx;
            curr_y += dy;
        }

        // Draw Start Menu
        if self.start_menu_open {
            let sm_w = 200;
            let sm_h = 240;
            
            let sm_x = match dock_pos {
                0 | 1 => dock_x,
                2 => dock_x + dock_w + 4,
                _ => dock_x - sm_w - 4,
            };
            let sm_y = match dock_pos {
                0 => dock_y + dock_h + 4,
                1 => dock_y - sm_h - 4,
                _ => dock_y,
            };
            
            // Windows XP / SkyOS hybrid look: blue left panel, white main panel
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x, sm_y, sm_w, sm_h, 0xF5F5F5, clip_opt);
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x, sm_y, 40, sm_h, 0x2A73C8, clip_opt); // Sky Blue sidebar
            
            // Draw vertical 'Mithl OS' in left panel
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 4, sm_y + sm_h - 20, "M", 0xFFFFFF, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 4, sm_y + sm_h - 40, "i", 0xFFFFFF, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 4, sm_y + sm_h - 60, "t", 0xFFFFFF, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 4, sm_y + sm_h - 80, "h", 0xFFFFFF, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 4, sm_y + sm_h - 100, "l", 0xFFFFFF, clip_opt);

            let bg_sys = if self.active_submenu == Some(1) { 0x295D94 } else { 0xF5F5F5 };
            let fg_sys = if self.active_submenu == Some(1) { 0xFFFFFF } else { 0x000000 };
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 40, sm_y + 10, sm_w - 40, 30, bg_sys, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 50, sm_y + 17, "System >", fg_sys, clip_opt);
            
            let bg_acc = if self.active_submenu == Some(2) { 0x295D94 } else { 0xF5F5F5 };
            let fg_acc = if self.active_submenu == Some(2) { 0xFFFFFF } else { 0x000000 };
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 40, sm_y + 40, sm_w - 40, 30, bg_acc, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 50, sm_y + 47, "Accessories >", fg_acc, clip_opt);
            
            let bg_gam = if self.active_submenu == Some(3) { 0x295D94 } else { 0xF5F5F5 };
            let fg_gam = if self.active_submenu == Some(3) { 0xFFFFFF } else { 0x000000 };
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 40, sm_y + 70, sm_w - 40, 30, bg_gam, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 50, sm_y + 77, "Games >", fg_gam, clip_opt);

            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 40, sm_y + sm_h - 40, sm_w - 40, 30, 0xE0E0E0, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 50, sm_y + sm_h - 33, "Shutdown", 0xCC0000, clip_opt);

            // Draw active submenu
            if let Some(sub) = self.active_submenu {
                let sub_x = sm_x + sm_w;
                let sub_w = 160;
                let sub_y = match sub {
                    1 => sm_y + 5,
                    2 => sm_y + 25,
                    3 => sm_y + 45,
                    _ => sm_y,
                };
                
                if sub == 1 {
                    // System submenu
                    fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sub_x, sub_y, sub_w, 124, 0xF5F5F5, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 4,  "Terminal", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 28, "File Manager", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 52, "Settings", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 76, "Task Manager", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 100,"Sys Monitor", 0x000000, clip_opt);
                } else if sub == 2 {
                    fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sub_x, sub_y, sub_w, 104, 0xF5F5F5, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 4,  "Notepad", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 24, "Calculator", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 44, "Clock", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 64, "Calendar", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 84, "Paint", 0x000000, clip_opt);
                } else if sub == 3 {
                    fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sub_x, sub_y, sub_w, 164, 0xF5F5F5, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 4,   "Snake", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 24,  "Minesweeper", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 44,  "Tetris", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 64,  "Pong", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 84,  "2048", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 104, "Sudoku", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 124, "Chess", 0x000000, clip_opt);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 144, "Calculator", 0x000000, clip_opt);
                }
            }
        }
        // Draw Context Menu
        if self.context_menu_open {
            let cm_w = 120;
            let cm_h = 44; // 2 items
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x, self.context_menu_y, cm_w, cm_h, 0xC0C0C0, clip_opt);
            
            // Draw border
            for i in 0..cm_w {
                fill_rect_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + i, self.context_menu_y, 1, 1, 0xFFFFFF, clip_opt);
                fill_rect_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + i, self.context_menu_y + cm_h - 1, 1, 1, 0x000000, clip_opt);
            }
            for i in 0..cm_h {
                fill_rect_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x, self.context_menu_y + i, 1, 1, 0xFFFFFF, clip_opt);
                fill_rect_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + cm_w - 1, self.context_menu_y + i, 1, 1, 0x000000, clip_opt);
            }

            if self.context_menu_target.is_some() {
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + 10, self.context_menu_y + 8, "Open", 0x000000, clip_opt);
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + 10, self.context_menu_y + 28, "Properties", 0x000000, clip_opt);
            } else {
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + 10, self.context_menu_y + 8, "New Folder", 0x000000, clip_opt);
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + 10, self.context_menu_y + 28, "Refresh", 0x000000, clip_opt);
            }
        }
        
        // Blitting is now handled centrally inside draw() to support software cursor overlay.
    }
    
    pub fn process_input(&mut self) -> bool {
        let fw = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock() as usize;
        let fh = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock() as usize;
        if self.width != fw || self.height != fh {
            self.width = fw;
            self.height = fh;
            self.backbuffer = alloc::vec![0; fw * fh];
        }

        let old_mouse_x = self.mouse_x;
        let old_mouse_y = self.mouse_y;
        
        let mut needs_redraw = false;
        let mut mouse_moved = false;
        
        while let Some(mev) = mouse::pop_event() {
            if self.mouse_x != mev.x as i32 || self.mouse_y != mev.y as i32 {
                self.mouse_x = mev.x as i32;
                self.mouse_y = mev.y as i32;
                mouse_moved = true;
            }
            
            // Handle hovering over start menu categories
            if self.start_menu_open {
                let sm_w = 200;
                let sm_h = 240;
                let sm_x = 0;
                let sm_y = self.height as i32 - 30 - sm_h;
                if self.mouse_x >= sm_x && self.mouse_x <= sm_x + sm_w && self.mouse_y >= sm_y && self.mouse_y <= sm_y + sm_h {
                    if self.mouse_y >= sm_y + 10 && self.mouse_y < sm_y + 40 {
                        self.active_submenu = Some(1);
                    } else if self.mouse_y >= sm_y + 40 && self.mouse_y < sm_y + 70 {
                        self.active_submenu = Some(2);
                    } else if self.mouse_y >= sm_y + 70 && self.mouse_y < sm_y + 100 {
                        self.active_submenu = Some(3);
                    } else {
                        self.active_submenu = None;
                    }
                }
            }

            let is_down = mev.buttons & 1 != 0;
            let is_right_down = mev.buttons & 2 != 0;
            
            // Close context menu if left clicked
            if self.context_menu_open && is_down && !self.mouse_left_down {
                self.context_menu_open = false;
                self.mouse_left_down = is_down;
                self.mouse_right_down = is_right_down;
                continue;
            }
            
            // Handle right click
            if is_right_down && !self.mouse_right_down {
                let mut clicked_idx = None;
                for (i, icon) in self.desktop_icons.iter().enumerate() {
                    if self.mouse_x >= icon.x && self.mouse_x <= icon.x + 48 && self.mouse_y >= icon.y && self.mouse_y <= icon.y + 60 {
                        clicked_idx = Some(i);
                        break;
                    }
                }
                self.context_menu_open = true;
                self.context_menu_x = self.mouse_x;
                self.context_menu_y = self.mouse_y;
                self.context_menu_target = clicked_idx;
            }
            
            // Handle clicking & dragging
            if is_down && !self.mouse_left_down {
                // Mouse just pressed
                let mut handled = false;
                
                // Calculate dock bounds
                let dock_pos = *crate::gui::settings_app::DOCK_POSITION.lock();
                let num_items = self.windows.len() as i32 + 1;
                let item_w = 120;
                let item_h = 30;
                let padding = 4;
                let v_item_size = 40;
                let dock_w = if dock_pos <= 1 { num_items * (item_w + padding) + padding } else { v_item_size + padding * 2 };
                let dock_h = if dock_pos <= 1 { item_h + padding * 2 } else { num_items * (v_item_size + padding) + padding };
                let mut dock_x = if dock_pos <= 1 { (self.width as i32 - dock_w) / 2 } else if dock_pos == 2 { 10 } else { self.width as i32 - dock_w - 10 };
                let mut dock_y = if dock_pos == 0 { 10 } else if dock_pos == 1 { self.height as i32 - dock_h - 10 } else { (self.height as i32 - dock_h) / 2 };
                if dock_x < 0 { dock_x = 0; }
                if dock_y < 0 { dock_y = 0; }

                // Check Start Menu items if open
                if self.start_menu_open {
                    let sm_w = 200;
                    let sm_h = 240;
                    let sm_x = match dock_pos {
                        0 | 1 => dock_x,
                        2 => dock_x + dock_w + 4,
                        _ => dock_x - sm_w - 4,
                    };
                    let sm_y = match dock_pos {
                        0 => dock_y + dock_h + 4,
                        1 => dock_y - sm_h - 4,
                        _ => dock_y,
                    };
                    
                    // First check if clicked inside active submenu
                    if let Some(sub) = self.active_submenu {
                        let sub_x = sm_x + sm_w;
                        let sub_w = 150;
                        let sub_y = match sub {
                            1 => sm_y + 5,
                            2 => sm_y + 25,
                            3 => sm_y + 45,
                            _ => sm_y,
                        };
                        let h = match sub {
                            1 => 24,
                            2  => 104,
                            3  => 144,
                            _ => 24,
                        };
                        
                        if self.mouse_x >= sub_x && self.mouse_x <= sub_x + sub_w && self.mouse_y >= sub_y && self.mouse_y <= sub_y + h {
                            if sub == 1 {
                                // System submenu: Terminal | File Manager | Settings | Task Manager | Sys Monitor
                                let sub_h = 124;
                                let _ = sub_h;
                                if self.mouse_y < sub_y + 24 {
                                    crate::apps::gui_apps::launch_terminal(self);
                                } else if self.mouse_y < sub_y + 52 {
                                    crate::apps::gui_apps::launch_file_manager(self);
                                } else if self.mouse_y < sub_y + 76 {
                                    crate::apps::gui_apps::launch_settings(self);
                                } else if self.mouse_y < sub_y + 100 {
                                    crate::apps::gui_apps::launch_task_manager(self);
                                } else {
                                    crate::apps::gui_apps::launch_sysmon(self);
                                }
                            } else if sub == 2 {
                                // Accessories -> Notepad, Calculator, Clock, Calendar, Paint
                                if self.mouse_y < sub_y + 20 {
                                    crate::apps::gui_apps::launch_notepad(self);
                                } else if self.mouse_y < sub_y + 44 {
                                    crate::apps::gui_apps::launch_calculator(self);
                                } else if self.mouse_y < sub_y + 64 {
                                    crate::apps::gui_apps::launch_clock(self);
                                } else if self.mouse_y < sub_y + 84 {
                                    crate::apps::gui_apps::launch_calendar(self);
                                } else {
                                    crate::apps::gui_apps::launch_paint(self);
                                }
                            } else if sub == 3 {
                                // Games -> Snake, Minesweeper, Tetris, Pong, 2048, Sudoku, Chess
                                if self.mouse_y < sub_y + 20 {
                                    crate::apps::gui_apps::launch_snake(self);
                                } else if self.mouse_y < sub_y + 44 {
                                    crate::apps::gui_apps::launch_minesweeper(self);
                                } else if self.mouse_y < sub_y + 64 {
                                    crate::apps::gui_apps::launch_tetris(self);
                                } else if self.mouse_y < sub_y + 84 {
                                    crate::apps::gui_apps::launch_pong(self);
                                } else if self.mouse_y < sub_y + 104 {
                                    crate::apps::gui_apps::launch_2048(self);
                                } else if self.mouse_y < sub_y + 124 {
                                    crate::apps::gui_apps::launch_sudoku(self);
                                } else if self.mouse_y < sub_y + 144 {
                                    crate::apps::gui_apps::launch_chess(self);
                                } else {
                                    crate::apps::gui_apps::launch_calculator(self); // fallback
                                }
                            }
                            self.start_menu_open = false;
                            self.active_submenu = None;
                            handled = true;
                        }
                    }
                    
                    if !handled && self.mouse_x >= sm_x && self.mouse_x <= sm_x + sm_w && self.mouse_y >= sm_y && self.mouse_y <= sm_y + sm_h {
                        // Clicked inside main start menu
                        if self.mouse_y >= sm_y + sm_h - 40 && self.mouse_y < sm_y + sm_h - 10 {
                            // Shutdown
                            return false;
                        }
                        // Check if we clicked a category
                        if self.mouse_y >= sm_y + 10 && self.mouse_y < sm_y + 40 {
                            self.active_submenu = Some(1);
                        } else if self.mouse_y >= sm_y + 40 && self.mouse_y < sm_y + 70 {
                            self.active_submenu = Some(2);
                        } else if self.mouse_y >= sm_y + 70 && self.mouse_y < sm_y + 100 {
                            self.active_submenu = Some(3);
                        }
                        handled = true;
                    }
                }

                // Check Dock clicks
                if !handled && self.mouse_x >= dock_x && self.mouse_x <= dock_x + dock_w && self.mouse_y >= dock_y && self.mouse_y <= dock_y + dock_h {
                    let start_w = if dock_pos <= 1 { item_w } else { v_item_size };
                    let start_h = if dock_pos <= 1 { item_h } else { v_item_size };
                    let mut curr_x = dock_x + padding;
                    let mut curr_y = dock_y + padding;
                    let dx = if dock_pos <= 1 { item_w + padding } else { 0 };
                    let dy = if dock_pos > 1 { v_item_size + padding } else { 0 };

                    // Start button click
                    if self.mouse_x >= curr_x && self.mouse_x <= curr_x + start_w && self.mouse_y >= curr_y && self.mouse_y <= curr_y + start_h {
                        self.start_menu_open = !self.start_menu_open;
                        handled = true;
                    } else {
                        curr_x += dx;
                        curr_y += dy;
                        
                        let iw = if dock_pos <= 1 { item_w } else { v_item_size };
                        let ih = if dock_pos <= 1 { item_h } else { v_item_size };
                        
                        let mut clicked_idx = None;
                        for (i, _w) in self.windows.iter().enumerate() {
                            if self.mouse_x >= curr_x && self.mouse_x <= curr_x + iw && self.mouse_y >= curr_y && self.mouse_y <= curr_y + ih {
                                clicked_idx = Some(i);
                                break;
                            }
                            curr_x += dx;
                            curr_y += dy;
                        }
                        
                        if let Some(i) = clicked_idx {
                            if self.windows[i].state == WindowState::Minimized {
                                self.windows[i].state = WindowState::Normal;
                            }
                            let win = self.windows.remove(i);
                            self.windows.push(win);
                            handled = true;
                        }
                        self.start_menu_open = false; // Close menu if clicked elsewhere
                    }
                } else if !handled {
                    self.start_menu_open = false; // Close menu if clicked elsewhere
                }
                
                // Check windows in reverse order (top to bottom)
                if !handled {
                    let mut action = None;
                    
                    for i in (0..self.windows.len()).rev() {
                        let w = &mut self.windows[i];
                        
                        if w.is_point_in_close_btn(self.mouse_x, self.mouse_y) {
                            action = Some((i, 0)); // 0 = close
                            break;
                        } else if w.is_point_in_max_btn(self.mouse_x, self.mouse_y) {
                            action = Some((i, 1)); // 1 = maximize
                            break;
                        } else if let Some(edge) = w.check_resize_zone(self.mouse_x, self.mouse_y) {
                            self.resized_window = Some(i);
                            self.active_edge = Some(edge);
                            action = Some((i, 3)); // 3 = bring to front
                            break;
                        } else if w.is_point_in_titlebar(self.mouse_x, self.mouse_y) {
                            self.dragged_window = Some(i);
                            w.drag_offset_x = self.mouse_x - w.x;
                            w.drag_offset_y = self.mouse_y - w.y;
                            
                            action = Some((i, 3)); // 3 = bring to front
                            break; // Stop checking lower windows
                        } else if w.is_point_inside(self.mouse_x, self.mouse_y) && w.state != WindowState::Minimized {
                            // Event dispatching to be handled by Window Manager API
                            
                            action = Some((i, 3)); // 3 = bring to front
                            break; // Stop checking lower windows
                        }
                    }
                    
                    if let Some((i, act)) = action {
                        if act == 0 {
                            self.windows.remove(i);
                        } else if act == 1 {
                            let w = &mut self.windows[i];
                            if w.state == WindowState::Maximized {
                                w.state = WindowState::Normal;
                                w.x = w.restore_x;
                                w.y = w.restore_y;
                                w.width = w.restore_width;
                                w.height = w.restore_height;
                                w.buffer = alloc::vec![0xFFFFFF; (w.width * w.height) as usize];
                            } else {
                                w.state = WindowState::Maximized;
                                w.restore_x = w.x;
                                w.restore_y = w.y;
                                w.restore_width = w.width;
                                w.restore_height = w.height;
                                w.x = 0;
                                w.y = 24;
                                w.width = self.width as i32;
                                w.height = self.height as i32 - 30 - 24;
                                w.buffer = alloc::vec![0xFFFFFF; (w.width * w.height) as usize];
                            }
                            let win = self.windows.remove(i);
                            self.windows.push(win);
                        } else if act == 2 {
                            self.windows[i].state = WindowState::Minimized;
                        } else if act == 3 {
                            let win = self.windows.remove(i);
                            self.windows.push(win);
                        }
                    } else {
                        // Desktop icon clicking
                        let mut clicked_idx = None;
                        for (i, icon) in self.desktop_icons.iter().enumerate() {
                            if self.mouse_x >= icon.x && self.mouse_x <= icon.x + 48 && self.mouse_y >= icon.y && self.mouse_y <= icon.y + 60 {
                                clicked_idx = Some(i);
                                break;
                            }
                        }
                        
                        if let Some(idx) = clicked_idx {
                            let current_ticks = crate::process::scheduler::get_ticks();
                            if self.last_clicked_icon == Some(idx) && (current_ticks - self.last_click_ticks) < 50 {
                                // Double click
                                let app_name = self.desktop_icons[idx].name.clone();
                                let mut win = Window::new((self.windows.len() + 1) as u32, &app_name, 100, 100, 400, 300);
                                self.windows.push(win);
                                self.last_clicked_icon = None;
                            } else {
                                // Single click -> Select / Drag
                                self.last_clicked_icon = Some(idx);
                                self.last_click_ticks = current_ticks;
                                self.dragged_icon = Some(idx);
                                self.icon_drag_offset_x = self.mouse_x - self.desktop_icons[idx].x;
                                self.icon_drag_offset_y = self.mouse_y - self.desktop_icons[idx].y;
                            }
                        }
                    }
                }
            } else if !is_down && self.mouse_left_down {
                // Mouse released
                self.dragged_window = None;
                self.resized_window = None;
                self.dragged_icon = None;
            }
            
            // Handle dragging
            if let Some(idx) = self.dragged_window {
                if is_down {
                    let w = &mut self.windows[idx];
                    w.x = self.mouse_x - w.drag_offset_x;
                    w.y = self.mouse_y - w.drag_offset_y;
                }
            }

            // Handle icon dragging
            if let Some(idx) = self.dragged_icon {
                if is_down {
                    let icon = &mut self.desktop_icons[idx];
                    icon.x = self.mouse_x - self.icon_drag_offset_x;
                    icon.y = self.mouse_y - self.icon_drag_offset_y;
                }
            }

            // Handle resizing
            if let Some(idx) = self.resized_window {
                if is_down {
                    let edge = self.active_edge.unwrap_or(2);
                    let w = &mut self.windows[idx];
                    
                    let mut new_x = w.x;
                    let mut new_y = w.y;
                    let mut new_w = w.width;
                    let mut new_h = w.height;

                    if edge == 0 || edge == 2 { // Right
                        new_w = (self.mouse_x - w.x).max(100);
                    }
                    if edge == 1 || edge == 2 { // Bottom
                        new_h = (self.mouse_y - w.y).max(100);
                    }
                    if edge == 3 { // Left
                        let dw = w.x - self.mouse_x;
                        if w.width + dw >= 100 {
                            new_x = self.mouse_x;
                            new_w = w.width + dw;
                        }
                    }
                    if edge == 4 { // Top
                        let dh = w.y - self.mouse_y;
                        if w.height + dh >= 100 {
                            new_y = self.mouse_y;
                            new_h = w.height + dh;
                        }
                    }
                    
                    if new_w != w.width || new_h != w.height || new_x != w.x || new_y != w.y {
                        w.x = new_x;
                        w.y = new_y;
                        w.width = new_w;
                        w.height = new_h;
                        w.buffer = alloc::vec![0xFFFFFF; (new_w * new_h) as usize];
                    }
                }
            }
            
            self.mouse_left_down = is_down;
            self.mouse_right_down = is_right_down;
        }
        
        let mut cursor = crate::drivers::video::CursorState::Normal;
        if self.resized_window.is_some() {
            let edge = self.active_edge.unwrap_or(2);
            if edge == 0 || edge == 3 { cursor = crate::drivers::video::CursorState::ResizeH; }
            else if edge == 1 || edge == 4 { cursor = crate::drivers::video::CursorState::ResizeV; }
        } else {
            for w in self.windows.iter().rev() {
                if let Some(edge) = w.check_resize_zone(self.mouse_x, self.mouse_y) {
                    if edge == 0 || edge == 3 { cursor = crate::drivers::video::CursorState::ResizeH; }
                    else if edge == 1 || edge == 4 { cursor = crate::drivers::video::CursorState::ResizeV; }
                    break;
                }
                if w.is_point_inside(self.mouse_x, self.mouse_y) && w.state != crate::gui::window::WindowState::Minimized {
                    break;
                }
            }
        }
        {
            let mut current = crate::drivers::video::CURRENT_CURSOR.lock();
            if *current != cursor {
                *current = cursor;
                mouse_moved = true; // force redraw of cursor
            }
        }
        
        // Check keyboard for exit
        if let Some(c) = keyboard::pop_char() {
            if c == '\x1B' { // ESC
                return false;
            } else {
                if let Some(w) = self.windows.last_mut() {
                    // Event dispatching to be handled by Window Manager API
                    needs_redraw = true;
                }
            }
        }
        
            // Removed hardcoded w.app.update()
            
        // Process IPC messages from Window Manager
        {
            let mut msgs = crate::gui::wm::GUI_MESSAGES.lock();
            for msg in msgs.drain(..) {
                match msg {
                    crate::gui::wm::GuiMessage::CreateWindow { id, title, x, y, w, h } => {
                        let win = crate::gui::window::Window::new(id, &title, x, y, w, h);
                        self.windows.push(win);
                        needs_redraw = true;
                    }
                    crate::gui::wm::GuiMessage::UpdateBuffer { id, buffer_ptr } => {
                        for win in &mut self.windows {
                            if win.id == id {
                                let slice = unsafe { core::slice::from_raw_parts(buffer_ptr as *const u32, win.buffer.len()) };
                                win.buffer.copy_from_slice(slice);
                                needs_redraw = true;
                                break;
                            }
                        }
                    }
                    crate::gui::wm::GuiMessage::CloseWindow { id } => {
                        self.windows.retain(|win| win.id != id);
                        needs_redraw = true;
                    }
                }
            }
        }

        let current_ticks = crate::process::scheduler::get_ticks();
        
        if needs_redraw || (current_ticks.saturating_sub(self.last_redraw_ticks) >= 5) {
            for r in &self.last_frame_rects {
                self.dirty_rects.push(*r);
            }
            self.last_frame_rects.clear();
            
            let mut current_rects = alloc::vec::Vec::new();
            
            if self.start_menu_open {
                let dock_pos = *crate::gui::settings_app::DOCK_POSITION.lock();
                let num_items = self.windows.len() as i32 + 1;
                let item_w = 120;
                let padding = 4;
                let v_item_size = 40;
                let mut dock_w;
                let mut dock_x;
                if dock_pos == 0 || dock_pos == 1 {
                    dock_w = num_items * (item_w + padding) + padding;
                    dock_x = (self.width as i32 - dock_w) / 2;
                    if dock_x < 0 { dock_x = 0; }
                } else {
                    dock_w = v_item_size + padding * 2;
                    dock_x = if dock_pos == 2 { 10 } else { self.width as i32 - dock_w - 10 };
                }
                
                let sm_w = 200;
                let sm_h = 240;
                let sm_x = match dock_pos {
                    0 | 1 => dock_x,
                    2 => dock_x + dock_w + 4,
                    _ => dock_x - sm_w - 4,
                };
                let sm_y = match dock_pos {
                    0 => 10 + (30 + padding * 2) + 4,
                    1 => self.height as i32 - (30 + padding * 2) - 10 - sm_h - 4,
                    _ => (self.height as i32 - (num_items * (v_item_size + padding) + padding)) / 2,
                };
                current_rects.push(crate::gui::clip::Rect { x: sm_x, y: sm_y, w: sm_w, h: sm_h });
            }
            
            if self.context_menu_open {
                current_rects.push(crate::gui::clip::Rect { x: self.context_menu_x, y: self.context_menu_y, w: 120, h: 44 });
            }
            
            current_rects.push(crate::gui::clip::Rect { x: 0, y: 0, w: self.width as i32, h: 60 });
            current_rects.push(crate::gui::clip::Rect { x: 0, y: self.height as i32 - 60, w: self.width as i32, h: 60 });
            current_rects.push(crate::gui::clip::Rect { x: 0, y: 0, w: 60, h: self.height as i32 });
            current_rects.push(crate::gui::clip::Rect { x: self.width as i32 - 60, y: 0, w: 60, h: self.height as i32 });
            
            for w in &self.windows {
                if w.state != WindowState::Minimized {
                    current_rects.push(crate::gui::clip::Rect { x: w.x - 4, y: w.y - 28, w: w.width + 8, h: w.height + 32 });
                }
            }
            
            for r in &current_rects {
                self.dirty_rects.push(*r);
                self.last_frame_rects.push(*r);
            }
            
            if mouse_moved {
                self.dirty_rects.push(crate::gui::clip::Rect { x: old_mouse_x, y: old_mouse_y, w: 32, h: 32 });
                self.dirty_rects.push(crate::gui::clip::Rect { x: self.mouse_x, y: self.mouse_y, w: 32, h: 32 });
            }
            
            self.draw();
            self.last_redraw_ticks = current_ticks;
        } else if mouse_moved {
            self.dirty_rects.push(crate::gui::clip::Rect { x: old_mouse_x, y: old_mouse_y, w: 32, h: 32 });
            self.dirty_rects.push(crate::gui::clip::Rect { x: self.mouse_x, y: self.mouse_y, w: 32, h: 32 });
            self.draw();
            self.last_redraw_ticks = crate::process::scheduler::get_ticks();
        }
        
        true
    }
}
