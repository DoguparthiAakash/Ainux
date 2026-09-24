use alloc::vec::Vec;
use crate::gui::app::App;
use crate::drivers::video;

#[derive(Clone, PartialEq, Copy)]
enum Direction { Up, Down, Left, Right }

fn fill_rect_buffer(buf: &mut [u32], buf_w: usize, buf_h: usize, rx: i32, ry: i32, rw: i32, rh: i32, color: u32) {
    for dy in 0..rh {
        let sy = ry + dy;
        if sy < 0 || sy >= buf_h as i32 { continue; }
        for dx in 0..rw {
            let sx = rx + dx;
            if sx < 0 || sx >= buf_w as i32 { continue; }
            buf[(sy * buf_w as i32 + sx) as usize] = color;
        }
    }
}

pub struct SnakeApp {
    snake: Vec<(i32, i32)>,
    food: (i32, i32),
    direction: Direction,
    score: i32,
    game_over: bool,
    grid_w: i32,
    grid_h: i32,
    ticks: u32,
}

impl SnakeApp {
    pub fn new() -> Self {
        Self {
            snake: alloc::vec![(10, 10)],
            food: (15, 15),
            direction: Direction::Right,
            score: 0,
            game_over: false,
            grid_w: 20,
            grid_h: 15,
            ticks: 0,
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
            self.food.0 = (self.food.0 * 7 + 13) % self.grid_w;
            self.food.1 = (self.food.1 * 11 + 17) % self.grid_h;
            while self.snake.contains(&self.food) {
                self.food.0 = (self.food.0 + 1) % self.grid_w;
            }
        } else {
            self.snake.pop();
        }
    }
}

impl App for SnakeApp {
    fn update(&mut self) {
        self.ticks += 1;
        if self.ticks % 20 == 0 {
            self.step();
        }
    }

    fn draw(&mut self, buffer: &mut [u32], width: usize, height: usize) {
        for i in 0..buffer.len() {
            buffer[i] = 0xFF000000; // Black background
        }
        
        let cell_size = 20;
        for &(x, y) in &self.snake {
            let px = x * cell_size as i32;
            let py = y * cell_size as i32;
            fill_rect_buffer(buffer, width, height, px, py, cell_size as i32, cell_size as i32, 0xFF00FF00); // Green
        }
        
        let fx = self.food.0 * cell_size as i32;
        let fy = self.food.1 * cell_size as i32;
        fill_rect_buffer(buffer, width, height, fx, fy, cell_size as i32, cell_size as i32, 0xFFFF0000); // Red
        
        if self.game_over {
            video::draw_text_to_buffer(buffer, width as i64, height as i64, 10, 10, "GAME OVER", 0xFFFFFFFF);
        }
    }

    fn on_mouse_event(&mut self, _x: i32, _y: i32, _buttons: u8) {}

    fn on_key_event(&mut self, c: char) {
        match c {
            'w' | 'W' => if self.direction != Direction::Down { self.direction = Direction::Up },
            's' | 'S' => if self.direction != Direction::Up { self.direction = Direction::Down },
            'a' | 'A' => if self.direction != Direction::Right { self.direction = Direction::Left },
            'd' | 'D' => if self.direction != Direction::Left { self.direction = Direction::Right },
            _ => {}
        }
    }
}
