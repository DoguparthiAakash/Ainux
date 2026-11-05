use alloc::vec::Vec;
use crate::drivers::video;
use crate::drivers::mouse;
use crate::drivers::keyboard;
use alloc::string::{String, ToString};
use super::window::{Window, WindowState};

// ─── Modern Dark Glassmorphism Palette ───────────────────────────────────────
// Base / Background
const C_BASE:        u32 = 0x0D1117; // Deep midnight — desktop background
const C_SURFACE:     u32 = 0x161B22; // Slightly lighter surface
const C_SURFACE2:    u32 = 0x1C2128; // Card / panel surface
const C_BORDER:      u32 = 0x30363D; // Subtle border
const C_BORDER_HL:   u32 = 0x484F58; // Highlighted border
// Accent
const C_ACCENT:      u32 = 0x7C3AED; // Electric purple (primary)
const C_ACCENT2:     u32 = 0x3B82F6; // Ocean blue (secondary)
const C_ACCENT_DIM:  u32 = 0x3D2065; // Dimmed purple for hover
// Text
const C_TEXT:        u32 = 0xE6EDF3; // Primary white text
const C_TEXT_MUTED:  u32 = 0x8B949E; // Muted grey text
const C_TEXT_DIM:    u32 = 0x484F58; // Very dim text
// Dock glass
const C_DOCK_BG:     u32 = 0x161B22; // Dock fill (simulates frosted glass)
const C_DOCK_BORDER: u32 = 0x30363D; // Dock border

/// Fast alpha-blend: src over dst, alpha is 0-255
#[inline(always)]
fn alpha_blend(dst: u32, src: u32, alpha: u32) -> u32 {
    let inv = 255 - alpha;
    let sr = (src >> 16) & 0xFF;
    let sg = (src >> 8)  & 0xFF;
    let sb =  src        & 0xFF;
    let dr = (dst >> 16) & 0xFF;
    let dg = (dst >> 8)  & 0xFF;
    let db =  dst        & 0xFF;
    let r = (sr * alpha + dr * inv) / 255;
    let g = (sg * alpha + dg * inv) / 255;
    let b = (sb * alpha + db * inv) / 255;
    (r << 16) | (g << 8) | b
}

/// Draw a horizontal gradient rect (left_color → right_color)
fn draw_gradient_h(buf: &mut [u32], bw: usize, bh: usize,
                   x: i32, y: i32, w: i32, h: i32,
                   c0: u32, c1: u32,
                   clip: Option<&crate::gui::clip::Rect>) {
    if w <= 0 || h <= 0 { return; }
    let mut x1 = x.max(0);
    let mut y1 = y.max(0);
    let mut x2 = (x + w).min(bw as i32);
    let mut y2 = (y + h).min(bh as i32);
    if let Some(c) = clip {
        x1 = x1.max(c.x); y1 = y1.max(c.y);
        x2 = x2.min(c.x + c.w); y2 = y2.min(c.y + c.h);
    }
    if x1 >= x2 || y1 >= y2 { return; }
    let r0 = (c0 >> 16) & 0xFF; let g0 = (c0 >> 8) & 0xFF; let b0 = c0 & 0xFF;
    let r1 = (c1 >> 16) & 0xFF; let g1 = (c1 >> 8) & 0xFF; let b1 = c1 & 0xFF;
    for sy in y1..y2 {
        for sx in x1..x2 {
            let t = ((sx - x) * 255 / w.max(1)) as u32;
            let r = (r0 * (255 - t) + r1 * t) / 255;
            let g = (g0 * (255 - t) + g1 * t) / 255;
            let b = (b0 * (255 - t) + b1 * t) / 255;
            buf[(sy * bw as i32 + sx) as usize] = (r << 16) | (g << 8) | b;
        }
    }
}

/// Draw a vertical gradient rect (top_color → bottom_color)
fn draw_gradient_v(buf: &mut [u32], bw: usize, bh: usize,
                   x: i32, y: i32, w: i32, h: i32,
                   c0: u32, c1: u32,
                   clip: Option<&crate::gui::clip::Rect>) {
    if w <= 0 || h <= 0 { return; }
    let mut x1 = x.max(0);
    let mut y1 = y.max(0);
    let mut x2 = (x + w).min(bw as i32);
    let mut y2 = (y + h).min(bh as i32);
    if let Some(c) = clip {
        x1 = x1.max(c.x); y1 = y1.max(c.y);
        x2 = x2.min(c.x + c.w); y2 = y2.min(c.y + c.h);
    }
    if x1 >= x2 || y1 >= y2 { return; }
    let r0 = (c0 >> 16) & 0xFF; let g0 = (c0 >> 8) & 0xFF; let b0 = c0 & 0xFF;
    let r1 = (c1 >> 16) & 0xFF; let g1 = (c1 >> 8) & 0xFF; let b1 = c1 & 0xFF;
    for sy in y1..y2 {
        let t = ((sy - y) * 255 / h.max(1)) as u32;
        let r = (r0 * (255 - t) + r1 * t) / 255;
        let g = (g0 * (255 - t) + g1 * t) / 255;
        let b = (b0 * (255 - t) + b1 * t) / 255;
        let row_color = (r << 16) | (g << 8) | b;
        for sx in x1..x2 {
            buf[(sy * bw as i32 + sx) as usize] = row_color;
        }
    }
}

/// Draw an alpha-blended BMP image scaled to (draw_w x draw_h)
fn draw_bmp_alpha(buf: &mut [u32], bw: usize, bh: usize,
                  bmp: &crate::gui::bmp::BmpImage, bx: i32, by: i32,
                  draw_w: usize, draw_h: usize,
                  clip: Option<&crate::gui::clip::Rect>) {
    if draw_w == 0 || draw_h == 0 || bmp.width == 0 || bmp.height == 0 { return; }
    let scale_x_mul = (bmp.width * 1024) / draw_w;
    let scale_y_mul = (bmp.height * 1024) / draw_h;

    for dy in 0..draw_h {
        let sy = by + dy as i32;
        if sy < 0 || sy >= bh as i32 { continue; }
        let src_y = (dy * scale_y_mul) / 1024;
        for dx in 0..draw_w {
            let sx = bx + dx as i32;
            if sx < 0 || sx >= bw as i32 { continue; }
            let src_x = (dx * scale_x_mul) / 1024;
            let color = bmp.data[src_y * bmp.width + src_x];
            let a = (color >> 24) & 0xFF;
            if a > 10 {
                if let Some(c) = clip {
                    if sx < c.x || sx >= c.x + c.w || sy < c.y || sy >= c.y + c.h {
                        continue;
                    }
                }
                let idx = (sy * bw as i32 + sx) as usize;
                if a >= 250 {
                    buf[idx] = color & 0x00FFFFFF;
                } else {
                    let bg = buf[idx];
                    let inv_a = 255 - a;
                    let r = (((color >> 16) & 0xFF) * a + ((bg >> 16) & 0xFF) * inv_a) / 255;
                    let g = (((color >> 8) & 0xFF) * a + ((bg >> 8) & 0xFF) * inv_a) / 255;
                    let b = ((color & 0xFF) * a + (bg & 0xFF) * inv_a) / 255;
                    buf[idx] = (r << 16) | (g << 8) | b;
                }
            }
        }
    }
}

/// Draw a window button icon from a BmpImage (20x20 scaled)
fn draw_bmp_button(buf: &mut [u32], bw: usize, bh: usize,
                   bmp: &crate::gui::bmp::BmpImage, bx: i32, by: i32,
                   clip: Option<&crate::gui::clip::Rect>) {
    let draw_w = core::cmp::min(bmp.width, 20);
    let draw_h = core::cmp::min(bmp.height, 20);
    if draw_w == 0 || draw_h == 0 { return; }
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
                if sy >= 0 && sy < bh as i32 && sx >= 0 && sx < bw as i32 {
                    if let Some(c) = clip {
                        if sx >= c.x && sx < c.x + c.w && sy >= c.y && sy < c.y + c.h {
                            buf[(sy * bw as i32 + sx) as usize] = color;
                        }
                    } else {
                        buf[(sy * bw as i32 + sx) as usize] = color;
                    }
                }
            }
        }
    }
}

/// Draw a 1px border rectangle outline
fn draw_rect_border(buf: &mut [u32], bw: usize, bh: usize,
                    x: i32, y: i32, w: i32, h: i32, color: u32,
                    clip: Option<&crate::gui::clip::Rect>) {
    fill_rect_buffer(buf, bw, bh, x,       y,       w, 1, color, clip); // top
    fill_rect_buffer(buf, bw, bh, x,       y + h-1, w, 1, color, clip); // bottom
    fill_rect_buffer(buf, bw, bh, x,       y,       1, h, color, clip); // left
    fill_rect_buffer(buf, bw, bh, x + w-1, y,       1, h, color, clip); // right
}

// Filled circle for macOS traffic lights & dock indicator dots
fn draw_circle_filled(buf: &mut [u32], bw: usize, bh: usize, cx: i32, cy: i32, r: i32, color: u32, clip: Option<&crate::gui::clip::Rect>) {
    let r2 = r * r;
    for dy in -r..=r {
        let py = cy + dy;
        if py < 0 || py >= bh as i32 { continue; }
        for dx in -r..=r {
            if dx * dx + dy * dy <= r2 {
                let px = cx + dx;
                if px < 0 || px >= bw as i32 { continue; }
                if let Some(c) = clip {
                    if px < c.x || px >= c.x + c.w || py < c.y || py >= c.y + c.h {
                        continue;
                    }
                }
                buf[(py * bw as i32 + px) as usize] = color;
            }
        }
    }
}

// Circle border outline
fn draw_circle_border(buf: &mut [u32], bw: usize, bh: usize, cx: i32, cy: i32, r: i32, color: u32, clip: Option<&crate::gui::clip::Rect>) {
    let r2 = r * r;
    let inner_r2 = (r - 1) * (r - 1);
    for dy in -r..=r {
        let py = cy + dy;
        if py < 0 || py >= bh as i32 { continue; }
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 <= r2 && d2 >= inner_r2 {
                let px = cx + dx;
                if px < 0 || px >= bw as i32 { continue; }
                if let Some(c) = clip {
                    if px < c.x || px >= c.x + c.w || py < c.y || py >= c.y + c.h {
                        continue;
                    }
                }
                buf[(py * bw as i32 + px) as usize] = color;
            }
        }
    }
}

// Utility to draw strings using the multi-font engine
fn draw_string_to_buffer(buf: &mut [u32], buf_w: usize, buf_h: usize, x: i32, y: i32, s: &str, fg: u32, clip: Option<&crate::gui::clip::Rect>) {
    let font = crate::gui::fonts::get_current_font();
    crate::gui::fonts::draw_string_to_buffer(buf, buf_w, buf_h, x, y, s, fg, font, clip);
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

/// Alpha-blend a filled rect over existing pixels (alpha 0-255)
fn fill_rect_alpha(buf: &mut [u32], bw: usize, bh: usize,
                   rx: i32, ry: i32, rw: i32, rh: i32,
                   color: u32, alpha: u32,
                   clip: Option<&crate::gui::clip::Rect>) {
    let mut x1 = rx.max(0);
    let mut y1 = ry.max(0);
    let mut x2 = (rx + rw).min(bw as i32);
    let mut y2 = (ry + rh).min(bh as i32);
    if let Some(c) = clip {
        x1 = x1.max(c.x); y1 = y1.max(c.y);
        x2 = x2.min(c.x + c.w); y2 = y2.min(c.y + c.h);
    }
    if x1 >= x2 || y1 >= y2 { return; }
    for sy in y1..y2 {
        for sx in x1..x2 {
            let idx = (sy * bw as i32 + sx) as usize;
            buf[idx] = alpha_blend(buf[idx], color, alpha);
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
    pub logo: crate::gui::bmp::BmpImage,
    pub active_edge: Option<u8>,
    pub last_redraw_ticks: u64,
    pub last_frame_rects: alloc::vec::Vec<crate::gui::clip::Rect>,
    pub start_menu_anim: u8,
    pub anim_ticks: u32,
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
        
        // Deduplicated icon list — one entry per unique app, 6 per column
        let mut add_icon = |name: &str, raw_data: &[u8]| {
            if let Some(bmp) = crate::gui::bmp::BmpImage::parse(raw_data) {
                let count = desktop_icons.len() as i32;
                let col = count / 8; // Max 8 icons per column
                let row = count % 8;
                desktop_icons.push(DesktopIcon {
                    name: alloc::string::String::from(name),
                    x: 16 + col * 76,
                    y: 16 + row * 76,
                    bmp,
                });
            }
        };
        
        // Unique apps only (no duplicates)
        add_icon("Terminal",   include_bytes!("../../icons/terminal.bmp"));
        add_icon("Settings",   include_bytes!("../../icons/settings.bmp"));
        add_icon("File Mgr",   include_bytes!("../../icons/fileexplorer.bmp"));
        add_icon("Notepad",    include_bytes!("../../icons/notepad.bmp"));
        add_icon("Clock",      include_bytes!("../../icons/clock.bmp"));
        add_icon("Calendar",   include_bytes!("../../icons/calander.bmp"));
        add_icon("Paint",      include_bytes!("../../icons/images.bmp"));
        add_icon("Browser",    include_bytes!("../../icons/bookmark.bmp"));
        add_icon("Calculator", include_bytes!("../../icons/sheetviewer.bmp"));
        add_icon("Sys Mon",    include_bytes!("../../icons/controlcenter.bmp"));
        add_icon("Task Mgr",   include_bytes!("../../icons/discmanagement.bmp"));
        add_icon("Snake",      include_bytes!("../../icons/game.bmp"));
        add_icon("Mines",      include_bytes!("../../icons/mine.bmp"));
        add_icon("Tetris",     include_bytes!("../../icons/tetris.bmp"));
        add_icon("Chess",      include_bytes!("../../icons/game.bmp"));
        add_icon("Pong",       include_bytes!("../../icons/pong.bmp"));
        add_icon("2048",       include_bytes!("../../icons/game.bmp"));
        add_icon("Sudoku",     include_bytes!("../../icons/game.bmp"));
        
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
            logo: crate::gui::bmp::BmpImage::parse(include_bytes!("../../icons/mithl_logo.bmp")).unwrap_or_else(|| crate::gui::bmp::BmpImage { width: 96, height: 64, data: alloc::vec![0; 96 * 64] }),
            active_edge: None,
            last_redraw_ticks: 0,
            last_frame_rects: alloc::vec::Vec::new(),
            start_menu_anim: 0,
            anim_ticks: 0,
        }
    }

    pub fn add_window(&mut self, mut w: Window) {
        w.anim_progress = 60;
        w.is_animating = true;
        self.windows.push(w);
        self.full_redraw = true;
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
        let cursor_x = self.mouse_x.clamp(0, self.width as i32 - 1);
        let cursor_y = self.mouse_y.clamp(0, self.height as i32 - 1);
        let mut cursor_backup = [0u32; 32 * 32];
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

        // ── 1. Wallpaper — Dark gradient (top-dark → bottom-slightly-lighter) ──────
        let base_color = *crate::gui::settings_app::WALLPAPER_COLOR.lock();
        // Blend from base_color at top toward a slightly lighter variant at bottom
        let br = (base_color >> 16) & 0xFF;
        let bg = (base_color >> 8)  & 0xFF;
        let bb =  base_color        & 0xFF;
        // bottom: add ~0x18 to each channel, clamped
        let er = (br + 0x18).min(0xFF);
        let eg = (bg + 0x18).min(0xFF);
        let eb = (bb + 0x18).min(0xFF);
        let end_color = (er << 16) | (eg << 8) | eb;
        draw_gradient_v(&mut self.backbuffer, self.width, self.height,
            0, 0, self.width as i32, self.height as i32,
            base_color, end_color, clip_opt);

        // ── 1.5. Desktop Icons ────────────────────────────────────────────────────
        for icon in &self.desktop_icons {
            let draw_w = core::cmp::min(icon.bmp.width, 48);
            let draw_h = core::cmp::min(icon.bmp.height, 48);
            let scale_x_mul = if draw_w > 0 { (icon.bmp.width * 1024) / draw_w } else { 1024 };
            let scale_y_mul = if draw_h > 0 { (icon.bmp.height * 1024) / draw_h } else { 1024 };

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
            // Icon label with subtle dark shadow for readability on dark wallpapers
            let text_x = icon.x + (draw_w as i32 / 2) - (icon.name.len() as i32 * 4);
            let ty = icon.y + draw_h as i32 + 4;
            let current_font = crate::gui::fonts::get_current_font();
            crate::gui::fonts::draw_string_with_shadow(&mut self.backbuffer, self.width, self.height, text_x, ty, &icon.name, C_TEXT, 0x000000, current_font, clip_opt);
        }

        // ── 2. Windows — Modern macOS Frosted Glass & Diffuse Shadows ─────────────
        for w in &mut self.windows {
            if w.state == WindowState::Minimized { continue; }

            let title_h = 24i32;

            // 2a. Multi-stage diffuse soft drop shadow (macOS aesthetic)
            fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                w.x + 8, w.y - title_h + 8, w.width + 6, w.height + title_h + 6,
                0x000000, 35, clip_opt);
            fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                w.x + 5, w.y - title_h + 5, w.width + 4, w.height + title_h + 4,
                0x000000, 65, clip_opt);
            fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                w.x + 2, w.y - title_h + 2, w.width + 2, w.height + title_h + 2,
                0x000000, 110, clip_opt);

            // 2b. Outer subtle border
            draw_rect_border(&mut self.backbuffer, self.width, self.height,
                w.x - 1, w.y - title_h - 1, w.width + 2, w.height + title_h + 2,
                0x334155, clip_opt);

            // 2c. Window frame — dark surface
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height,
                w.x, w.y - title_h, w.width, w.height + title_h,
                C_SURFACE, clip_opt);

            // 2d. Title bar — frosted glass gradient
            draw_gradient_v(&mut self.backbuffer, self.width, self.height,
                w.x, w.y - title_h, w.width, title_h,
                0x283046, 0x1E2235, clip_opt);

            // Glass highlight line across titlebar top
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height,
                w.x, w.y - title_h, w.width, 1, 0x475569, clip_opt);

            // 2e. Title bar bottom separator line
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height,
                w.x, w.y - 1, w.width, 1, C_BORDER, clip_opt);

            // 2f. macOS Traffic Light Controls (Top-Left)
            let cy = w.y - 12;
            let hover_traffic = self.mouse_x >= w.x + 6 && self.mouse_x <= w.x + 58 &&
                                self.mouse_y >= w.y - title_h && self.mouse_y <= w.y;

            // Close button (Red)
            let red_x = w.x + 14;
            draw_circle_filled(&mut self.backbuffer, self.width, self.height, red_x, cy, 5, 0xFF5F56, clip_opt);
            draw_circle_border(&mut self.backbuffer, self.width, self.height, red_x, cy, 5, 0xE0443E, clip_opt);

            // Minimize button (Yellow)
            let yel_x = w.x + 31;
            draw_circle_filled(&mut self.backbuffer, self.width, self.height, yel_x, cy, 5, 0xFFBD2E, clip_opt);
            draw_circle_border(&mut self.backbuffer, self.width, self.height, yel_x, cy, 5, 0xDEA123, clip_opt);

            // Maximize button (Green)
            let grn_x = w.x + 48;
            draw_circle_filled(&mut self.backbuffer, self.width, self.height, grn_x, cy, 5, 0x27C93F, clip_opt);
            draw_circle_border(&mut self.backbuffer, self.width, self.height, grn_x, cy, 5, 0x1AAB29, clip_opt);

            if hover_traffic {
                // 'x' glyph on red
                for i in -2..=2 {
                    let rx = red_x + i;
                    let ry1 = cy + i;
                    let ry2 = cy - i;
                    if rx >= 0 && rx < self.width as i32 {
                        if ry1 >= 0 && ry1 < self.height as i32 {
                            self.backbuffer[(ry1 * self.width as i32 + rx) as usize] = 0x4D0000;
                        }
                        if ry2 >= 0 && ry2 < self.height as i32 {
                            self.backbuffer[(ry2 * self.width as i32 + rx) as usize] = 0x4D0000;
                        }
                    }
                }
                // '-' glyph on yellow
                for dx in -2..=2 {
                    let yx = yel_x + dx;
                    if yx >= 0 && yx < self.width as i32 && cy >= 0 && cy < self.height as i32 {
                        self.backbuffer[(cy * self.width as i32 + yx) as usize] = 0x5A4000;
                    }
                }
                // '+' glyph on green
                for dx in -2..=2 {
                    let gx = grn_x + dx;
                    if gx >= 0 && gx < self.width as i32 && cy >= 0 && cy < self.height as i32 {
                        self.backbuffer[(cy * self.width as i32 + gx) as usize] = 0x00450A;
                    }
                }
                for dy in -2..=2 {
                    let gy = cy + dy;
                    if grn_x >= 0 && grn_x < self.width as i32 && gy >= 0 && gy < self.height as i32 {
                        self.backbuffer[(gy * self.width as i32 + grn_x) as usize] = 0x00450A;
                    }
                }
            }

            // 2g. Window title string (centered macOS style with typography shadow)
            let title_pixel_len = w.title.len() as i32 * 8;
            let title_x = (w.x + (w.width - title_pixel_len) / 2).max(w.x + 64);
            let title_y = w.y - title_h + 5;
            let current_font = crate::gui::fonts::get_current_font();
            crate::gui::fonts::draw_string_with_shadow(&mut self.backbuffer, self.width, self.height,
                title_x, title_y, &w.title, C_TEXT, 0x000000, current_font, clip_opt);

            // 2h. Resize handle (bottom-right, subtle grid dots)
            let res_x = w.x + w.width - 12;
            let res_y = w.y + w.height - 12;
            for rr in 0..3 {
                for rc in 0..3 {
                    if rr + rc >= 2 { // Only bottom-right triangle pattern
                        let px = res_x + rc * 4;
                        let py = res_y + rr * 4;
                        fill_rect_buffer(&mut self.backbuffer, self.width, self.height,
                            px, py, 2, 2, C_BORDER_HL, clip_opt);
                    }
                }
            }

            // 2i. Window content blit
            let mut bx1 = w.x.max(0);
            let mut by1 = w.y.max(0);
            let mut bx2 = (w.x + w.width).min(self.width as i32);
            let mut by2 = (w.y + w.height).min(self.height as i32);
            if let Some(c) = clip_opt {
                bx1 = bx1.max(c.x); by1 = by1.max(c.y);
                bx2 = bx2.min(c.x + c.w); by2 = by2.min(c.y + c.h);
            }
            if bx1 < bx2 && by1 < by2 {
                if w.buffer_ptr != 0 && w.buffer_width > 0 && w.buffer_height > 0 {
                    let src_ptr = w.buffer_ptr as *const u32;
                    let copy_width = (bx2 - bx1) as usize;
                    for sy in by1..by2 {
                        let dy = sy - w.y;
                        let dst_idx_start = (sy * self.width as i32 + bx1) as usize;
                        let dx_start = bx1 - w.x;
                        if dy < w.buffer_height {
                            let valid_copy_w = if dx_start < w.buffer_width {
                                (w.buffer_width - dx_start).min(copy_width as i32).max(0) as usize
                            } else {
                                0
                            };
                            let src_idx = (dy * w.buffer_width + dx_start) as usize;
                            if valid_copy_w > 0 && src_idx + valid_copy_w <= w.buffer_len && dst_idx_start + valid_copy_w <= self.backbuffer.len() {
                                unsafe {
                                    let src_slice = core::slice::from_raw_parts(src_ptr.add(src_idx), valid_copy_w);
                                    self.backbuffer[dst_idx_start..dst_idx_start + valid_copy_w].copy_from_slice(src_slice);
                                }
                            }
                            if valid_copy_w < copy_width {
                                for col in valid_copy_w..copy_width {
                                    let idx = dst_idx_start + col;
                                    if idx < self.backbuffer.len() {
                                        self.backbuffer[idx] = C_SURFACE;
                                    }
                                }
                            }
                        } else {
                            for col in 0..copy_width {
                                let idx = dst_idx_start + col;
                                if idx < self.backbuffer.len() {
                                    self.backbuffer[idx] = C_SURFACE;
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── 3. Taskbar / Dock — Glassmorphism ────────────────────────────────────
        let dock_pos = *crate::gui::settings_app::DOCK_POSITION.lock();

        let num_items = self.windows.len() as i32 + 1; // +1 for Start
        let item_w   = 120i32;
        let item_h   = 32i32;
        let padding  = 5i32;
        let v_item   = 44i32;

        let (dock_w, dock_h, dock_x, dock_y, step_x, step_y);
        if dock_pos == 0 || dock_pos == 1 {
            dock_w = num_items * (item_w + padding) + padding;
            dock_h = item_h + padding * 2 + 4;
            dock_x = ((self.width as i32 - dock_w) / 2).max(0);
            dock_y = if dock_pos == 0 { 8 } else { self.height as i32 - dock_h - 8 };
            step_x = item_w + padding;
            step_y = 0;
        } else {
            dock_w = v_item + padding * 2;
            dock_h = num_items * (v_item + padding) + padding;
            dock_x = if dock_pos == 2 { 8 } else { self.width as i32 - dock_w - 8 };
            dock_y = ((self.height as i32 - dock_h) / 2).max(0);
            step_x = 0;
            step_y = v_item + padding;
        }

        // Dock glass background (semi-transparent dark surface)
        fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
            dock_x, dock_y, dock_w, dock_h, C_DOCK_BG, 210, clip_opt);
        // Dock border — thin accent
        draw_rect_border(&mut self.backbuffer, self.width, self.height,
            dock_x, dock_y, dock_w, dock_h, C_DOCK_BORDER, clip_opt);
        // Top highlight line (simulates glass sheen)
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height,
            dock_x + 1, dock_y + 1, dock_w - 2, 1, C_BORDER_HL, clip_opt);

        let mut curr_x = dock_x + padding;
        let mut curr_y = dock_y + padding;

        // ── Start button (accent gradient pill) ───────────────────────────────
        let start_w = if dock_pos <= 1 { item_w } else { v_item };
        let start_h = if dock_pos <= 1 { item_h } else { v_item };
        let (sc0, sc1) = if self.start_menu_open {
            (C_ACCENT, C_ACCENT_DIM)
        } else {
            (0x2D1B69, C_ACCENT)
        };
        draw_gradient_h(&mut self.backbuffer, self.width, self.height,
            curr_x, curr_y, start_w, start_h, sc0, sc1, clip_opt);
        draw_rect_border(&mut self.backbuffer, self.width, self.height,
            curr_x, curr_y, start_w, start_h, C_ACCENT, clip_opt);
        // Start button label & Mithl OS logo
        if start_w >= 80 {
            let logo_w = 24i32;
            let logo_h = 16i32;
            let slabel = "Mithl OS";
            let text_w = slabel.len() as i32 * 8;
            let total_w = logo_w + 6 + text_w;
            let content_x = curr_x + (start_w - total_w) / 2;
            let logo_y = curr_y + (start_h - logo_h) / 2;
            let text_y = curr_y + (start_h - 16) / 2;
            draw_bmp_alpha(&mut self.backbuffer, self.width, self.height,
                &self.logo, content_x, logo_y, logo_w as usize, logo_h as usize, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height,
                content_x + logo_w + 6, text_y, slabel, C_TEXT, clip_opt);
        } else {
            let logo_w = 26i32;
            let logo_h = 18i32;
            let logo_x = curr_x + (start_w - logo_w) / 2;
            let logo_y = curr_y + (start_h - logo_h) / 2;
            draw_bmp_alpha(&mut self.backbuffer, self.width, self.height,
                &self.logo, logo_x, logo_y, logo_w as usize, logo_h as usize, clip_opt);
        }
        curr_x += step_x;
        curr_y += step_y;

        // ── Window buttons ────────────────────────────────────────────────────
        for (i, win) in self.windows.iter().enumerate() {
            let iw = if dock_pos <= 1 { item_w } else { v_item };
            let ih = if dock_pos <= 1 { item_h } else { v_item };
            let is_active = i == self.windows.len() - 1 && win.state != WindowState::Minimized;

            // macOS hover elevation & bounce animation
            let is_hover = self.mouse_x >= curr_x && self.mouse_x < curr_x + iw &&
                           self.mouse_y >= curr_y && self.mouse_y < curr_y + ih;
            let bounce = if win.is_animating { if (self.anim_ticks / 3) % 2 == 0 { -4 } else { -1 } } else { 0 };
            let hover_offset = if is_hover { -2 } else { 0 };
            let item_y = curr_y + hover_offset + bounce;

            let (bg0, bg1) = if is_active {
                (C_SURFACE2, 0x253040)
            } else if is_hover {
                (0x1E293B, 0x0F172A)
            } else {
                (0x0F141A, C_SURFACE)
            };
            draw_gradient_v(&mut self.backbuffer, self.width, self.height,
                curr_x, item_y, iw, ih, bg0, bg1, clip_opt);
            draw_rect_border(&mut self.backbuffer, self.width, self.height,
                curr_x, item_y, iw, ih,
                if is_active { C_ACCENT2 } else if is_hover { 0x64748B } else { C_BORDER }, clip_opt);

            // Active running app indicator dot (macOS style glow dot)
            if dock_pos <= 1 {
                let dot_x = curr_x + iw / 2;
                let dot_y = curr_y + ih + 2;
                draw_circle_filled(&mut self.backbuffer, self.width, self.height, dot_x, dot_y, 2,
                    if is_active { 0x38BDF8 } else { 0x64748B }, clip_opt);
            } else {
                let dot_x = curr_x + iw + 2;
                let dot_y = curr_y + ih / 2;
                draw_circle_filled(&mut self.backbuffer, self.width, self.height, dot_x, dot_y, 2,
                    if is_active { 0x38BDF8 } else { 0x64748B }, clip_opt);
            }

            // Icon in dock item
            let icon_name = match win.title.as_str() {
                "Mithl Terminal"  => "Terminal",
                "MithlFS Explorer"=> "File Mgr",
                "System Monitor"  => "Sys Mon",
                "Minesweeper"     => "Mines",
                "Task Manager"    => "Task Mgr",
                "Notepad"         => "Notepad",
                "Clock"           => "Clock",
                "Calendar"        => "Calendar",
                "Tetris"          => "Tetris",
                "Pong"            => "Pong",
                "Settings"        => "Settings",
                other             => other,
            };
            if let Some(icon) = self.desktop_icons.iter().find(|ic| ic.name == icon_name) {
                let dw = 16usize;
                let dh = 16usize;
                let sxm = (icon.bmp.width * 1024) / dw;
                let sym2 = (icon.bmp.height * 1024) / dh;
                let ix = if dock_pos <= 1 { curr_x + 6 } else { curr_x + (v_item / 2) - 8 };
                let iy = if dock_pos <= 1 { item_y + 8 } else { item_y + (v_item / 2) - 8 };
                for dy_bmp in 0..dh {
                    let bsy = iy + dy_bmp as i32;
                    if bsy < 0 || bsy >= self.height as i32 { continue; }
                    let src_y = (dy_bmp * sym2) / 1024;
                    for dx_bmp in 0..dw {
                        let bsx = ix + dx_bmp as i32;
                        if bsx < 0 || bsx >= self.width as i32 { continue; }
                        let src_x = (dx_bmp * sxm) / 1024;
                        let c = icon.bmp.data[src_y * icon.bmp.width + src_x];
                        if c & 0xFF000000 != 0 && c != 0xFFFFFFFF {
                            if let Some(cl) = clip_opt {
                                if bsx >= cl.x && bsx < cl.x + cl.w && bsy >= cl.y && bsy < cl.y + cl.h {
                                    self.backbuffer[(bsy * self.width as i32 + bsx) as usize] = c;
                                }
                            } else {
                                self.backbuffer[(bsy * self.width as i32 + bsx) as usize] = c;
                            }
                        }
                    }
                }
            }

            if dock_pos <= 1 {
                let mut td = alloc::string::String::new();
                for (j, c) in win.title.chars().enumerate() { if j > 10 { break; } td.push(c); }
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height,
                    curr_x + 26, item_y + ih / 2 - 4, &td,
                    if is_active { C_TEXT } else { C_TEXT_MUTED }, clip_opt);
            }
            curr_x += step_x;
            curr_y += step_y;
        }

        // ── 4. Start Menu — Modern Dark Glass ────────────────────────────────────
        if self.start_menu_open {
            let sm_w = 220i32;
            let sm_h = 280i32;
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

            // Glass background
            fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                sm_x, sm_y, sm_w, sm_h, C_SURFACE, 230, clip_opt);
            draw_rect_border(&mut self.backbuffer, self.width, self.height,
                sm_x, sm_y, sm_w, sm_h, C_ACCENT, clip_opt);

            // Header bar
            draw_gradient_v(&mut self.backbuffer, self.width, self.height,
                sm_x, sm_y, sm_w, 36, 0x1E1030, C_SURFACE2, clip_opt);
            draw_bmp_alpha(&mut self.backbuffer, self.width, self.height,
                &self.logo, sm_x + 12, sm_y + 9, 27, 18, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height,
                sm_x + 46, sm_y + 10, "Mithl", C_ACCENT, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height,
                sm_x + 46 + 5*8, sm_y + 10, " OS", C_TEXT, clip_opt);
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height,
                sm_x, sm_y + 36, sm_w, 1, C_BORDER, clip_opt);

            // Menu entries with hover highlight
            let entries: &[(&str, u32)] = &[
                ("System >",       1),
                ("Accessories >",  2),
                ("Games >",        3),
            ];
            let entry_h = 32i32;
            for (idx, (label, sub_id)) in entries.iter().enumerate() {
                let ey = sm_y + 44 + idx as i32 * entry_h;
                let is_hov = self.active_submenu == Some(*sub_id);
                if is_hov {
                    fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                        sm_x + 4, ey, sm_w - 8, entry_h - 2, C_ACCENT_DIM, 180, clip_opt);
                    fill_rect_buffer(&mut self.backbuffer, self.width, self.height,
                        sm_x + 4, ey + entry_h - 2, sm_w - 8, 1, C_ACCENT, clip_opt);
                }
                let fg = if is_hov { C_TEXT } else { C_TEXT_MUTED };
                draw_string_to_buffer(&mut self.backbuffer, self.width, self.height,
                    sm_x + 16, ey + 8, label, fg, clip_opt);
            }

            // Divider + Action buttons (Restart & Power Off)
            fill_rect_buffer(&mut self.backbuffer, self.width, self.height,
                sm_x, sm_y + sm_h - 44, sm_w, 1, C_BORDER, clip_opt);
            fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                sm_x, sm_y + sm_h - 44, sm_w, 44, 0x111625, 230, clip_opt);

            let btn_y = sm_y + sm_h - 36;
            let btn_h = 28i32;
            let btn_w = (sm_w - 24) / 2; // 98px

            // Restart button (amber)
            let r_x = sm_x + 8;
            let r_hover = self.mouse_x >= r_x && self.mouse_x < r_x + btn_w && self.mouse_y >= btn_y && self.mouse_y < btn_y + btn_h;
            fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                r_x, btn_y, btn_w, btn_h, if r_hover { 0x452B05 } else { 0x2A1B07 }, 220, clip_opt);
            draw_rect_border(&mut self.backbuffer, self.width, self.height,
                r_x, btn_y, btn_w, btn_h, if r_hover { 0xF59E0B } else { 0xD97706 }, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height,
                r_x + 18, btn_y + 6, "Restart", 0xFCD34D, clip_opt);

            // Power Off button (red)
            let p_x = sm_x + sm_w - 8 - btn_w;
            let p_hover = self.mouse_x >= p_x && self.mouse_x < p_x + btn_w && self.mouse_y >= btn_y && self.mouse_y < btn_y + btn_h;
            fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                p_x, btn_y, btn_w, btn_h, if p_hover { 0x501015 } else { 0x350A0E }, 220, clip_opt);
            draw_rect_border(&mut self.backbuffer, self.width, self.height,
                p_x, btn_y, btn_w, btn_h, if p_hover { 0xEF4444 } else { 0xDC2626 }, clip_opt);
            draw_string_to_buffer(&mut self.backbuffer, self.width, self.height,
                p_x + 12, btn_y + 6, "Power Off", 0xFCA5A5, clip_opt);

            // Submenus
            if let Some(sub) = self.active_submenu {
                let sub_x = sm_x + sm_w + 2;
                let sub_w = 170i32;
                let sub_y = sm_y + 44 + (sub as i32 - 1) * 32;

                let items: &[&str] = match sub {
                    1 => &["Terminal", "File Manager", "Settings", "Task Manager", "Sys Monitor", "Browser"],
                    2 => &["Notepad", "Calculator", "Clock", "Calendar", "Paint"],
                    3 => &["Snake", "Minesweeper", "Tetris", "Pong", "2048", "Sudoku", "Chess"],
                    _ => &[],
                };
                let sh = items.len() as i32 * 28 + 8;
                fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                    sub_x, sub_y, sub_w, sh, C_SURFACE, 230, clip_opt);
                draw_rect_border(&mut self.backbuffer, self.width, self.height,
                    sub_x, sub_y, sub_w, sh, C_BORDER, clip_opt);
                for (si, item) in items.iter().enumerate() {
                    draw_string_to_buffer(&mut self.backbuffer, self.width, self.height,
                        sub_x + 12, sub_y + 8 + si as i32 * 28, item, C_TEXT, clip_opt);
                }
            }
        }

        // ── 5. Context Menu — Windows 11 Dark Acrylic ─────────────────────────────
        if self.context_menu_open {
            let cm_w = 180i32;
            let is_icon = self.context_menu_target.is_some();
            let cm_h = if is_icon { 104i32 } else { 276i32 };

            // Drop shadow
            fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                self.context_menu_x + 3, self.context_menu_y + 3, cm_w, cm_h, 0x000000, 120, clip_opt);

            // Acrylic dark container
            fill_rect_alpha(&mut self.backbuffer, self.width, self.height,
                self.context_menu_x, self.context_menu_y, cm_w, cm_h, C_SURFACE2, 245, clip_opt);
            draw_rect_border(&mut self.backbuffer, self.width, self.height,
                self.context_menu_x, self.context_menu_y, cm_w, cm_h, 0x3E4059, clip_opt);

            let mut curr_y = self.context_menu_y + 6;
            let items: &[(&str, bool)] = if is_icon {
                &[
                    ("Open", false),
                    ("Open in Terminal", false),
                    ("", true),
                    ("Remove from Desktop", false),
                    ("", true),
                    ("Properties", false),
                ]
            } else {
                &[
                    ("View: Auto-Arrange", false),
                    ("Refresh Desktop", false),
                    ("", true),
                    ("Change Font", false),
                    ("Next Wallpaper", false),
                    ("Display Settings", false),
                    ("", true),
                    ("New Text Document", false),
                    ("New Folder", false),
                    ("", true),
                    ("Open in Terminal", false),
                    ("Task Manager", false),
                    ("System Monitor", false),
                    ("", true),
                    ("Restart System", false),
                    ("Shut Down", false),
                ]
            };

            for &(label, is_sep) in items {
                if is_sep {
                    fill_rect_buffer(&mut self.backbuffer, self.width, self.height,
                        self.context_menu_x + 8, curr_y + 2, cm_w - 16, 1, 0x34374E, clip_opt);
                    curr_y += 6;
                } else {
                    let is_hover = self.mouse_x >= self.context_menu_x + 4 &&
                                   self.mouse_x <= self.context_menu_x + cm_w - 4 &&
                                   self.mouse_y >= curr_y &&
                                   self.mouse_y < curr_y + 20;
                    if is_hover {
                        fill_rect_buffer(&mut self.backbuffer, self.width, self.height,
                            self.context_menu_x + 4, curr_y, cm_w - 8, 20, 0x3B4252, clip_opt);
                        draw_string_to_buffer(&mut self.backbuffer, self.width, self.height,
                            self.context_menu_x + 12, curr_y + 2, label, 0xFFFFFF, clip_opt);
                    } else {
                        draw_string_to_buffer(&mut self.backbuffer, self.width, self.height,
                            self.context_menu_x + 12, curr_y + 2, label, C_TEXT, clip_opt);
                    }
                    curr_y += 20;
                }
            }
        }

        // Blitting is handled centrally inside draw() to support the software cursor overlay.
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
        
        let mut needs_redraw = self.full_redraw;
        let mut mouse_moved = false;
        
        while let Some(mev) = mouse::pop_event() {
            if self.mouse_x != mev.x as i32 || self.mouse_y != mev.y as i32 {
                self.mouse_x = mev.x as i32;
                self.mouse_y = mev.y as i32;
                mouse_moved = true;
                if self.context_menu_open {
                    needs_redraw = true;
                }
            }
            
            // Handle hovering over start menu categories
            if self.start_menu_open {
                let dock_pos = *crate::gui::settings_app::DOCK_POSITION.lock();
                let num_items = self.windows.len() as i32 + 1;
                let item_w = 120;
                let item_h = 32;
                let padding = 5;
                let v_item = 44;
                let (dock_w, dock_h, dock_x, dock_y) = if dock_pos <= 1 {
                    let dw = num_items * (item_w + padding) + padding;
                    let dh = item_h + padding * 2 + 4;
                    let dx = ((self.width as i32 - dw) / 2).max(0);
                    let dy = if dock_pos == 0 { 8 } else { self.height as i32 - dh - 8 };
                    (dw, dh, dx, dy)
                } else {
                    let dw = v_item + padding * 2;
                    let dh = num_items * (v_item + padding) + padding;
                    let dx = if dock_pos == 2 { 8 } else { self.width as i32 - dw - 8 };
                    let dy = ((self.height as i32 - dh) / 2).max(0);
                    (dw, dh, dx, dy)
                };

                let sm_w = 220;
                let sm_h = 280;
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
                if self.mouse_x >= sm_x && self.mouse_x <= sm_x + sm_w && self.mouse_y >= sm_y && self.mouse_y <= sm_y + sm_h {
                    if self.mouse_y >= sm_y + 44 && self.mouse_y < sm_y + 76 {
                        self.active_submenu = Some(1);
                    } else if self.mouse_y >= sm_y + 76 && self.mouse_y < sm_y + 108 {
                        self.active_submenu = Some(2);
                    } else if self.mouse_y >= sm_y + 108 && self.mouse_y < sm_y + 140 {
                        self.active_submenu = Some(3);
                    } else {
                        self.active_submenu = None;
                    }
                }
            }

            let is_down = mev.buttons & 1 != 0;
            let is_right_down = mev.buttons & 2 != 0;
            
            // Handle context menu selection on click
            if self.context_menu_open && is_down && !self.mouse_left_down {
                let cm_w = 180i32;
                let is_icon = self.context_menu_target.is_some();
                let cm_h = if is_icon { 104i32 } else { 276i32 };
                let cm_x = self.context_menu_x;
                let cm_y = self.context_menu_y;

                let mut clicked_action: Option<&str> = None;
                if self.mouse_x >= cm_x && self.mouse_x <= cm_x + cm_w &&
                   self.mouse_y >= cm_y && self.mouse_y <= cm_y + cm_h {
                    let items: &[(&str, bool)] = if is_icon {
                        &[
                            ("Open", false),
                            ("Open in Terminal", false),
                            ("", true),
                            ("Remove from Desktop", false),
                            ("", true),
                            ("Properties", false),
                        ]
                    } else {
                        &[
                            ("View: Auto-Arrange", false),
                            ("Refresh Desktop", false),
                            ("", true),
                            ("Change Font", false),
                            ("Next Wallpaper", false),
                            ("Display Settings", false),
                            ("", true),
                            ("New Text Document", false),
                            ("New Folder", false),
                            ("", true),
                            ("Open in Terminal", false),
                            ("Task Manager", false),
                            ("System Monitor", false),
                            ("", true),
                            ("Restart System", false),
                            ("Shut Down", false),
                        ]
                    };

                    let mut curr_y = cm_y + 6;
                    for &(label, is_sep) in items {
                        if is_sep {
                            curr_y += 6;
                        } else {
                            if self.mouse_y >= curr_y && self.mouse_y < curr_y + 20 {
                                clicked_action = Some(label);
                                break;
                            }
                            curr_y += 20;
                        }
                    }
                }

                self.context_menu_open = false;
                self.full_redraw = true;

                if let Some(act) = clicked_action {
                    match act {
                        "View: Auto-Arrange" => {
                            let mut row = 0i32;
                            let mut col = 0i32;
                            for icon in &mut self.desktop_icons {
                                icon.x = 24 + col * 76;
                                icon.y = 24 + row * 80;
                                row += 1;
                                if 24 + (row + 1) * 80 > (self.height as i32 - 60) {
                                    row = 0;
                                    col += 1;
                                }
                            }
                        }
                        "Refresh Desktop" => {
                            self.full_redraw = true;
                        }
                        "Change Font" => {
                            crate::gui::fonts::cycle_font();
                            self.full_redraw = true;
                        }
                        "Restart System" => {
                            crate::drivers::power::reboot();
                        }
                        "Shut Down" => {
                            return false;
                        }
                        "New Text Document" => {
                            crate::apps::gui_apps::launch_notepad(self);
                        }
                        "New Folder" => {
                            crate::apps::gui_apps::launch_file_manager(self);
                        }
                        "Open in Terminal" => {
                            crate::apps::gui_apps::launch_terminal(self);
                        }
                        "Task Manager" => {
                            crate::apps::gui_apps::launch_task_manager(self);
                        }
                        "System Monitor" => {
                            crate::apps::gui_apps::launch_sysmon(self);
                        }
                        "Next Wallpaper" => {
                            let wallpapers = [0x111625, 0x181825, 0x0E2038, 0x1E1E2E, 0x2A153A, 0x0D2B1E];
                            let mut wp = crate::gui::settings_app::WALLPAPER_COLOR.lock();
                            let curr = *wp;
                            let mut next_wp = wallpapers[0];
                            for i in 0..wallpapers.len() {
                                if wallpapers[i] == curr {
                                    next_wp = wallpapers[(i + 1) % wallpapers.len()];
                                    break;
                                }
                            }
                            *wp = next_wp;
                        }
                        "Display Settings" => {
                            crate::apps::gui_apps::launch_settings(self);
                        }
                        "Open" => {
                            if let Some(idx) = self.context_menu_target {
                                if idx < self.desktop_icons.len() {
                                    let app_name = self.desktop_icons[idx].name.clone();
                                    match app_name.as_str() {
                                        "Terminal" => crate::apps::gui_apps::launch_terminal(self),
                                        "Settings" => crate::apps::gui_apps::launch_settings(self),
                                        "File Mgr" => crate::apps::gui_apps::launch_file_manager(self),
                                        "Task Mgr" => crate::apps::gui_apps::launch_task_manager(self),
                                        "Sys Mon" => crate::apps::gui_apps::launch_sysmon(self),
                                        "Paint" => crate::apps::gui_apps::launch_paint(self),
                                        "Clock" => crate::apps::gui_apps::launch_clock(self),
                                        "Calendar" => crate::apps::gui_apps::launch_calendar(self),
                                        "Tetris" => crate::apps::gui_apps::launch_tetris(self),
                                        "Pong" => crate::apps::gui_apps::launch_pong(self),
                                        "Mines" => crate::apps::gui_apps::launch_minesweeper(self),
                                        "Browser" => crate::apps::gui_apps::launch_browser(self),
                                        "Notepad" => crate::apps::gui_apps::launch_notepad(self),
                                        "Calculator" => crate::apps::gui_apps::launch_calculator(self),
                                        "Snake" => crate::apps::gui_apps::launch_snake(self),
                                        "2048" => crate::apps::gui_apps::launch_2048(self),
                                        "Sudoku" => crate::apps::gui_apps::launch_sudoku(self),
                                        "Chess" => crate::apps::gui_apps::launch_chess(self),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        "Remove from Desktop" => {
                            if let Some(idx) = self.context_menu_target {
                                if idx < self.desktop_icons.len() {
                                    self.desktop_icons.remove(idx);
                                }
                            }
                        }
                        "Properties" => {
                            crate::apps::gui_apps::launch_settings(self);
                        }
                        _ => {}
                    }
                }

                self.mouse_left_down = is_down;
                self.mouse_right_down = is_right_down;
                continue;
            }
            
            // Handle clicking & dragging
            let just_pressed_left = is_down && !self.mouse_left_down;
            let just_pressed_right = is_right_down && !self.mouse_right_down;
            
            if just_pressed_left || just_pressed_right {
                let btn = if just_pressed_right { 2 } else { 1 };
                let is_left_click = btn == 1;
                // Mouse just pressed
                let mut handled = false;
                
                // Calculate dock bounds
                let dock_pos = *crate::gui::settings_app::DOCK_POSITION.lock();
                let num_items = self.windows.len() as i32 + 1;
                let item_w = 120;
                let item_h = 32;
                let padding = 5;
                let v_item = 44;
                let (dock_w, dock_h, dock_x, dock_y) = if dock_pos <= 1 {
                    let dw = num_items * (item_w + padding) + padding;
                    let dh = item_h + padding * 2 + 4;
                    let dx = ((self.width as i32 - dw) / 2).max(0);
                    let dy = if dock_pos == 0 { 8 } else { self.height as i32 - dh - 8 };
                    (dw, dh, dx, dy)
                } else {
                    let dw = v_item + padding * 2;
                    let dh = num_items * (v_item + padding) + padding;
                    let dx = if dock_pos == 2 { 8 } else { self.width as i32 - dw - 8 };
                    let dy = ((self.height as i32 - dh) / 2).max(0);
                    (dw, dh, dx, dy)
                };

                // Check Start Menu items if open
                if self.start_menu_open {
                    let sm_w = 220;
                    let sm_h = 280;
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
                        let sub_x = sm_x + sm_w + 2;
                        let sub_w = 170;
                        let sub_y = sm_y + 44 + (sub as i32 - 1) * 32;
                        let h = match sub {
                            1 => 6 * 28 + 8,
                            2 => 5 * 28 + 8,
                            3 => 7 * 28 + 8,
                            _ => 28,
                        };
                        
                        if self.mouse_x >= sub_x && self.mouse_x <= sub_x + sub_w && self.mouse_y >= sub_y && self.mouse_y <= sub_y + h {
                            let item_idx = ((self.mouse_y - sub_y - 8) / 28).max(0);
                            if sub == 1 {
                                match item_idx {
                                    0 => crate::apps::gui_apps::launch_terminal(self),
                                    1 => crate::apps::gui_apps::launch_file_manager(self),
                                    2 => crate::apps::gui_apps::launch_settings(self),
                                    3 => crate::apps::gui_apps::launch_task_manager(self),
                                    4 => crate::apps::gui_apps::launch_sysmon(self),
                                    _ => crate::apps::gui_apps::launch_browser(self),
                                }
                            } else if sub == 2 {
                                match item_idx {
                                    0 => crate::apps::gui_apps::launch_notepad(self),
                                    1 => crate::apps::gui_apps::launch_calculator(self),
                                    2 => crate::apps::gui_apps::launch_clock(self),
                                    3 => crate::apps::gui_apps::launch_calendar(self),
                                    _ => crate::apps::gui_apps::launch_paint(self),
                                }
                            } else if sub == 3 {
                                match item_idx {
                                    0 => crate::apps::gui_apps::launch_snake(self),
                                    1 => crate::apps::gui_apps::launch_minesweeper(self),
                                    2 => crate::apps::gui_apps::launch_tetris(self),
                                    3 => crate::apps::gui_apps::launch_pong(self),
                                    4 => crate::apps::gui_apps::launch_2048(self),
                                    5 => crate::apps::gui_apps::launch_sudoku(self),
                                    _ => crate::apps::gui_apps::launch_chess(self),
                                }
                            }
                            self.start_menu_open = false;
                            self.active_submenu = None;
                            handled = true;
                        }
                    }
                    
                    if !handled && self.mouse_x >= sm_x && self.mouse_x <= sm_x + sm_w && self.mouse_y >= sm_y && self.mouse_y <= sm_y + sm_h {
                        // Clicked inside main start menu
                        let btn_y = sm_y + sm_h - 36;
                        let btn_h = 28i32;
                        let btn_w = (sm_w - 24) / 2;
                        let r_x = sm_x + 8;
                        let p_x = sm_x + sm_w - 8 - btn_w;

                        if self.mouse_y >= btn_y && self.mouse_y < btn_y + btn_h {
                            if self.mouse_x >= r_x && self.mouse_x < r_x + btn_w {
                                crate::drivers::power::reboot();
                            } else if self.mouse_x >= p_x && self.mouse_x < p_x + btn_w {
                                return false; // Power off
                            }
                        }
                        // Check if we clicked a category
                        if self.mouse_y >= sm_y + 44 && self.mouse_y < sm_y + 76 {
                            self.active_submenu = Some(1);
                        } else if self.mouse_y >= sm_y + 76 && self.mouse_y < sm_y + 108 {
                            self.active_submenu = Some(2);
                        } else if self.mouse_y >= sm_y + 108 && self.mouse_y < sm_y + 140 {
                            self.active_submenu = Some(3);
                        }
                        handled = true;
                    }
                }

                // Check Dock clicks
                if !handled && self.mouse_x >= dock_x && self.mouse_x <= dock_x + dock_w && self.mouse_y >= dock_y && self.mouse_y <= dock_y + dock_h {
                    let start_w = if dock_pos <= 1 { item_w } else { v_item };
                    let start_h = if dock_pos <= 1 { item_h } else { v_item };
                    let mut curr_x = dock_x + padding;
                    let mut curr_y = dock_y + padding;
                    let dx = if dock_pos <= 1 { item_w + padding } else { 0 };
                    let dy = if dock_pos > 1 { v_item + padding } else { 0 };

                    // Start button click
                    if self.mouse_x >= curr_x && self.mouse_x <= curr_x + start_w && self.mouse_y >= curr_y && self.mouse_y <= curr_y + start_h {
                        self.start_menu_open = !self.start_menu_open;
                        handled = true;
                    } else {
                        curr_x += dx;
                        curr_y += dy;
                        
                        let iw = if dock_pos <= 1 { item_w } else { v_item };
                        let ih = if dock_pos <= 1 { item_h } else { v_item };
                        
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
                        
                        if w.is_point_in_close_btn(self.mouse_x, self.mouse_y) && is_left_click {
                            action = Some((i, 0)); // 0 = close
                            break;
                        } else if w.is_point_in_max_btn(self.mouse_x, self.mouse_y) && is_left_click {
                            action = Some((i, 1)); // 1 = maximize
                            break;
                        } else if w.is_point_in_min_btn(self.mouse_x, self.mouse_y) && is_left_click {
                            action = Some((i, 2)); // 2 = minimize
                            break;
                        } else if let Some(edge) = w.check_resize_zone(self.mouse_x, self.mouse_y) {
                            if is_left_click {
                                self.resized_window = Some(i);
                                self.active_edge = Some(edge);
                            }
                            action = Some((i, 3)); // 3 = bring to front
                            break;
                        } else if w.is_point_in_titlebar(self.mouse_x, self.mouse_y) {
                            if is_left_click {
                                self.dragged_window = Some(i);
                                w.drag_offset_x = self.mouse_x - w.x;
                                w.drag_offset_y = self.mouse_y - w.y;
                            }
                            action = Some((i, 3)); // 3 = bring to front
                            break; // Stop checking lower windows
                        } else if w.is_point_inside(self.mouse_x, self.mouse_y) && w.state != WindowState::Minimized {
                            let rel_x = self.mouse_x - w.x;
                            let rel_y = self.mouse_y - w.y; // w.y is top of client area
                            if rel_y >= 0 && rel_y < w.height && rel_x >= 0 && rel_x < w.width {
                                crate::gui::wm::WINDOW_EVENTS.lock().push((
                                    w.id,
                                    crate::gui::wm::GuiEvent::MouseClick {
                                        x: rel_x,
                                        y: rel_y,
                                        button: btn
                                    }
                                ));
                            }
                            
                            action = Some((i, 3)); // 3 = bring to front
                            break; // Stop checking lower windows
                        }
                    }
                    
                    if let Some((i, act)) = action {
                        if act == 0 {
                            let w = self.windows.remove(i);
                            crate::gui::wm::WINDOW_EVENTS.lock().push((w.id, crate::gui::wm::GuiEvent::WindowClosed));
                        } else if act == 1 {
                            let w = &mut self.windows[i];
                            if w.state == WindowState::Maximized {
                                w.state = WindowState::Normal;
                                w.x = w.restore_x;
                                w.y = w.restore_y;
                                w.width = w.restore_width;
                                w.height = w.restore_height;
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
                            }
                            let event_w = w.width;
                            let event_h = w.height;
                            let event_id = w.id;
                            crate::gui::wm::WINDOW_EVENTS.lock().push((event_id, crate::gui::wm::GuiEvent::Resize { w: event_w, h: event_h }));
                            let win = self.windows.remove(i);
                            self.windows.push(win);
                        } else if act == 2 {
                            self.windows[i].state = WindowState::Minimized;
                        } else if act == 3 {
                            let win = self.windows.remove(i);
                            self.windows.push(win);
                        }
                    } else {
                        // Desktop click
                        let mut clicked_idx = None;
                        for (i, icon) in self.desktop_icons.iter().enumerate() {
                            if self.mouse_x >= icon.x && self.mouse_x <= icon.x + 48 && self.mouse_y >= icon.y && self.mouse_y <= icon.y + 60 {
                                clicked_idx = Some(i);
                                break;
                            }
                        }
                        
                        if btn == 2 {
                            let cm_w = 180i32;
                            let cm_h = if clicked_idx.is_some() { 104i32 } else { 210i32 };
                            let mut cx = self.mouse_x;
                            let mut cy = self.mouse_y;
                            if cx + cm_w > self.width as i32 {
                                cx = (self.width as i32 - cm_w - 4).max(0);
                            }
                            if cy + cm_h > self.height as i32 {
                                cy = (self.height as i32 - cm_h - 4).max(0);
                            }
                            self.context_menu_open = true;
                            self.context_menu_x = cx;
                            self.context_menu_y = cy;
                            self.context_menu_target = clicked_idx;
                            self.full_redraw = true;
                        } else if let Some(idx) = clicked_idx {
                            let current_ticks = crate::process::scheduler::get_ticks();
                            if self.last_clicked_icon == Some(idx) && (current_ticks - self.last_click_ticks) < 100 {
                                // Double click
                                let app_name = self.desktop_icons[idx].name.clone();
                                match app_name.as_str() {
                                    "Terminal" => crate::apps::gui_apps::launch_terminal(self),
                                    "Settings" => crate::apps::gui_apps::launch_settings(self),
                                    "File Mgr" => crate::apps::gui_apps::launch_file_manager(self),
                                    "Task Mgr" => crate::apps::gui_apps::launch_task_manager(self),
                                    "Sys Mon" => crate::apps::gui_apps::launch_sysmon(self),
                                    "Paint" => crate::apps::gui_apps::launch_paint(self),
                                    "Clock" => crate::apps::gui_apps::launch_clock(self),
                                    "Calendar" => crate::apps::gui_apps::launch_calendar(self),
                                    "Tetris" => crate::apps::gui_apps::launch_tetris(self),
                                    "Pong" => crate::apps::gui_apps::launch_pong(self),
                                    "Mines" => crate::apps::gui_apps::launch_minesweeper(self),
                                    "Browser" => crate::apps::gui_apps::launch_browser(self),
                                    "Notepad" => crate::apps::gui_apps::launch_notepad(self),
                                    "Calculator" => crate::apps::gui_apps::launch_calculator(self),
                                    "Snake" => crate::apps::gui_apps::launch_snake(self),
                                    "2048" => crate::apps::gui_apps::launch_2048(self),
                                    "Sudoku" => crate::apps::gui_apps::launch_sudoku(self),
                                    "Chess" => crate::apps::gui_apps::launch_chess(self),
                                    _ => {} // Fallback for things like "Games" folder
                                }
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
                if let Some(idx) = self.resized_window {
                    if idx < self.windows.len() {
                        let win = &self.windows[idx];
                        crate::gui::wm::WINDOW_EVENTS.lock().push((win.id, crate::gui::wm::GuiEvent::Resize { w: win.width, h: win.height }));
                    }
                }
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
                    crate::gui::wm::WINDOW_EVENTS.lock().push((
                        w.id,
                        crate::gui::wm::GuiEvent::KeyPress { key: c }
                    ));
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
                        let mut win = crate::gui::window::Window::new(id, &title, x, y, w, h);
                        win.anim_progress = 60;
                        win.is_animating = true;
                        self.windows.push(win);
                        needs_redraw = true;
                    }
                    crate::gui::wm::GuiMessage::UpdateBuffer { id, buffer_ptr } => {
                        for win in &mut self.windows {
                            if win.id == id {
                                win.buffer_ptr = buffer_ptr;
                                needs_redraw = true;
                                break;
                            }
                        }
                    }
                    crate::gui::wm::GuiMessage::ResizeBuffer { id, w, h, buffer_ptr } => {
                        for win in &mut self.windows {
                            if win.id == id {
                                win.buffer_ptr = buffer_ptr;
                                win.buffer_width = w;
                                win.buffer_height = h;
                                win.buffer_len = (w * h) as usize;
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

        self.anim_ticks = self.anim_ticks.wrapping_add(1);
        for w in &mut self.windows {
            if w.is_animating {
                w.anim_progress = (w.anim_progress + 15).min(100);
                if w.anim_progress >= 100 {
                    w.is_animating = false;
                }
                needs_redraw = true;
            }
        }
        if self.start_menu_open && self.start_menu_anim < 100 {
            self.start_menu_anim = (self.start_menu_anim + 25).min(100);
            needs_redraw = true;
        } else if !self.start_menu_open {
            self.start_menu_anim = 0;
        }

        let current_ticks = crate::process::scheduler::get_ticks();
        
        if needs_redraw {
            for r in &self.last_frame_rects {
                self.dirty_rects.push(*r);
            }
            self.last_frame_rects.clear();
            
            let mut current_rects = alloc::vec::Vec::new();
            
            if self.start_menu_open {
                let dock_pos = *crate::gui::settings_app::DOCK_POSITION.lock();
                let num_items = self.windows.len() as i32 + 1;
                let item_w = 120;
                let padding = 5;
                let v_item_size = 44;
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
                
                let sm_w = 220;
                let sm_h = 280;
                let sm_x = match dock_pos {
                    0 | 1 => dock_x,
                    2 => dock_x + dock_w + 4,
                    _ => dock_x - sm_w - 4,
                };
                let sm_y = match dock_pos {
                    0 => 10 + (32 + padding * 2) + 4,
                    1 => self.height as i32 - (32 + padding * 2) - 10 - sm_h - 4,
                    _ => (self.height as i32 - (num_items * (v_item_size + padding) + padding)) / 2,
                };
                current_rects.push(crate::gui::clip::Rect { x: sm_x, y: sm_y, w: sm_w, h: sm_h });
            }
            
            if self.context_menu_open {
                let cm_h = if self.context_menu_target.is_some() { 104 } else { 276 };
                current_rects.push(crate::gui::clip::Rect { x: self.context_menu_x - 4, y: self.context_menu_y - 4, w: 188, h: cm_h + 8 });
            }
            
            current_rects.push(crate::gui::clip::Rect { x: 0, y: 0, w: self.width as i32, h: 60 });
            current_rects.push(crate::gui::clip::Rect { x: 0, y: self.height as i32 - 60, w: self.width as i32, h: 60 });
            current_rects.push(crate::gui::clip::Rect { x: 0, y: 0, w: 60, h: self.height as i32 });
            current_rects.push(crate::gui::clip::Rect { x: self.width as i32 - 60, y: 0, w: 60, h: self.height as i32 });
            
            for w in &self.windows {
                if w.state != WindowState::Minimized {
                    current_rects.push(crate::gui::clip::Rect { x: w.x - 4, y: w.y - 32, w: w.width + 16, h: w.height + 40 });
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
