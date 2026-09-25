import re
import sys

def main():
    path = "/mnt/e/lh/lsr/Ainux/custom_kernel/apps/gui/compositor.rs"
    with open(path, "r") as f:
        content = f.read()

    # 1. Modify fill_rect_buffer definition
    orig_fill_rect = """fn fill_rect_buffer(buf: &mut [u32], buf_w: usize, buf_h: usize, rx: i32, ry: i32, rw: i32, rh: i32, color: u32) {
    for dy in 0..rh {
        let sy = ry + dy;
        if sy < 0 || sy >= buf_h as i32 { continue; }
        for dx in 0..rw {
            let sx = rx + dx;
            if sx < 0 || sx >= buf_w as i32 { continue; }
            buf[(sy * buf_w as i32 + sx) as usize] = color;
        }
    }
}"""
    
    new_fill_rect = """fn fill_rect_buffer(buf: &mut [u32], buf_w: usize, buf_h: usize, rx: i32, ry: i32, rw: i32, rh: i32, color: u32, clip: Option<&crate::gui::clip::Rect>) {
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
}"""
    content = content.replace(orig_fill_rect, new_fill_rect)

    # 2. Modify draw_string_to_buffer definition
    orig_draw_str = """fn draw_string_to_buffer(buf: &mut [u32], buf_w: usize, buf_h: usize, x: i32, y: i32, s: &str, fg: u32) {
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
}"""

    new_draw_str = """fn draw_string_to_buffer(buf: &mut [u32], buf_w: usize, buf_h: usize, x: i32, y: i32, s: &str, fg: u32, clip: Option<&crate::gui::clip::Rect>) {
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
}"""
    content = content.replace(orig_draw_str, new_draw_str)

    # 3. Add fields to Compositor struct
    orig_struct = """pub struct Compositor {
    pub backbuffer: Vec<u32>,
    pub width: usize,
    pub height: usize,
    pub windows: Vec<Window>,"""
    
    new_struct = """pub struct Compositor {
    pub dirty_rects: Vec<crate::gui::clip::Rect>,
    pub full_redraw: bool,
    pub backbuffer: Vec<u32>,
    pub width: usize,
    pub height: usize,
    pub windows: Vec<Window>,"""
    content = content.replace(orig_struct, new_struct)
    
    orig_new = """        Self {
            backbuffer: alloc::vec![0; w * h],"""
            
    new_new = """        Self {
            dirty_rects: alloc::vec::Vec::new(),
            full_redraw: true,
            backbuffer: alloc::vec![0; w * h],"""
    content = content.replace(orig_new, new_new)

    # 4. Modify draw() to compute clip_rect
    orig_draw_start = """    pub fn draw(&mut self) {
        // 1. Clear background (Desktop Wallpaper)
        let bg_color = *crate::gui::settings_app::WALLPAPER_COLOR.lock();
        // Fill entire screen
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 0, 0, self.width as i32, self.height as i32, bg_color | 0xFF000000);"""
        
    new_draw_start = """    pub fn invalidate(&mut self, rect: crate::gui::clip::Rect) {
        self.dirty_rects.push(rect);
    }
    
    pub fn invalidate_all(&mut self) {
        self.full_redraw = true;
    }

    pub fn draw(&mut self) {
        let clip_rect = if self.full_redraw {
            self.full_redraw = false;
            self.dirty_rects.clear();
            Some(crate::gui::clip::Rect { x: 0, y: 0, w: self.width as i32, h: self.height as i32 })
        } else if self.dirty_rects.is_empty() {
            video::invalidate_cursor_backup();
            video::draw_mouse_cursor(self.mouse_x, self.mouse_y);
            return;
        } else {
            let mut min_x = self.width as i32;
            let mut min_y = self.height as i32;
            let mut max_x = 0;
            let mut max_y = 0;
            for r in &self.dirty_rects {
                min_x = min_x.min(r.x);
                min_y = min_y.min(r.y);
                max_x = max_x.max(r.x + r.w);
                max_y = max_y.max(r.y + r.h);
            }
            self.dirty_rects.clear();
            min_x = min_x.max(0);
            min_y = min_y.max(0);
            max_x = max_x.min(self.width as i32);
            max_y = max_y.min(self.height as i32);
            if min_x >= max_x || min_y >= max_y {
                video::invalidate_cursor_backup();
                video::draw_mouse_cursor(self.mouse_x, self.mouse_y);
                return;
            }
            Some(crate::gui::clip::Rect { x: min_x, y: min_y, w: max_x - min_x, h: max_y - min_y })
        };
        let clip_opt = clip_rect.as_ref();

        // 1. Clear background (Desktop Wallpaper)
        let bg_color = *crate::gui::settings_app::WALLPAPER_COLOR.lock();
        // Fill entire screen
        fill_rect_buffer(&mut self.backbuffer, self.width, self.height, 0, 0, self.width as i32, self.height as i32, bg_color | 0xFF000000, clip_opt);"""
        
    content = content.replace(orig_draw_start, new_draw_start)
    
    # 5. Fix all calls to fill_rect_buffer and draw_string_to_buffer inside draw()
    # Let's just do a regex replace to add `, clip_opt` to calls that don't have it yet.
    # We must restrict this to draw() only.
    # A simple way: just replace `fill_rect_buffer(..., color)` with `fill_rect_buffer(..., color, clip_opt)`
    content = re.sub(r'fill_rect_buffer\(([^;]+),\s*([^;]+)\);', r'fill_rect_buffer(\1, \2, clip_opt);', content)
    content = re.sub(r'draw_string_to_buffer\(([^;]+),\s*([^;]+)\);', r'draw_string_to_buffer(\1, \2, clip_opt);', content)
    
    # We need to fix window blitting clip!
    # orig:
    #             // Draw window content (blit its buffer)
    #             for dy in 0..w.height {
    #                 let sy = w.y + dy;
    #                 if sy < 0 || sy >= self.height as i32 { continue; }
    #                 
    #                 for dx in 0..w.width {
    #                     let sx = w.x + dx;
    #                     if sx < 0 || sx >= self.width as i32 { continue; }
    #                     
    #                     let src_idx = (dy * w.width + dx) as usize;
    #                     let dst_idx = (sy * self.width as i32 + sx) as usize;
    #                     self.backbuffer[dst_idx] = w.buffer[src_idx];
    #                 }
    #             }
    orig_blit = """            // Draw window content (blit its buffer)
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
            }"""
            
    new_blit = """            // Draw window content (blit its buffer)
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
            }"""
    content = content.replace(orig_blit, new_blit)
    
    # 6. Desktop Icons BMP draw clipping
    orig_icon_bmp = """                    let color = icon.bmp.data[src_y * icon.bmp.width + src_x];
                    if color & 0xFF000000 != 0 && color != 0xFFFFFFFF { // extremely basic transparency for white bg bmp
                        self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                    }"""
    new_icon_bmp = """                    let color = icon.bmp.data[src_y * icon.bmp.width + src_x];
                    if color & 0xFF000000 != 0 && color != 0xFFFFFFFF {
                        if let Some(c) = clip_opt {
                            if sx >= c.x && sx < c.x + c.w && sy >= c.y && sy < c.y + c.h {
                                self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                            }
                        } else {
                            self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                        }
                    }"""
    content = content.replace(orig_icon_bmp, new_icon_bmp, 1)

    # 7. Button BMP draw clipping
    orig_btn_bmp = """                        if color & 0xFF000000 != 0 && color != 0xFFFFFFFF { 
                            if sy >= 0 && sy < self.height as i32 && sx >= 0 && sx < self.width as i32 {
                                self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                            }
                        }"""
    new_btn_bmp = """                        if color & 0xFF000000 != 0 && color != 0xFFFFFFFF { 
                            if sy >= 0 && sy < self.height as i32 && sx >= 0 && sx < self.width as i32 {
                                if let Some(c) = clip_opt {
                                    if sx >= c.x && sx < c.x + c.w && sy >= c.y && sy < c.y + c.h {
                                        self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                                    }
                                } else {
                                    self.backbuffer[(sy * self.width as i32 + sx) as usize] = color;
                                }
                            }
                        }"""
    content = content.replace(orig_btn_bmp, new_btn_bmp, 1)
    
    # 8. Dock Icon BMP draw clipping
    orig_dock_bmp = """                        let c = icon.bmp.data[src_y * icon.bmp.width + src_x];
                        if c & 0xFF000000 != 0 && c != 0xFFFFFFFF { 
                            self.backbuffer[(sy * self.width as i32 + sx) as usize] = c;
                        }"""
    new_dock_bmp = """                        let c = icon.bmp.data[src_y * icon.bmp.width + src_x];
                        if c & 0xFF000000 != 0 && c != 0xFFFFFFFF { 
                            if let Some(cl) = clip_opt {
                                if sx >= cl.x && sx < cl.x + cl.w && sy >= cl.y && sy < cl.y + cl.h {
                                    self.backbuffer[(sy * self.width as i32 + sx) as usize] = c;
                                }
                            } else {
                                self.backbuffer[(sy * self.width as i32 + sx) as usize] = c;
                            }
                        }"""
    content = content.replace(orig_dock_bmp, new_dock_bmp, 1)
    
    # 9. Blit region to framebuffer (only the clipped region)
    orig_blit_fb = """        // 4. Blit backbuffer to framebuffer
        // Use blit_buffer_opaque to correctly handle hardware pitch (stride)
        video::blit_buffer_opaque(&self.backbuffer, 0, 0, self.width as i32, self.height as i32, self.width as i32);"""
    
    new_blit_fb = """        // 4. Blit backbuffer to framebuffer
        if let Some(c) = clip_opt {
            let offset = c.y as usize * self.width + c.x as usize;
            let src_slice = &self.backbuffer[offset..];
            video::blit_buffer_opaque(src_slice, c.x, c.y, c.w, c.h, self.width as i32);
        } else {
            video::blit_buffer_opaque(&self.backbuffer, 0, 0, self.width as i32, self.height as i32, self.width as i32);
        }"""
    content = content.replace(orig_blit_fb, new_blit_fb)

    with open(path, "w") as f:
        f.write(content)

if __name__ == "__main__":
    main()
