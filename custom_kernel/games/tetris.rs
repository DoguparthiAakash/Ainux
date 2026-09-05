use alloc::format;
use crate::drivers::video;
use crate::drivers::keyboard;
use alloc::vec::Vec;
use spin::Mutex;

pub static ACTIVE_STATE: Mutex<Option<GameTetris>> = Mutex::new(None);
pub static SAVED_STATE: Mutex<Option<GameTetris>> = Mutex::new(None);

fn draw_digit_fb(x: i32, y: i32, digit: u8, scale: i32, color: u32) {
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
                video::fill_rect((x + (col_idx * scale)) as i64, (y + (row_idx as i32 * scale)) as i64, scale as i64, scale as i64, color);
            }
        }
    }
}

fn draw_number_fb(mut x: i32, y: i32, mut num: u32, scale: i32, color: u32) {
    if num == 0 {
        draw_digit_fb(x, y, 0, scale, color);
        return;
    }
    
    let mut digits = Vec::new();
    while num > 0 {
        digits.push((num % 10) as u8);
        num /= 10;
    }
    
    for digit in digits.iter().rev() {
        draw_digit_fb(x, y, *digit, scale, color);
        x += 6 * scale; // 5 wide + 1 gap
    }
}

// 7 Standard Tetrominoes (I, J, L, O, S, T, Z)
// Each represented as a 4x4 grid in a 1D array.
const PIECES: [[[u8; 4]; 4]; 7] = [
    // I (Cyan: 0xFF00FFFF)
    [
        [0, 0, 0, 0],
        [1, 1, 1, 1],
        [0, 0, 0, 0],
        [0, 0, 0, 0]
    ],
    // J (Blue: 0xFF0000FF)
    [
        [2, 0, 0, 0],
        [2, 2, 2, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0]
    ],
    // L (Orange: 0xFFFFA500)
    [
        [0, 0, 3, 0],
        [3, 3, 3, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0]
    ],
    // O (Yellow: 0xFFFFFF00)
    [
        [0, 4, 4, 0],
        [0, 4, 4, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0]
    ],
    // S (Green: 0xFF00FF00)
    [
        [0, 5, 5, 0],
        [5, 5, 0, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0]
    ],
    // T (Purple: 0xFF800080)
    [
        [0, 6, 0, 0],
        [6, 6, 6, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0]
    ],
    // Z (Red: 0xFFFF0000)
    [
        [7, 7, 0, 0],
        [0, 7, 7, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0]
    ]
];

const COLORS: [u32; 8] = [
    0xFF111111, // 0: Empty (Dark gray background)
    0xFF00FFFF, // 1: I (Cyan)
    0xFF0000FF, // 2: J (Blue)
    0xFFFFA500, // 3: L (Orange)
    0xFFFFFF00, // 4: O (Yellow)
    0xFF00FF00, // 5: S (Green)
    0xFFFF00FF, // 6: T (Purple)
    0xFFFF0000, // 7: Z (Red)
];

#[derive(Clone)]
pub struct GameTetris {
    grid: [[u8; 10]; 20],
    curr_piece: [[u8; 4]; 4],
    curr_x: i32,
    curr_y: i32,
    score: u32,
    game_over: bool,
    rng_state: u64,
}

impl GameTetris {
    fn new() -> Self {
        let mut game = Self {
            grid: [[0; 10]; 20],
            score: 0,
            game_over: false,
            rng_state: crate::cpu::cpuid::rdtsc(),
            curr_piece: [[0; 4]; 4],
            curr_x: 0,
            curr_y: 0,
        };
        game.spawn_piece();
        game
    }

    fn rand(&mut self) -> usize {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state as usize
    }

    fn spawn_piece(&mut self) {
        let p_idx = self.rand() % 7;
        self.curr_piece = PIECES[p_idx];
        self.curr_x = 3;
        self.curr_y = 0;
        
        if !self.is_valid(self.curr_x, self.curr_y, &self.curr_piece) {
            self.game_over = true;
        }
    }

    fn is_valid(&self, nx: i32, ny: i32, piece: &[[u8; 4]; 4]) -> bool {
        for r in 0..4 {
            for c in 0..4 {
                if piece[r][c] != 0 {
                    let gx = nx + c as i32;
                    let gy = ny + r as i32;
                    if gx < 0 || gx >= 10 || gy >= 20 {
                        return false;
                    }
                    if gy >= 0 && self.grid[gy as usize][gx as usize] != 0 {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn rotate(&mut self) {
        let mut new_piece = [[0; 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                new_piece[c][3 - r] = self.curr_piece[r][c];
            }
        }
        if self.is_valid(self.curr_x, self.curr_y, &new_piece) {
            self.curr_piece = new_piece;
        }
    }

    fn move_dx(&mut self, dx: i32) {
        if self.is_valid(self.curr_x + dx, self.curr_y, &self.curr_piece) {
            self.curr_x += dx;
        }
    }

    fn fall(&mut self) -> bool {
        if self.is_valid(self.curr_x, self.curr_y + 1, &self.curr_piece) {
            self.curr_y += 1;
            true
        } else {
            self.lock_piece();
            false
        }
    }
    
    fn hard_drop(&mut self) {
        while self.is_valid(self.curr_x, self.curr_y + 1, &self.curr_piece) {
            self.curr_y += 1;
        }
        self.lock_piece();
    }

    fn lock_piece(&mut self) {
        for r in 0..4 {
            for c in 0..4 {
                if self.curr_piece[r][c] != 0 {
                    let gy = self.curr_y + r as i32;
                    let gx = self.curr_x + c as i32;
                    if gy >= 0 && gy < 20 && gx >= 0 && gx < 10 {
                        self.grid[gy as usize][gx as usize] = self.curr_piece[r][c];
                    }
                }
            }
        }
        self.clear_lines();
        self.spawn_piece();
    }

    fn clear_lines(&mut self) {
        let mut lines_cleared = 0;
        let mut y = 19;
        while y >= 0 {
            let mut full = true;
            for x in 0..10 {
                if self.grid[y as usize][x] == 0 {
                    full = false;
                    break;
                }
            }
            if full {
                lines_cleared += 1;
                for move_y in (1..=y).rev() {
                    for x in 0..10 {
                        self.grid[move_y as usize][x] = self.grid[(move_y - 1) as usize][x];
                    }
                }
                for x in 0..10 {
                    self.grid[0][x] = 0;
                }
                // Check this line again since blocks fell down
                y += 1; 
            }
            y -= 1;
        }
        
        match lines_cleared {
            1 => self.score += 100,
            2 => self.score += 300,
            3 => self.score += 500,
            4 => self.score += 800,
            _ => {}
        }
    }
}

pub fn run() {
    video::clear();
    let (w, h) = video::get_resolution();
    
    let mut game = ACTIVE_STATE.lock().take().unwrap_or_else(|| GameTetris::new());

    let top_margin = 80;
    let render_h = if h > top_margin { h - top_margin } else { h };

    // Draw header once
    video::fill_rect(0, 0, w as i64, top_margin as i64, 0xFF111111);
    video::put_str_at(2, 1, "TETRIS - AINUX GAMES", 0xFF00FF00, 0xFF111111);
    video::put_str_at(2, 2, "Use ARROW KEYS/WASD to move, W/UP to rotate. SPACE to hard drop. Q to quit.", 0xFF888888, 0xFF111111);

    let available_h = render_h as i32 - 40;
    let available_w = w as i32 - 40;
    let spacing = (available_h / 400).max(1); // minimal spacing
    let max_cell_h = (available_h - (21 * spacing)) / 20;
    let max_cell_w = (available_w - (11 * spacing)) / 10;
    let cell_size = max_cell_h.min(max_cell_w).max(5);
    
    let board_w = 10 * cell_size + 11 * spacing;
    let board_h = 20 * cell_size + 21 * spacing;
    let start_x = (w as i32 / 2) - (board_w / 2);
    let start_y = (render_h as i32 / 2) - (board_h / 2) + top_margin as i32;
    
    let mut last_fall_tick = crate::process::scheduler::get_ticks();
    // Default speed: 50 ticks = 500ms per drop
    let fall_speed = 50; 
    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            // bg
            video::fill_rect(0, top_margin as i64, w as i64, render_h as i64, 0xFF111111);
            
            video::fill_rect(start_x as i64, start_y as i64, board_w as i64, board_h as i64, 0xFF333333);

            // Draw grid
            for y in 0..20 {
                for x in 0..10 {
                    let val = game.grid[y as usize][x as usize];
                    let px = start_x + spacing + (x as i32 * (cell_size + spacing));
                    let py = start_y + spacing + (y as i32 * (cell_size + spacing));
                    video::fill_rect(px as i64, py as i64, cell_size as i64, cell_size as i64, COLORS[val as usize]);
                }
            }
            
            // Draw current piece
            for r in 0..4 {
                for c in 0..4 {
                    let val = game.curr_piece[r][c];
                    if val != 0 {
                        let gx = game.curr_x + c as i32;
                        let gy = game.curr_y + r as i32;
                        if gy >= 0 {
                            let px = start_x + spacing + (gx * (cell_size + spacing));
                            let py = start_y + spacing + (gy * (cell_size + spacing));
                            video::fill_rect(px as i64, py as i64, cell_size as i64, cell_size as i64, COLORS[val as usize]);
                        }
                    }
                }
            }

            // Draw score
            draw_number_fb(20, top_margin as i32 + 20, game.score, 4, 0xFFFFFFFF);

            if game.game_over {
                video::put_str_at((w/8/2) - 4, (h/16/2) + 2, " GAME OVER ", 0xFFFFFFFF, 0xFFFF0000);
            }
            needs_redraw = false;
        }
        
        if game.game_over {
            let c = keyboard::get_char(); // block
            if c == 'q' || c == 'Q' || c == '\x1B' {
                break;
            }
            continue; // wait for quit
        }

        // Non-blocking input handling
        if let Some(c) = keyboard::pop_char() {
            match c {
                'q' | 'Q' | '\x1B' => break,
                'a' | 'A' | keyboard::KEY_LEFT => { game.move_dx(-1); needs_redraw = true; },
                'd' | 'D' | keyboard::KEY_RIGHT => { game.move_dx(1); needs_redraw = true; },
                'w' | 'W' | keyboard::KEY_UP => { game.rotate(); needs_redraw = true; },
                's' | 'S' | keyboard::KEY_DOWN => { game.fall(); last_fall_tick = crate::process::scheduler::get_ticks(); needs_redraw = true; },
                ' ' => { game.hard_drop(); last_fall_tick = crate::process::scheduler::get_ticks(); needs_redraw = true; },
                _ => {}
            }
        }
        
        let current_tick = crate::process::scheduler::get_ticks();
        if current_tick - last_fall_tick > fall_speed {
            game.fall();
            last_fall_tick = current_tick;
            needs_redraw = true;
        }
        
        // Yield to prevent pegging the CPU
        crate::process::scheduler::yield_now();
    }
    
    *ACTIVE_STATE.lock() = Some(game);
    video::clear();
}
