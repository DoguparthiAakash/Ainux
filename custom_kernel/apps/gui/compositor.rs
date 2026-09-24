use alloc::vec::Vec;
use crate::drivers::video;
use crate::drivers::mouse;
use crate::drivers::keyboard;
use super::window::{Window, WindowState};
use super::terminal_app::TerminalApp;
use super::notepad_app::NotepadApp;

// Utility to draw strings to a generic buffer
fn draw_string_to_buffer(buf: &mut [u32], buf_w: usize, buf_h: usize, x: i32, y: i32, s: &str, fg: u32) {
    let mut curr_x = x;
    for c in s.chars() {
        if curr_x >= 0 && (curr_x + 8) <= buf_w as i32 && y >= 0 && (y + 16) <= buf_h as i32 {
            // we need to access the font data. video::draw_char_raw uses VGA_FONT
            // Instead, we will expose a video::draw_char_to_buffer function, or just use it.
            // For now we'll add a helper in video.rs or replicate font logic. 
            // We'll call video::draw_char_to_buffer
            video::draw_char_to_buffer(buf, buf_w, buf_h, curr_x as usize, y as usize, c, fg);
        }
        curr_x += 8;
    }
}

fn fill_rect_buffer(buf: &mut [u32], buf_w: usize, buf_h: usize, rx: i32, ry: i32, rw: i32, rh: i32, color: u32) {
    for dy in 0..rh {
        let sy = ry + dy;
        if sy < 0 || sy >= buf_h as i32 { continue; }
        for dx in 0..rw {
            let sx = rx + dx;
            if sx < 0 || sx >= buf_w as i32 { continue; }
            buf[(sy * buf_w as i32 + sx) as usize] = color;
        }
    }
}

pub struct Compositor {
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
        }
    }

    pub fn add_window(&mut self, w: Window) {
        self.windows.push(w);
    }

    pub fn draw(&mut self) {
        // 1. Clear background (Desktop Wallpaper)
        let bg_color = *crate::gui::settings_app::WALLPAPER_COLOR.lock();
        // Start below the top menu bar (y=24)
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 0, 24, self.width as i32, self.height as i32 - 24, bg_color | 0xFF000000);

        // 1.2 Draw Top Menu Bar (Mac-like)
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 0, 0, self.width as i32, 24, 0xDFDFDF);
        // Thin shadow under menu bar
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 0, 24, self.width as i32, 1, 0x808080);
        draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, 10, 4, "Mithl OS", 0x000000);
        if let Some(w) = self.windows.last() {
            if w.state != WindowState::Minimized {
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, 100, 4, &w.title, 0x000000);
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, 180, 4, "File  Edit  View  Help", 0x000000);
            }
        }

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
                    if color & 0xFF000000 != 0 && color != 0xFFFFFFFF { // extremely basic transparency for white bg bmp
                        self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                    }
                }
            }
            // Draw text
            let text_x = icon.x + (draw_w as i32 / 2) - (icon.name.len() as i32 * 4);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, text_x, icon.y + draw_h as i32 + 5, &icon.name, 0xFFFFFF);
        }

        // 2. Draw Windows from bottom to top
        for w in &mut self.windows {
            if w.state == WindowState::Minimized { continue; }
            
            let border = 2;
            let title_h = 24;
            
            // Draw window frame
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 
                w.x - border, w.y - title_h - border, w.width + border * 2, w.height + title_h + border * 2, 0xC0C0C0);
                
            // Draw title bar
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 
                w.x, w.y - title_h, w.width, title_h, 0x000080); // Active blue
                
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, w.x + 4, w.y - title_h + 4, &w.title, 0xFFFFFF);

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
                                self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
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
            
            w.app.update();
            w.app.draw(&mut w.buffer, w.width as usize, w.height as usize);
            
            // Draw Resize Handle (bottom right)
            let res_x = w.x + w.width - 12;
            let res_y = w.y + w.height - 12;
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, res_x, res_y, 12, 12, 0x808080);
            
            // Draw window content (blit its buffer)
            for dy in 0..w.height {
                let sy = w.y + dy;
                if sy < 0 || sy >= self.height as i32 { continue; }
                
                for dx in 0..w.width {
                    let sx = w.x + dx;
                    if sx < 0 || sx >= self.width as i32 { continue; }
                    
                    let src_idx = (dy * w.width + dx) as usize;
                    let dst_idx = (sy * self.width as i32 + sx) as usize;
                    self.backbuffer[dst_idx] = w.buffer[src_idx];
                }
            }
        }
        
        // 3. Draw Taskbar
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 0, self.height as i32 - 30, self.width as i32, 30, 0xC0C0C0);
        // Start Button
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 2, self.height as i32 - 28, 60, 26, if self.start_menu_open { 0x808080 } else { 0xDFDFDF });
        draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, 10, self.height as i32 - 22, "Start", 0x000000);
        
            // Draw Taskbar Items (one for each window)
        let mut tb_x = 70;
        for (i, w) in self.windows.iter().enumerate() {
            let item_w = 120;
            // Draw recessed if it is the active window (last in list)
            let color = if i == self.windows.len() - 1 && w.state != WindowState::Minimized { 0x808080 } else { 0xDFDFDF };
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, tb_x, self.height as i32 - 28, item_w, 26, color);
            
            // Draw Window Icon
            if let Some(icon) = self.desktop_icons.iter().find(|ic| ic.name == w.title) {
                let draw_w = 16;
                let draw_h = 16;
                let scale_x_mul = (icon.bmp.width * 1024) / draw_w;
                let scale_y_mul = (icon.bmp.height * 1024) / draw_h;

                for dy in 0..draw_h {
                    let sy = self.height as i32 - 23 + dy as i32;
                    if sy < 0 || sy >= self.height as i32 { continue; }
                    let src_y = (dy * scale_y_mul) / 1024;
                    
                    for dx in 0..draw_w {
                        let sx = tb_x + 6 + dx as i32;
                        if sx < 0 || sx >= self.width as i32 { continue; }
                        let src_x = (dx * scale_x_mul) / 1024;
                        
                        let color = icon.bmp.data[src_y * icon.bmp.width + src_x];
                        if color & 0xFF000000 != 0 && color != 0xFFFFFFFF { 
                            self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                        }
                    }
                }
            }

            // Truncate title
            let mut title_disp = alloc::string::String::new();
            for (j, c) in w.title.chars().enumerate() {
                if j > 10 { break; }
                title_disp.push(c);
            }
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, tb_x + 28, self.height as i32 - 22, &title_disp, 0x000000);
            
            tb_x += item_w + 4;
        }

        // Draw Start Menu
        if self.start_menu_open {
            let sm_w = 200;
            let sm_h = 240;
            let sm_x = 0;
            let sm_y = self.height as i32 - 30 - sm_h;
            
            // Windows XP / Mac hybrid look: left panel dark, right panel light
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x, sm_y, sm_w, sm_h, 0xEEEEEE);
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x, sm_y, 40, sm_h, 0x0055AA);
            
            // Draw vertical 'Mithl OS' in left panel
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 4, sm_y + sm_h - 20, "M", 0xFFFFFF);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 4, sm_y + sm_h - 40, "i", 0xFFFFFF);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 4, sm_y + sm_h - 60, "t", 0xFFFFFF);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 4, sm_y + sm_h - 80, "h", 0xFFFFFF);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 4, sm_y + sm_h - 100, "l", 0xFFFFFF);

            let bg_sys = if self.active_submenu == Some(1) { 0x0078D7 } else { 0xEEEEEE };
            let fg_sys = if self.active_submenu == Some(1) { 0xFFFFFF } else { 0x000000 };
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 40, sm_y + 10, sm_w - 40, 30, bg_sys);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 50, sm_y + 17, "System >", fg_sys);
            
            let bg_acc = if self.active_submenu == Some(2) { 0x0078D7 } else { 0xEEEEEE };
            let fg_acc = if self.active_submenu == Some(2) { 0xFFFFFF } else { 0x000000 };
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 40, sm_y + 40, sm_w - 40, 30, bg_acc);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 50, sm_y + 47, "Accessories >", fg_acc);
            
            let bg_gam = if self.active_submenu == Some(3) { 0x0078D7 } else { 0xEEEEEE };
            let fg_gam = if self.active_submenu == Some(3) { 0xFFFFFF } else { 0x000000 };
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 40, sm_y + 70, sm_w - 40, 30, bg_gam);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 50, sm_y + 77, "Games >", fg_gam);

            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 40, sm_y + sm_h - 40, sm_w - 40, 30, 0xDDDDDD);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sm_x + 50, sm_y + sm_h - 33, "Shutdown", 0x000000);

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
                    // System submenu: Terminal, File Manager, Settings, Task Manager, Sys Monitor
                    fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sub_x, sub_y, sub_w, 124, 0xC0C0C0);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 4,  "Terminal",     0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 28, "File Manager", 0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 52, "Settings",     0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 76, "Task Manager", 0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 100,"Sys Monitor",  0x000000);
                } else if sub == 2 {
                    fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sub_x, sub_y, sub_w, 104, 0xC0C0C0);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 4,  "Notepad",    0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 24, "Calculator", 0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 44, "Clock",      0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 64, "Calendar",   0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 84, "Paint",      0x000000);
                } else if sub == 3 {
                    fill_rect_buffer(&mut self.backbuffer, self.width, self.height, sub_x, sub_y, sub_w, 164, 0xC0C0C0);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 4,   "Snake",      0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 24,  "Minesweeper",0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 44,  "Tetris",     0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 64,  "Pong",       0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 84,  "2048",       0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 104, "Sudoku",     0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 124, "Chess",      0x000000);
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, sub_x + 10, sub_y + 144, "Calculator", 0x000000);
                }
            }
        }
        // Draw Context Menu
        if self.context_menu_open {
            let cm_w = 120;
            let cm_h = 44; // 2 items
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x, self.context_menu_y, cm_w, cm_h, 0xC0C0C0);
            
            // Draw border
            for i in 0..cm_w {
                fill_rect_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + i, self.context_menu_y, 1, 1, 0xFFFFFF);
                fill_rect_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + i, self.context_menu_y + cm_h - 1, 1, 1, 0x000000);
            }
            for i in 0..cm_h {
                fill_rect_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x, self.context_menu_y + i, 1, 1, 0xFFFFFF);
                fill_rect_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + cm_w - 1, self.context_menu_y + i, 1, 1, 0x000000);
            }

            if self.context_menu_target.is_some() {
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + 10, self.context_menu_y + 8, "Open", 0x000000);
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + 10, self.context_menu_y + 28, "Properties", 0x000000);
            } else {
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + 10, self.context_menu_y + 8, "New Folder", 0x000000);
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height, self.context_menu_x + 10, self.context_menu_y + 28, "Refresh", 0x000000);
            }
        }
        
        // 4. Blit backbuffer to framebuffer
        // Use blit_buffer_opaque to correctly handle hardware pitch (stride)
        video::blit_buffer_opaque(&self.backbuffer, 0, 0, self.width as i32, self.height as i32, self.width as i32);
        
        // 5. Draw mouse directly on top (using existing hardware cursor overlay if possible, or draw to FB)
        video::draw_mouse_cursor(self.mouse_x, self.mouse_y);
    }
    
    pub fn process_input(&mut self) -> bool {
        let fw = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock() as usize;
        let fh = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock() as usize;
        if self.width != fw || self.height != fh {
            self.width = fw;
            self.height = fh;
            self.backbuffer = alloc::vec![0; fw * fh];
        }

        let mut needs_redraw = false;
        
        while let Some(mev) = mouse::pop_event() {
            self.mouse_x = mev.x as i32;
            self.mouse_y = mev.y as i32;
            needs_redraw = true;
            
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
                
                // Check Start Menu items if open
                if self.start_menu_open {
                    let sm_w = 150;
                    let sm_h = 100;
                    let sm_x = 0;
                    let sm_y = self.height as i32 - 30 - sm_h;
                    
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
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Terminal", 50, 50, 480, 320);
                                    win.app = super::app::AppType::Terminal(TerminalApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 52 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "File Manager", 80, 80, 380, 380);
                                    win.app = super::app::AppType::FileManager(crate::gui::file_manager_app::FileManagerApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 76 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Settings", 100, 100, 380, 300);
                                    win.app = super::app::AppType::Settings(crate::gui::settings_app::SettingsApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 100 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Task Manager", 120, 120, 360, 360);
                                    win.app = super::app::AppType::TaskManager(crate::gui::task_manager_app::TaskManagerApp::new());
                                    self.windows.push(win);
                                } else {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Sys Monitor", 140, 140, 320, 300);
                                    win.app = super::app::AppType::SysMon(crate::gui::sysmon_app::SysMonApp::new());
                                    self.windows.push(win);
                                }
                            } else if sub == 2 {
                                // Accessories -> Notepad, Calculator, Clock, Calendar, Paint
                                if self.mouse_y < sub_y + 20 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Notepad", 100, 100, 400, 300);
                                    win.app = super::app::AppType::Notepad(crate::gui::notepad_app::NotepadApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 44 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Calculator", 150, 150, 300, 200);
                                    win.app = super::app::AppType::Calculator(crate::gui::calculator_app::CalculatorApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 64 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Clock", 200, 120, 380, 160);
                                    win.app = super::app::AppType::Clock(crate::gui::clock_app::ClockApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 84 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Calendar", 180, 100, 290, 230);
                                    win.app = super::app::AppType::Calendar(crate::gui::calendar_app::CalendarApp::new());
                                    self.windows.push(win);
                                } else {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Paint", 180, 100, 420, 350);
                                    win.app = super::app::AppType::Paint(crate::gui::paint_app::PaintApp::new());
                                    self.windows.push(win);
                                }
                            } else if sub == 3 {
                                // Games -> Snake, Minesweeper, Tetris, Pong, 2048, Sudoku, Chess
                                if self.mouse_y < sub_y + 20 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Snake", 150, 150, 400, 300);
                                    win.app = super::app::AppType::Snake(crate::gui::snake_app::SnakeApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 44 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Minesweeper", 120, 80, 240, 260);
                                    win.app = super::app::AppType::Minesweeper(crate::gui::minesweeper_app::MinesweeperApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 64 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Tetris", 100, 50, 300, 500);
                                    win.app = super::app::AppType::Tetris(crate::gui::tetris_app::TetrisApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 84 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Pong", 100, 50, 420, 320);
                                    win.app = super::app::AppType::Pong(crate::gui::pong_app::PongApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 104 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "2048", 100, 50, 300, 320);
                                    win.app = super::app::AppType::Game2048(crate::gui::game2048_app::Game2048App::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 124 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Sudoku", 100, 50, 300, 320);
                                    win.app = super::app::AppType::Sudoku(crate::gui::sudoku_app::SudokuApp::new());
                                    self.windows.push(win);
                                } else if self.mouse_y < sub_y + 144 {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Chess", 100, 50, 420, 440);
                                    win.app = super::app::AppType::Chess(crate::gui::chess_app::ChessApp::new());
                                    self.windows.push(win);
                                } else {
                                    let mut win = Window::new((self.windows.len() + 1) as u32, "Calculator", 100, 50, 300, 200);
                                    win.app = super::app::AppType::Calculator(crate::gui::calculator_app::CalculatorApp::new());
                                    self.windows.push(win);
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
                        // Clicked a category, do nothing (wait for submenu click)
                        handled = true;
                    }
                }

                // Check Start Button
                if !handled && self.mouse_x >= 2 && self.mouse_x <= 62 && self.mouse_y >= self.height as i32 - 28 && self.mouse_y <= self.height as i32 - 2 {
                    self.start_menu_open = !self.start_menu_open;
                    handled = true;
                } else if !handled && self.mouse_y >= self.height as i32 - 28 && self.mouse_y <= self.height as i32 - 2 {
                    // Check taskbar items
                    let mut tb_x = 70;
                    let mut clicked_idx = None;
                    for (i, _w) in self.windows.iter().enumerate() {
                        let item_w = 100;
                        if self.mouse_x >= tb_x && self.mouse_x <= tb_x + item_w {
                            clicked_idx = Some(i);
                            break;
                        }
                        tb_x += item_w + 4;
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
                            let local_x = self.mouse_x - w.x;
                            let local_y = self.mouse_y - w.y;
                            w.app.on_mouse_event(local_x, local_y, mev.buttons);
                            
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
                                match app_name.as_str() {
                                    "Terminal" => win.app = super::app::AppType::Terminal(TerminalApp::new()),
                                    "File Manager" => win.app = super::app::AppType::FileManager(crate::gui::file_manager_app::FileManagerApp::new()),
                                    "Settings" => win.app = super::app::AppType::Settings(crate::gui::settings_app::SettingsApp::new()),
                                    "Task Manager" => win.app = super::app::AppType::TaskManager(crate::gui::task_manager_app::TaskManagerApp::new()),
                                    "Sys Monitor" => win.app = super::app::AppType::SysMon(crate::gui::sysmon_app::SysMonApp::new()),
                                    "Notepad" => win.app = super::app::AppType::Notepad(NotepadApp::new()),
                                    "Calculator" => win.app = super::app::AppType::Calculator(crate::gui::calculator_app::CalculatorApp::new()),
                                    "Clock" => win.app = super::app::AppType::Clock(crate::gui::clock_app::ClockApp::new()),
                                    "Calendar" => win.app = super::app::AppType::Calendar(crate::gui::calendar_app::CalendarApp::new()),
                                    "Paint" => win.app = super::app::AppType::Paint(crate::gui::paint_app::PaintApp::new()),
                                    "Snake" => win.app = super::app::AppType::Snake(crate::gui::snake_app::SnakeApp::new()),
                                    "Minesweeper" => win.app = super::app::AppType::Minesweeper(crate::gui::minesweeper_app::MinesweeperApp::new()),
                                    "Tetris" => win.app = super::app::AppType::Tetris(crate::gui::tetris_app::TetrisApp::new()),
                                    "Pong" => win.app = super::app::AppType::Pong(crate::gui::pong_app::PongApp::new()),
                                    "2048" => win.app = super::app::AppType::Game2048(crate::gui::game2048_app::Game2048App::new()),
                                    "Sudoku" => win.app = super::app::AppType::Sudoku(crate::gui::sudoku_app::SudokuApp::new()),
                                    "Chess" => win.app = super::app::AppType::Chess(crate::gui::chess_app::ChessApp::new()),
                                    "Browser" => win.app = super::app::AppType::Browser(crate::gui::browser_app::BrowserApp::new()),
                                    _ => {}
                                }
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
        
        // Check keyboard for exit
        if let Some(c) = keyboard::pop_char() {
            if c == '\x1B' { // ESC
                return false;
            } else {
                if let Some(w) = self.windows.last_mut() {
                    w.app.on_key_event(c);
                    needs_redraw = true;
                }
            }
        }
        
        self.draw();
        
        true
    }
}
