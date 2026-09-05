use crate::drivers::video;
use crate::drivers::keyboard;
use spin::Mutex;

pub static ACTIVE_STATE: Mutex<Option<GamePong>> = Mutex::new(None);
pub static SAVED_STATE: Mutex<Option<GamePong>> = Mutex::new(None);

#[derive(Clone)]
pub struct GamePong {
    p1_y: i32,
    p2_y: i32,
    ball_x: i32,
    ball_y: i32,
    ball_vx: i32,
    ball_vy: i32,
    p1_score: i32,
    p2_score: i32,
    grid_w: i32,
    grid_h: i32,
}

impl GamePong {
    fn new(grid_w: i32, grid_h: i32) -> Self {
        Self {
            p1_y: grid_h / 2 - 2,
            p2_y: grid_h / 2 - 2,
            ball_x: grid_w / 2,
            ball_y: grid_h / 2,
            ball_vx: -1,
            ball_vy: 1,
            p1_score: 0,
            p2_score: 0,
            grid_w,
            grid_h,
        }
    }

    fn step(&mut self) {
        self.ball_x += self.ball_vx;
        self.ball_y += self.ball_vy;

        // Top and bottom collision
        if self.ball_y <= 0 || self.ball_y >= self.grid_h - 1 {
            self.ball_vy = -self.ball_vy;
        }

        let paddle_h = 5;

        // Player 1 collision
        if self.ball_x == 1 && self.ball_y >= self.p1_y && self.ball_y <= self.p1_y + paddle_h {
            self.ball_vx = -self.ball_vx;
        }

        // Player 2 collision (AI or simple logic)
        if self.ball_x == self.grid_w - 2 && self.ball_y >= self.p2_y && self.ball_y <= self.p2_y + paddle_h {
            self.ball_vx = -self.ball_vx;
        }

        // AI movement for Player 2
        if self.ball_y > self.p2_y + paddle_h / 2 && self.p2_y + paddle_h < self.grid_h {
            self.p2_y += 1;
        } else if self.ball_y < self.p2_y + paddle_h / 2 && self.p2_y > 0 {
            self.p2_y -= 1;
        }

        // Scoring
        if self.ball_x < 0 {
            self.p2_score += 1;
            self.reset_ball();
        } else if self.ball_x >= self.grid_w {
            self.p1_score += 1;
            self.reset_ball();
        }
    }

    fn reset_ball(&mut self) {
        self.ball_x = self.grid_w / 2;
        self.ball_y = self.grid_h / 2;
        self.ball_vx = if self.p1_score > self.p2_score { 1 } else { -1 };
        self.ball_vy = 1;
    }
}

pub fn run() {
    video::clear();
    let (w, h) = video::get_resolution();
    
    let top_margin = 80;
    let render_h = if h > top_margin { h - top_margin } else { h };
    
    let cell_size = 15;
    let grid_w = (w as i32 - 40) / cell_size;
    let grid_h = (render_h as i32 - 40) / cell_size;
    
    let start_x = (w as i32 / 2) - (grid_w * cell_size / 2);
    let start_y = (render_h as i32 / 2) - (grid_h * cell_size / 2) + top_margin as i32;
    
    let mut game = ACTIVE_STATE.lock().take().unwrap_or_else(|| GamePong::new(grid_w, grid_h));
    
    let mut last_tick = crate::process::scheduler::get_ticks();
    let mut needs_redraw = true;
    
    loop {
        if needs_redraw {
            // Draw
            video::fill_rect(0, 0, w as i64, h as i64, 0xFF0D0D1A);
            video::put_str_at(2, 1, "PONG - AINUX GAMES (2D MODE)", 0xFFFFFFFF, 0xFF0D0D1A);
            video::put_str_at(2, 2, "Use W/S to move up and down. Q to quit.", 0xFFAAAAAA, 0xFF0D0D1A);
            
            let score_str = alloc::format!("P1: {}   P2: {}", game.p1_score, game.p2_score);
            video::put_str_at((w/2 - 60) as usize, 60, &score_str, 0xFF00FF00, 0xFF0D0D1A);
            
            // Borders
            video::fill_rect((start_x - 2) as i64, (start_y - 2) as i64, (grid_w * cell_size + 4) as i64, (grid_h * cell_size + 4) as i64, 0xFF444444);
            video::fill_rect(start_x as i64, start_y as i64, (grid_w * cell_size) as i64, (grid_h * cell_size) as i64, 0xFF111111);
            
            // Player 1
            video::fill_rect((start_x + 1 * cell_size) as i64, (start_y + game.p1_y * cell_size) as i64, cell_size as i64, (5 * cell_size) as i64, 0xFF00AAFF);
            
            // Player 2
            video::fill_rect((start_x + (grid_w - 2) * cell_size) as i64, (start_y + game.p2_y * cell_size) as i64, cell_size as i64, (5 * cell_size) as i64, 0xFFFF0000);
            
            // Ball
            video::fill_rect((start_x + game.ball_x * cell_size) as i64, (start_y + game.ball_y * cell_size) as i64, cell_size as i64, cell_size as i64, 0xFFFFFFFF);

            needs_redraw = false;
        }

        // Input and Step
        let current = crate::process::scheduler::get_ticks();
        let delay = if current > last_tick + 10 {
            last_tick = current;
            true
        } else { false };

        if delay {
            game.step();
            needs_redraw = true;
        }

        if let Some(c) = keyboard::pop_char() {
            match c {
                'q' | 'Q' | '\x1B' => break,
                'w' | 'W' | keyboard::KEY_UP => { if game.p1_y > 0 { game.p1_y -= 2; needs_redraw = true; } },
                's' | 'S' | keyboard::KEY_DOWN => { if game.p1_y + 5 < grid_h { game.p1_y += 2; needs_redraw = true; } },
                _ => {}
            }
        }
    }
    
    *ACTIVE_STATE.lock() = Some(game);
    video::clear();
}
