use alloc::format;
use crate::drivers::video;
use crate::drivers::keyboard;
use alloc::string::String;
use alloc::vec::Vec;

fn fill_rect_buf(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, width: i32, height: i32, color: u32) {
    let start_x = x.max(0);
    let start_y = y.max(0);
    let end_x = (x + width).min(w as i32);
    let end_y = (y + height).min(h as i32);
    
    for row in start_y..end_y {
        let row_offset = (row as usize) * w;
        for col in start_x..end_x {
            buf[row_offset + (col as usize)] = color;
        }
    }
}

fn draw_digit_buf(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, digit: u8, scale: i32, color: u32) {
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
                fill_rect_buf(buf, w, h, x + (col_idx * scale), y + (row_idx as i32 * scale), scale, scale, color);
            }
        }
    }
}

fn draw_number_buf(buf: &mut [u32], w: usize, h: usize, mut x: i32, y: i32, mut num: u32, scale: i32, color: u32) {
    if num == 0 {
        draw_digit_buf(buf, w, h, x, y, 0, scale, color);
        return;
    }
    
    let mut digits = Vec::new();
    while num > 0 {
        digits.push((num % 10) as u8);
        num /= 10;
    }
    
    for digit in digits.iter().rev() {
        draw_digit_buf(buf, w, h, x, y, *digit, scale, color);
        x += 6 * scale; // 5 wide + 1 gap
    }
}

fn get_color_for_value(value: u32) -> (u32, u32) { // (bg, fg)
    match value {
        0 => (0xFF3C3A32, 0x00000000), // empty cell color
        2 => (0xFFEEE4DA, 0xFF776E65),
        4 => (0xFFEDE0C8, 0xFF776E65),
        8 => (0xFFF2B179, 0xFFF9F6F2),
        16 => (0xFFF59563, 0xFFF9F6F2),
        32 => (0xFFF67C5F, 0xFFF9F6F2),
        64 => (0xFFF65E3B, 0xFFF9F6F2),
        128 => (0xFFEDCF72, 0xFFF9F6F2),
        256 => (0xFFEDCC61, 0xFFF9F6F2),
        512 => (0xFFEDC850, 0xFFF9F6F2),
        1024 => (0xFFEDC53F, 0xFFF9F6F2),
        2048 => (0xFFEDC22E, 0xFFF9F6F2),
        _ => (0xFF3C3A32, 0xFFF9F6F2), // super high values
    }
}

struct Game2048 {
    grid: [[u32; 4]; 4],
    score: u32,
    game_over: bool,
    rng_state: u64,
}

impl Game2048 {
    fn new() -> Self {
        let mut game = Self {
            grid: [[0; 4]; 4],
            score: 0,
            game_over: false,
            rng_state: crate::cpu::cpuid::rdtsc(),
        };
        game.spawn_tile();
        game.spawn_tile();
        game
    }

    fn rand(&mut self) -> u32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state as u32
    }

    fn spawn_tile(&mut self) {
        let mut empty = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                if self.grid[y][x] == 0 {
                    empty.push((x, y));
                }
            }
        }
        if empty.is_empty() { return; }
        
        let idx = (self.rand() as usize) % empty.len();
        let (x, y) = empty[idx];
        let val = if (self.rand() % 10) < 9 { 2 } else { 4 };
        self.grid[y][x] = val;
    }

    fn move_left(&mut self) -> bool {
        let mut moved = false;
        for y in 0..4 {
            let mut row = [0; 4];
            let mut idx = 0;
            for x in 0..4 {
                if self.grid[y][x] != 0 {
                    row[idx] = self.grid[y][x];
                    idx += 1;
                }
            }
            let mut merged = [0; 4];
            let mut m_idx = 0;
            let mut skip = false;
            for i in 0..4 {
                if skip { skip = false; continue; }
                if i < 3 && row[i] != 0 && row[i] == row[i+1] {
                    merged[m_idx] = row[i] * 2;
                    self.score += merged[m_idx];
                    skip = true;
                    m_idx += 1;
                } else if row[i] != 0 {
                    merged[m_idx] = row[i];
                    m_idx += 1;
                }
            }
            for x in 0..4 {
                if self.grid[y][x] != merged[x] {
                    moved = true;
                    self.grid[y][x] = merged[x];
                }
            }
        }
        moved
    }

    fn move_right(&mut self) -> bool {
        // Reverse row, move_left, reverse back
        self.reverse_horizontal();
        let moved = self.move_left();
        self.reverse_horizontal();
        moved
    }

    fn move_up(&mut self) -> bool {
        self.transpose();
        let moved = self.move_left();
        self.transpose();
        moved
    }

    fn move_down(&mut self) -> bool {
        self.transpose();
        self.reverse_horizontal();
        let moved = self.move_left();
        self.reverse_horizontal();
        self.transpose();
        moved
    }

    fn transpose(&mut self) {
        let mut new_grid = [[0; 4]; 4];
        for y in 0..4 {
            for x in 0..4 {
                new_grid[x][y] = self.grid[y][x];
            }
        }
        self.grid = new_grid;
    }

    fn reverse_horizontal(&mut self) {
        for y in 0..4 {
            self.grid[y].reverse();
        }
    }

    fn check_game_over(&mut self) {
        // Any empty?
        for y in 0..4 {
            for x in 0..4 {
                if self.grid[y][x] == 0 { return; }
            }
        }
        // Any horizontal merges?
        for y in 0..4 {
            for x in 0..3 {
                if self.grid[y][x] == self.grid[y][x+1] { return; }
            }
        }
        // Any vertical merges?
        for y in 0..3 {
            for x in 0..4 {
                if self.grid[y][x] == self.grid[y+1][x] { return; }
            }
        }
        self.game_over = true;
    }
}

pub fn run() {
    video::clear();
    let (w, h) = video::get_resolution();
    
    let mut game = Game2048::new();

    let top_margin = 80;
    let render_h = if h > top_margin { h - top_margin } else { h };
    let mut backbuffer = alloc::vec![0xFFFAF8EF; w * render_h];

    // Draw header once
    video::fill_rect(0, 0, w as i64, top_margin as i64, 0xFFFAF8EF);
    video::put_str_at(2, 1, "2048 - AINUX GAMES", 0xFF776E65, 0xFFFAF8EF);
    video::put_str_at(2, 2, "Use ARROW KEYS/WASD to merge numbers. Q to quit.", 0xFF776E65, 0xFFFAF8EF);

    let cell_size = 100;
    let spacing = 15;
    let board_size = 4 * cell_size + 5 * spacing;
    let start_x = (w as i32 / 2) - (board_size / 2);
    let start_y = (render_h as i32 / 2) - (board_size / 2);

    loop {
        backbuffer.fill(0xFFFAF8EF); // Page bg
        
        // Draw score
        let score_str = format!("SCORE: {}", game.score);
        // We'll just draw the score string by custom bitmap, or just use the shell put_str
        
        // Board Background
        fill_rect_buf(&mut backbuffer, w, render_h, start_x, start_y, board_size, board_size, 0xFFBBADA0);

        for y in 0..4 {
            for x in 0..4 {
                let val = game.grid[y][x];
                let (bg, fg) = get_color_for_value(val);
                
                let px = start_x + spacing + (x as i32 * (cell_size + spacing));
                let py = start_y + spacing + (y as i32 * (cell_size + spacing));
                
                fill_rect_buf(&mut backbuffer, w, render_h, px, py, cell_size, cell_size, bg);
                
                if val > 0 {
                    let mut num_len = 0;
                    let mut temp = val;
                    while temp > 0 { num_len += 1; temp /= 10; }
                    
                    let scale = if num_len >= 4 { 3 } else { 4 };
                    let text_w = num_len as i32 * 6 * scale;
                    let text_x = px + (cell_size / 2) - (text_w / 2);
                    let text_y = py + (cell_size / 2) - (5 * scale / 2);
                    
                    draw_number_buf(&mut backbuffer, w, render_h, text_x, text_y, val, scale, fg);
                }
            }
        }

        if game.game_over {
            // Draw overlay
            fill_rect_buf(&mut backbuffer, w, render_h, start_x, start_y, board_size, board_size, 0x88EDC22E);
            // Draw "Game Over" but we don't have font easily, so just let them see they can't move
        }

        video::copy_buffer_region(&backbuffer, top_margin * w);

        let c = keyboard::get_char();
        
        let mut moved = false;
        match c {
            'q' | 'Q' => break,
            'w' | 'W' | keyboard::KEY_UP => moved = game.move_up(),
            's' | 'S' | keyboard::KEY_DOWN => moved = game.move_down(),
            'a' | 'A' | keyboard::KEY_LEFT => moved = game.move_left(),
            'd' | 'D' | keyboard::KEY_RIGHT => moved = game.move_right(),
            _ => {}
        }
        
        if moved {
            game.spawn_tile();
            game.check_game_over();
        }
    }
    video::clear();
}
