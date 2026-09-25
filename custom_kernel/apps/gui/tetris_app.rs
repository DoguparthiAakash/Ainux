
use crate::drivers::keyboard;

const PIECES: [[[u8; 4]; 4]; 7] = [
    // I (Cyan: 0xFF00FFFF)
    [ [0, 0, 0, 0], [1, 1, 1, 1], [0, 0, 0, 0], [0, 0, 0, 0] ],
    // J (Blue: 0xFF0000FF)
    [ [2, 0, 0, 0], [2, 2, 2, 0], [0, 0, 0, 0], [0, 0, 0, 0] ],
    // L (Orange: 0xFFFFA500)
    [ [0, 0, 3, 0], [3, 3, 3, 0], [0, 0, 0, 0], [0, 0, 0, 0] ],
    // O (Yellow: 0xFFFFFF00)
    [ [0, 4, 4, 0], [0, 4, 4, 0], [0, 0, 0, 0], [0, 0, 0, 0] ],
    // S (Green: 0xFF00FF00)
    [ [0, 5, 5, 0], [5, 5, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0] ],
    // T (Purple: 0xFF800080)
    [ [0, 6, 0, 0], [6, 6, 6, 0], [0, 0, 0, 0], [0, 0, 0, 0] ],
    // Z (Red: 0xFFFF0000)
    [ [7, 7, 0, 0], [0, 7, 7, 0], [0, 0, 0, 0], [0, 0, 0, 0] ]
];

const COLORS: [u32; 8] = [
    0xFF111111, // 0: Empty
    0xFF00FFFF, // 1: I (Cyan)
    0xFF0000FF, // 2: J (Blue)
    0xFFFFA500, // 3: L (Orange)
    0xFFFFFF00, // 4: O (Yellow)
    0xFF00FF00, // 5: S (Green)
    0xFFFF00FF, // 6: T (Purple)
    0xFFFF0000, // 7: Z (Red)
];

pub struct TetrisApp {
    grid: [[u8; 10]; 20],
    curr_piece: [[u8; 4]; 4],
    curr_x: i32,
    curr_y: i32,
    score: u32,
    game_over: bool,
    rng_state: u64,
    last_fall_tick: u64,
    needs_redraw: bool,
}

impl TetrisApp {
    pub fn new() -> Self {
        let mut app = Self {
            grid: [[0; 10]; 20],
            curr_piece: [[0; 4]; 4],
            curr_x: 0,
            curr_y: 0,
            score: 0,
            game_over: false,
            rng_state: crate::cpu::cpuid::rdtsc(),
            last_fall_tick: crate::process::scheduler::get_ticks(),
            needs_redraw: true,
        };
        app.spawn_piece();
        app
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

pub fn tetris_main() {
    let id = 12;
    let width = 280;
    let height = 400;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("Tetris"),
        x: 100,
        y: 100,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFF111111; width * height];
    let mut app = TetrisApp::new();
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    if app.game_over { continue; }
                    match c {
                        'a' | 'A' | keyboard::KEY_LEFT => { app.move_dx(-1); app.needs_redraw = true; },
                        'd' | 'D' | keyboard::KEY_RIGHT => { app.move_dx(1); app.needs_redraw = true; },
                        'w' | 'W' | keyboard::KEY_UP => { app.rotate(); app.needs_redraw = true; },
                        's' | 'S' | keyboard::KEY_DOWN => { 
                            app.fall(); 
                            app.last_fall_tick = crate::process::scheduler::get_ticks(); 
                            app.needs_redraw = true; 
                        },
                        ' ' => { 
                            app.hard_drop(); 
                            app.last_fall_tick = crate::process::scheduler::get_ticks(); 
                            app.needs_redraw = true; 
                        },
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        
        if !app.game_over {
            let current_tick = crate::process::scheduler::get_ticks();
            let fall_speed: u64 = 50; 
            if current_tick > app.last_fall_tick + fall_speed {
                app.fall();
                app.last_fall_tick = current_tick;
                app.needs_redraw = true;
            }
        }
        
        if app.needs_redraw {
            TetrisApp::fill(&mut buffer, width, height, 0, 0, width as i32, height as i32, 0xFF111111);
            
            crate::drivers::video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 4, 4, "TETRIS", 0xFF00FF00);
            let score_str = alloc::format!("Score: {}", app.score);
            crate::drivers::video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 4, 20, &score_str, 0xFFFFFFFF);
            
            if app.game_over {
                crate::drivers::video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 4, 40, "GAME OVER", 0xFFFF0000);
            }
            
            // Calculate cell size
            let available_h = height as i32 - 10;
            let available_w = width as i32 - 80;
            let spacing = 1;
            let max_cell_h = (available_h - (21 * spacing)) / 20;
            let max_cell_w = (available_w - (11 * spacing)) / 10;
            let cell_size = max_cell_h.min(max_cell_w).max(5);
            
            let board_w = 10 * cell_size + 11 * spacing;
            let board_h = 20 * cell_size + 21 * spacing;
            let start_x = 75; // Right of the score
            let start_y = (height as i32 - board_h) / 2;
            
            TetrisApp::fill(&mut buffer, width, height, start_x, start_y, board_w, board_h, 0xFF333333);
            
            for gy in 0..20 {
                for gx in 0..10 {
                    let val = app.grid[gy as usize][gx as usize];
                    let px = start_x + spacing + (gx as i32 * (cell_size + spacing));
                    let py = start_y + spacing + (gy as i32 * (cell_size + spacing));
                    TetrisApp::fill(&mut buffer, width, height, px, py, cell_size, cell_size, COLORS[val as usize]);
                }
            }
            
            for r in 0..4 {
                for c in 0..4 {
                    let val = app.curr_piece[r][c];
                    if val != 0 {
                        let gx = app.curr_x + c as i32;
                        let gy = app.curr_y + r as i32;
                        if gy >= 0 {
                            let px = start_x + spacing + (gx * (cell_size + spacing));
                            let py = start_y + spacing + (gy * (cell_size + spacing));
                            TetrisApp::fill(&mut buffer, width, height, px, py, cell_size, cell_size, COLORS[val as usize]);
                        }
                    }
                }
            }
            
            app.needs_redraw = false;
            
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::yield_now();
    }
}
