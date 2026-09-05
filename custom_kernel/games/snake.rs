use alloc::vec::Vec;
use crate::drivers::video;
use crate::drivers::keyboard;
use spin::Mutex;

pub static ACTIVE_STATE: Mutex<Option<GameSnake>> = Mutex::new(None);
pub static SAVED_STATE: Mutex<Option<GameSnake>> = Mutex::new(None);

#[derive(Clone, PartialEq, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone)]
pub struct GameSnake {
    snake: Vec<(i32, i32)>,
    food: (i32, i32),
    direction: Direction,
    score: i32,
    game_over: bool,
    grid_w: i32,
    grid_h: i32,
    rng_state: u32,
}

impl GameSnake {
    fn new(grid_w: i32, grid_h: i32) -> Self {
        let mut game = Self {
            snake: alloc::vec![(grid_w / 2, grid_h / 2)],
            food: (0, 0),
            direction: Direction::Right,
            score: 0,
            game_over: false,
            grid_w,
            grid_h,
            rng_state: 12345, // basic LCG
        };
        game.spawn_food();
        game
    }

    fn next_rand(&mut self) -> u32 {
        self.rng_state = self.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.rng_state
    }

    fn spawn_food(&mut self) {
        loop {
            let rx = (self.next_rand() % self.grid_w as u32) as i32;
            let ry = (self.next_rand() % self.grid_h as u32) as i32;
            if !self.snake.contains(&(rx, ry)) {
                self.food = (rx, ry);
                break;
            }
        }
    }

    fn step(&mut self) {
        if self.game_over { return; }
        
        let head = self.snake[0];
        let next_pos = match self.direction {
            Direction::Up => (head.0, head.1 - 1),
            Direction::Down => (head.0, head.1 + 1),
            Direction::Left => (head.0 - 1, head.1),
            Direction::Right => (head.0 + 1, head.1),
        };

        if next_pos.0 < 0 || next_pos.0 >= self.grid_w || next_pos.1 < 0 || next_pos.1 >= self.grid_h || self.snake.contains(&next_pos) {
            self.game_over = true;
            return;
        }

        self.snake.insert(0, next_pos);

        if next_pos == self.food {
            self.score += 10;
            self.spawn_food();
        } else {
            self.snake.pop();
        }
    }
}

pub fn run() {
    video::clear();
    let (w, h) = video::get_resolution();
    
    let top_margin = 80;
    let render_h = if h > top_margin { h - top_margin } else { h };
    
    let cell_size = 20;
    let grid_w = (w as i32 - 40) / cell_size;
    let grid_h = (render_h as i32 - 40) / cell_size;
    
    let start_x = (w as i32 / 2) - (grid_w * cell_size / 2);
    let start_y = (render_h as i32 / 2) - (grid_h * cell_size / 2) + top_margin as i32;
    
    let mut game = ACTIVE_STATE.lock().take().unwrap_or_else(|| GameSnake::new(grid_w, grid_h));
    
    let mut last_tick = crate::process::scheduler::get_ticks();
    let mut needs_redraw = true;
    
    loop {
        if needs_redraw {
            // Draw
            video::fill_rect(0, 0, w as i64, h as i64, 0xFF0D0D1A);
            video::put_str_at(2, 1, "SNAKE - AINUX GAMES (2D MODE)", 0xFFFFFFFF, 0xFF0D0D1A);
            video::put_str_at(2, 2, "Use ARROW KEYS/WASD to move. Q to quit. R to restart.", 0xFFAAAAAA, 0xFF0D0D1A);
            
            let score_str = alloc::format!("SCORE: {}", game.score);
            video::put_str_at(2, 4, &score_str, 0xFF00FF00, 0xFF0D0D1A);
            
            // Borders
            video::fill_rect((start_x - 2) as i64, (start_y - 2) as i64, (grid_w * cell_size + 4) as i64, (grid_h * cell_size + 4) as i64, 0xFF444444);
            video::fill_rect(start_x as i64, start_y as i64, (grid_w * cell_size) as i64, (grid_h * cell_size) as i64, 0xFF111111);
            
            // Food
            video::fill_rect((start_x + game.food.0 * cell_size) as i64, (start_y + game.food.1 * cell_size) as i64, cell_size as i64, cell_size as i64, 0xFFFF0000);
            
            // Snake
            for (i, p) in game.snake.iter().enumerate() {
                let color = if i == 0 { 0xFF00FF00 } else { 0xFF00AA00 };
                video::fill_rect((start_x + p.0 * cell_size) as i64, (start_y + p.1 * cell_size) as i64, cell_size as i64, cell_size as i64, color);
            }

            if game.game_over {
                video::put_str_at((w/2 - 40) as usize, (h/2) as usize, "GAME OVER", 0xFFFF0000, 0xFF111111);
            }
            
            needs_redraw = false;
        }
        
        // Input
        let current = crate::process::scheduler::get_ticks();
        let delay = if current > last_tick + 15 {
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
                'r' | 'R' => { game = GameSnake::new(grid_w, grid_h); needs_redraw = true; },
                'w' | 'W' | keyboard::KEY_UP => { if game.direction != Direction::Down { game.direction = Direction::Up; needs_redraw = true; } },
                's' | 'S' | keyboard::KEY_DOWN => { if game.direction != Direction::Up { game.direction = Direction::Down; needs_redraw = true; } },
                'a' | 'A' | keyboard::KEY_LEFT => { if game.direction != Direction::Right { game.direction = Direction::Left; needs_redraw = true; } },
                'd' | 'D' | keyboard::KEY_RIGHT => { if game.direction != Direction::Left { game.direction = Direction::Right; needs_redraw = true; } },
                _ => {}
            }
        }
    }
    
    *ACTIVE_STATE.lock() = Some(game);
    video::clear();
}
