use alloc::vec::Vec;
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

pub fn snake_main() {
    let id = 17;
    let width = 400;
    let height = 300;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("Snake"),
        x: 100,
        y: 100,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFF000000; width * height];
    let mut app = SnakeApp::new();
    let mut ticks = 0;
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    match c {
                        'w' | 'W' | crate::drivers::keyboard::KEY_UP => if app.direction != Direction::Down { app.direction = Direction::Up },
                        's' | 'S' | crate::drivers::keyboard::KEY_DOWN => if app.direction != Direction::Up { app.direction = Direction::Down },
                        'a' | 'A' | crate::drivers::keyboard::KEY_LEFT => if app.direction != Direction::Right { app.direction = Direction::Left },
                        'd' | 'D' | crate::drivers::keyboard::KEY_RIGHT => if app.direction != Direction::Left { app.direction = Direction::Right },
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        
        ticks += 1;
        let mut needs_redraw = false;
        
        if ticks % 20 == 0 {
            app.step();
            needs_redraw = true;
        }
        
        if needs_redraw {
            for i in 0..buffer.len() {
                buffer[i] = 0xFF000000;
            }
            
            let cell_size = 20;
            for &(x, y) in &app.snake {
                let px = x * cell_size as i32;
                let py = y * cell_size as i32;
                fill_rect_buffer(&mut buffer, width, height, px, py, cell_size as i32, cell_size as i32, 0xFF00FF00); // Green
            }
            
            let fx = app.food.0 * cell_size as i32;
            let fy = app.food.1 * cell_size as i32;
            fill_rect_buffer(&mut buffer, width, height, fx, fy, cell_size as i32, cell_size as i32, 0xFFFF0000); // Red
            
            if app.game_over {
                video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 10, 10, "GAME OVER", 0xFFFFFFFF);
            }
            
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::yield_now();
    }
}
