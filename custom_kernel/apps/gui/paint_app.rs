use alloc::vec::Vec;
use crate::gui::app::App;
use crate::drivers::video;
use crate::drivers::mouse;

const COLORS: [u32; 8] = [
    0xFF000000, // Black
    0xFFFFFFFF, // White
    0xFFFF0000, // Red
    0xFF00FF00, // Green
    0xFF0000FF, // Blue
    0xFFFFFF00, // Yellow
    0xFFFF00FF, // Magenta
    0xFF00FFFF, // Cyan
];

pub struct PaintApp {
    canvas: Vec<u32>,
    canvas_w: usize,
    canvas_h: usize,
    selected_color: usize,
    brush_size: usize,
    is_drawing: bool,
    last_x: i32,
    last_y: i32,
}

impl PaintApp {
    pub fn new() -> Self {
        let cw = 400;
        let ch = 300;
        let mut canvas = alloc::vec![0xFFFFFFFF; cw * ch]; // White background
        Self {
            canvas,
            canvas_w: cw,
            canvas_h: ch,
            selected_color: 0,
            brush_size: 3,
            is_drawing: false,
            last_x: -1,
            last_y: -1,
        }
    }

    fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut curr_x = x0;
        let mut curr_y = y0;

        loop {
            // Draw a brush stroke around current point
            let bs = self.brush_size as i32;
            for by in -bs..=bs {
                for bx in -bs..=bs {
                    if bx*bx + by*by <= bs*bs {
                        let px = curr_x + bx;
                        let py = curr_y + by;
                        if px >= 0 && px < self.canvas_w as i32 && py >= 0 && py < self.canvas_h as i32 {
                            self.canvas[(py * self.canvas_w as i32 + px) as usize] = COLORS[self.selected_color];
                        }
                    }
                }
            }

            if curr_x == x1 && curr_y == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                curr_x += sx;
            }
            if e2 <= dx {
                err += dx;
                curr_y += sy;
            }
        }
    }
}

impl App for PaintApp {
    fn update(&mut self) {}

    fn draw(&mut self, buffer: &mut [u32], width: usize, height: usize) {
        // Draw Toolbar background
        for y in 0..40 {
            for x in 0..width {
                buffer[y * width + x] = 0xFFCCCCCC; // light gray toolbar
            }
        }

        // Draw color palette
        for i in 0..COLORS.len() {
            let cx = 10 + i * 30;
            let cy = 10;
            for y in cy..(cy + 20) {
                for x in cx..(cx + 20) {
                    buffer[y * width + x] = COLORS[i];
                }
            }
            // Highlight selected
            if i == self.selected_color {
                for x in (cx - 2)..(cx + 22) {
                    buffer[(cy - 2) * width + x] = 0xFFFF0000;
                    buffer[(cy + 21) * width + x] = 0xFFFF0000;
                }
                for y in (cy - 2)..(cy + 22) {
                    buffer[y * width + cx - 2] = 0xFFFF0000;
                    buffer[y * width + cx + 21] = 0xFFFF0000;
                }
            }
        }

        video::draw_text_to_buffer(buffer, width as i64, height as i64, 260, 15, "Paint", 0xFF000000);

        // Draw Canvas
        let cx_offset = 10;
        let cy_offset = 50;
        
        for y in 0..height {
            if y < cy_offset { continue; }
            for x in 0..width {
                if x >= cx_offset && x < cx_offset + self.canvas_w && y < cy_offset + self.canvas_h {
                    let cy = y - cy_offset;
                    let cx = x - cx_offset;
                    buffer[y * width + x] = self.canvas[cy * self.canvas_w + cx];
                } else {
                    buffer[y * width + x] = 0xFF333333; // dark background for non-canvas area
                }
            }
        }
    }

    fn on_mouse_event(&mut self, x: i32, y: i32, buttons: u8) {
        let left_down = (buttons & 1) != 0;

        if left_down {
            // Check toolbar interaction
            if y >= 10 && y <= 30 {
                for i in 0..COLORS.len() {
                    let cx = 10 + i as i32 * 30;
                    if x >= cx && x < cx + 20 {
                        self.selected_color = i;
                        return;
                    }
                }
            }

            // Canvas interaction
            let width = 640; // Default logical window width usually, but let's approximate based on offset
            let cx_offset = (width - self.canvas_w as i32) / 2;
            // Since we don't know window width reliably in on_mouse_event, let's assume standard offset for now.
            // Wait, we can compute it if we keep track of width. For now, assuming centered in 640x480.
            let cx_offset = 10; // We'll just hardcode the canvas to left-align in mouse coords for simplicity if width is unknown. 
            // Better yet, let's draw canvas at x=10, y=50 always in draw() as well to match perfectly.
            
            let cy_offset = 50;
            let cx = x - cx_offset;
            let cy = y - cy_offset;

            if cx >= 0 && cx < self.canvas_w as i32 && cy >= 0 && cy < self.canvas_h as i32 {
                if !self.is_drawing {
                    self.is_drawing = true;
                    self.last_x = cx;
                    self.last_y = cy;
                }
                self.draw_line(self.last_x, self.last_y, cx, cy);
                self.last_x = cx;
                self.last_y = cy;
            } else {
                self.is_drawing = false;
            }
        } else {
            self.is_drawing = false;
        }
    }

    fn on_key_event(&mut self, _c: char) {}
}
