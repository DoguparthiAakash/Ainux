use crate::drivers::video;
use crate::drivers::keyboard;
use super::logic::get_default_grid;
use spin::Mutex;

pub static ACTIVE_STATE: Mutex<Option<GameSudoku>> = Mutex::new(None);
pub static SAVED_STATE: Mutex<Option<GameSudoku>> = Mutex::new(None);

#[derive(Clone)]
pub struct GameSudoku {
    grid: [[u8; 9]; 9],
    original_grid: [[u8; 9]; 9],
    cursor_x: usize,
    cursor_y: usize,
}

impl GameSudoku {
    fn new() -> Self {
        let grid = get_default_grid();
        let original_grid = grid.clone();
        Self {
            grid,
            original_grid,
            cursor_x: 0,
            cursor_y: 0,
        }
    }
}

fn draw_line_fb(w: usize, h: usize, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32, top_margin: usize) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut e2;
    loop {
        if x0 >= 0 && x0 < w as i32 && y0 >= 0 && y0 < h as i32 {
            crate::drivers::video::draw_pixel(x0 as i64, (y0 + top_margin as i32) as i64, color);
        }
        if x0 == x1 && y0 == y1 { break; }
        e2 = 2 * err;
        if e2 >= dy { err += dy; x0 += sx; }
        if e2 <= dx { err += dx; y0 += sy; }
    }
}

fn draw_digit_fb(w: usize, h: usize, x: i32, y: i32, digit: u8, scale: i32, color: u32, top_margin: usize) {
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
                video::fill_rect((x + (col_idx * scale)) as i64, (y + top_margin as i32 + (row_idx as i32 * scale)) as i64, scale as i64, scale as i64, color);
            }
        }
    }
}

pub fn run() {
    video::clear();
    let (w, h) = video::get_resolution();
    
    let mut game = ACTIVE_STATE.lock().take().unwrap_or_else(|| GameSudoku::new());

    let top_margin = 80;
    let render_h = if h > top_margin { h - top_margin } else { h };

    let available_dim = (render_h as i32 - 40).min(w as i32 - 40).max(81);
    let cell_size = available_dim / 9;
    let board_size_2d = cell_size * 9;
    let start_x = (w as i32 / 2) - (board_size_2d / 2);
    let start_y = (render_h as i32 / 2) - (board_size_2d / 2);

    let mut first_draw = true;

    loop {
        if first_draw {
            video::fill_rect(0, 0, w as i64, h as i64, 0xFF0D0D1A);
            video::put_str_at(2, 1, "SUDOKU - AINUX GAMES (2D MODE)", 0xFFFFFFFF, 0xFF0D0D1A);
            video::put_str_at(2, 2, "Use ARROW KEYS/WASD to move, 1-9 to enter numbers, 0 to clear, Q to quit.", 0xFFAAAAAA, 0xFF0D0D1A);
            first_draw = false;
        }
        
        for y in 0..9 {
            for x in 0..9 {
                let px = start_x + (x as i32 * cell_size);
                let py = start_y + (y as i32 * cell_size);
                
                let bg_color = if x == game.cursor_x && y == game.cursor_y {
                    0xFF333333
                } else if ((x / 3) + (y / 3)) % 2 == 0 {
                    0xFF111111
                } else {
                    0xFF1A1A1A
                };

                video::fill_rect(px as i64, (py + top_margin as i32) as i64, cell_size as i64, cell_size as i64, bg_color);
                
                // Borders
                draw_line_fb(w, render_h, px, py, px + cell_size, py, 0xFF444444, top_margin);
                draw_line_fb(w, render_h, px, py, px, py + cell_size, 0xFF444444, top_margin);

                let val = game.grid[y][x];
                if val != 0 {
                    let text_color = if game.original_grid[y][x] != 0 { 0xFFFFFFFF } else { 0xFF00FFCC };
                    let scale = (cell_size / 15).max(1);
                    draw_digit_fb(w, render_h, px + (cell_size / 2) - (5 * scale / 2), py + (cell_size / 2) - (5 * scale / 2), val, scale, text_color, top_margin);
                }
            }
        }
        
        // Thicker 3x3 borders
        for i in 0..=3 {
            let px = start_x + (i * 3) as i32 * cell_size;
            let py = start_y + (i * 3) as i32 * cell_size;
            video::fill_rect(px as i64, (start_y + top_margin as i32) as i64, 2, board_size_2d as i64, 0xFF888888);
            video::fill_rect(start_x as i64, (py + top_margin as i32) as i64, board_size_2d as i64, 2, 0xFF888888);
        }

        // Handle input (Blocking so we don't spin rendering if not needed, improving performance)
        let c = keyboard::get_char();
        match c {
            'q' | 'Q' | '\x1B' | '\x03' => break,
            'w' | 'W' | keyboard::KEY_UP => if game.cursor_y > 0 { game.cursor_y -= 1 },
            's' | 'S' | keyboard::KEY_DOWN => if game.cursor_y < 8 { game.cursor_y += 1 },
            'a' | 'A' | keyboard::KEY_LEFT => if game.cursor_x > 0 { game.cursor_x -= 1 },
            'd' | 'D' | keyboard::KEY_RIGHT => if game.cursor_x < 8 { game.cursor_x += 1 },
            '1'..='9' => {
                if game.original_grid[game.cursor_y][game.cursor_x] == 0 {
                    game.grid[game.cursor_y][game.cursor_x] = c.to_digit(10).unwrap() as u8;
                }
            },
            '0' | ' ' | '\x08' => { // Backspace or 0 or space to clear
                if game.original_grid[game.cursor_y][game.cursor_x] == 0 {
                    game.grid[game.cursor_y][game.cursor_x] = 0;
                }
            }
            _ => {}
        }
    }
    
    *ACTIVE_STATE.lock() = Some(game);
    video::clear();
}
