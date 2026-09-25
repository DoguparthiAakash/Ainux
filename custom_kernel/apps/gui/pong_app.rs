
use crate::drivers::keyboard;

pub struct PongApp {
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
    last_tick: u64,
    needs_redraw: bool,
}

impl PongApp {
    pub fn new() -> Self {
        let grid_w = 24;
        let grid_h = 17;
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
            last_tick: crate::process::scheduler::get_ticks(),
            needs_redraw: true,
        }
    }

    fn reset_ball(&mut self) {
        self.ball_x = self.grid_w / 2;
        self.ball_y = self.grid_h / 2;
        self.ball_vx = if self.p1_score > self.p2_score { 1 } else { -1 };
        self.ball_vy = 1;
    }

    fn step(&mut self) {
        self.ball_x += self.ball_vx;
        self.ball_y += self.ball_vy;

        if self.ball_y <= 0 || self.ball_y >= self.grid_h - 1 {
            self.ball_vy = -self.ball_vy;
        }

        let paddle_h = 5;

        if self.ball_x == 1 && self.ball_y >= self.p1_y && self.ball_y <= self.p1_y + paddle_h {
            self.ball_vx = -self.ball_vx;
        }

        if self.ball_x == self.grid_w - 2 && self.ball_y >= self.p2_y && self.ball_y <= self.p2_y + paddle_h {
            self.ball_vx = -self.ball_vx;
        }

        if self.ball_y > self.p2_y + paddle_h / 2 && self.p2_y + paddle_h < self.grid_h {
            self.p2_y += 1;
        } else if self.ball_y < self.p2_y + paddle_h / 2 && self.p2_y > 0 {
            self.p2_y -= 1;
        }

        if self.ball_x < 0 {
            self.p2_score += 1;
            self.reset_ball();
        } else if self.ball_x >= self.grid_w {
            self.p1_score += 1;
            self.reset_ball();
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

pub fn pong_main() {
    let id = 13;
    let width = 400;
    let height = 320;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("Pong"),
        x: 150,
        y: 150,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFF0D0D1A; width * height];
    let mut app = PongApp::new();
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    match c {
                        'w' | 'W' | keyboard::KEY_UP => { if app.p1_y > 0 { app.p1_y -= 2; app.needs_redraw = true; } },
                        's' | 'S' | keyboard::KEY_DOWN => { if app.p1_y + 5 < app.grid_h { app.p1_y += 2; app.needs_redraw = true; } },
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        
        let current = crate::process::scheduler::get_ticks();
        if current > app.last_tick + 10 {
            app.last_tick = current;
            app.step();
            app.needs_redraw = true;
        }
        
        if app.needs_redraw {
            PongApp::fill(&mut buffer, width, height, 0, 0, width as i32, height as i32, 0xFF0D0D1A);
            
            crate::drivers::video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 2, 2, "PONG", 0xFFFFFFFF);
            crate::drivers::video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 2, 18, "W/S: Move", 0xFFAAAAAA);
            
            let score_str = alloc::format!("P1: {}   P2: {}", app.p1_score, app.p2_score);
            crate::drivers::video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, (width/2 - 40) as i64, 2, &score_str, 0xFF00FF00);
            
            let cell_size = 15;
            let start_x = 20;
            let start_y = 40;
            
            // Borders
            PongApp::fill(&mut buffer, width, height, start_x - 2, start_y - 2, app.grid_w * cell_size + 4, app.grid_h * cell_size + 4, 0xFF444444);
            PongApp::fill(&mut buffer, width, height, start_x, start_y, app.grid_w * cell_size, app.grid_h * cell_size, 0xFF111111);
            
            // Player 1
            PongApp::fill(&mut buffer, width, height, start_x + 1 * cell_size, start_y + app.p1_y * cell_size, cell_size, 5 * cell_size, 0xFF00AAFF);
            
            // Player 2
            PongApp::fill(&mut buffer, width, height, start_x + (app.grid_w - 2) * cell_size, start_y + app.p2_y * cell_size, cell_size, 5 * cell_size, 0xFFFF0000);
            
            // Ball
            PongApp::fill(&mut buffer, width, height, start_x + app.ball_x * cell_size, start_y + app.ball_y * cell_size, cell_size, cell_size, 0xFFFFFFFF);
            
            app.needs_redraw = false;
            
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::yield_now();
    }
}
