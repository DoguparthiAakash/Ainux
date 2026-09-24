use crate::gui::app::App;
use crate::drivers::keyboard;
use crate::games::sudoku::logic::get_default_grid;

fn draw_digit_fb(buf: &mut [u32], buf_w: usize, buf_h: usize, x: i32, y: i32, digit: u8, scale: i32, color: u32) {
    if digit > 9 { return; }
    let font: [[u8; 5]; 10] = [
        [0b01110, 0b10001, 0b10001, 0b10001, 0b01110], // 0
        [0b00100, 0b01100, 0b00100, 0b00100, 0b01110], // 1
        [0b01110, 0b10001, 0b00110, 0b01000, 0b11111], // 2
        [0b01110, 0b10001, 0b00110, 0b10001, 0b01110], // 3
        [0b10010, 0b10010, 0b11111, 0b00010, 0b00010], // 4
        [0b11111, 0b10000, 0b11110, 0b00001, 0b11110], // 5
        [0b01110, 0b10000, 0b11110, 0b10001, 0b01110], // 6
        [0b11111, 0b00001, 0b00010, 0b00100, 0b00100], // 7
        [0b01110, 0b10001, 0b01110, 0b10001, 0b01110], // 8
        [0b01110, 0b10001, 0b01111, 0b00001, 0b01110], // 9
    ];
    let bitmap = font[digit as usize];
    for (row_idx, row_val) in bitmap.iter().enumerate() {
        for col_idx in 0..5 {
            if (row_val & (1 << (4 - col_idx))) != 0 {
                SudokuApp::fill(buf, buf_w, buf_h, x + (col_idx * scale), y + (row_idx as i32 * scale), scale, scale, color);
            }
        }
    }
}

pub struct SudokuApp {
    grid: [[u8; 9]; 9],
    original_grid: [[u8; 9]; 9],
    cursor_x: usize,
    cursor_y: usize,
    needs_redraw: bool,
}

impl SudokuApp {
    pub fn new() -> Self {
        let grid = get_default_grid();
        let original_grid = grid.clone();
        Self {
            grid,
            original_grid,
            cursor_x: 0,
            cursor_y: 0,
            needs_redraw: true,
        }
    }

    fn fill(buf: &mut [u32], bw: usize, bh: usize, x: i32, y: i32, w: i32, h: i32, c: u32) {
        for dy in 0..h {
            let sy = y + dy; if sy < 0 || sy >= bh as i32 { continue; }
            for dx in 0..w {
                let sx = x + dx; if sx < 0 || sx >= bw as i32 { continue; }
                buf[(sy * bw as i32 + sx) as usize] = c;
            }
        }
    }
}

impl App for SudokuApp {
    fn update(&mut self) {}

    fn draw(&mut self, buf: &mut [u32], w: usize, h: usize) {
        if !self.needs_redraw { return; }
        
        Self::fill(buf, w, h, 0, 0, w as i32, h as i32, 0xFF0D0D1A);
        
        crate::drivers::video::draw_text_to_buffer(buf, w as i64, h as i64, 4, 4, "SUDOKU", 0xFFFFFFFF);
        
        let available_dim = (h as i32 - 30).min(w as i32 - 10).max(81);
        let cell_size = available_dim / 9;
        let board_size_2d = cell_size * 9;
        let start_x = (w as i32 / 2) - (board_size_2d / 2);
        let start_y = (h as i32 / 2) - (board_size_2d / 2) + 10;
        
        for y in 0..9 {
            for x in 0..9 {
                let px = start_x + (x as i32 * cell_size);
                let py = start_y + (y as i32 * cell_size);
                
                let bg_color = if x == self.cursor_x && y == self.cursor_y {
                    0xFF333333
                } else if ((x / 3) + (y / 3)) % 2 == 0 {
                    0xFF111111
                } else {
                    0xFF1A1A1A
                };

                Self::fill(buf, w, h, px, py, cell_size, cell_size, bg_color);
                
                // Borders
                Self::fill(buf, w, h, px, py, cell_size, 1, 0xFF444444);
                Self::fill(buf, w, h, px, py, 1, cell_size, 0xFF444444);

                let val = self.grid[y][x];
                if val != 0 {
                    let text_color = if self.original_grid[y][x] != 0 { 0xFFFFFFFF } else { 0xFF00FFCC };
                    let scale = (cell_size / 15).max(1);
                    draw_digit_fb(buf, w, h, px + (cell_size / 2) - (5 * scale / 2), py + (cell_size / 2) - (5 * scale / 2), val, scale, text_color);
                }
            }
        }
        
        for i in 0..=3 {
            let px = start_x + (i * 3) as i32 * cell_size;
            let py = start_y + (i * 3) as i32 * cell_size;
            Self::fill(buf, w, h, px, start_y, 2, board_size_2d, 0xFF888888);
            Self::fill(buf, w, h, start_x, py, board_size_2d, 2, 0xFF888888);
        }
        
        self.needs_redraw = false;
    }

    fn on_mouse_event(&mut self, _x: i32, _y: i32, _buttons: u8) {}

    fn on_key_event(&mut self, c: char) {
        match c {
            'w' | 'W' | keyboard::KEY_UP => { if self.cursor_y > 0 { self.cursor_y -= 1; self.needs_redraw = true; } },
            's' | 'S' | keyboard::KEY_DOWN => { if self.cursor_y < 8 { self.cursor_y += 1; self.needs_redraw = true; } },
            'a' | 'A' | keyboard::KEY_LEFT => { if self.cursor_x > 0 { self.cursor_x -= 1; self.needs_redraw = true; } },
            'd' | 'D' | keyboard::KEY_RIGHT => { if self.cursor_x < 8 { self.cursor_x += 1; self.needs_redraw = true; } },
            '1'..='9' => {
                if self.original_grid[self.cursor_y][self.cursor_x] == 0 {
                    if let Some(digit) = c.to_digit(10) {
                        self.grid[self.cursor_y][self.cursor_x] = digit as u8;
                        self.needs_redraw = true;
                    }
                }
            },
            '0' | ' ' | '\x08' => {
                if self.original_grid[self.cursor_y][self.cursor_x] == 0 {
                    self.grid[self.cursor_y][self.cursor_x] = 0;
                    self.needs_redraw = true;
                }
            }
            _ => {}
        }
    }
}
