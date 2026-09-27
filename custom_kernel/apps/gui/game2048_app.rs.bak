
use crate::drivers::keyboard;
use alloc::vec::Vec;
use alloc::format;

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
                Game2048App::fill(buf, buf_w, buf_h, x + (col_idx * scale), y + (row_idx as i32 * scale), scale, scale, color);
            }
        }
    }
}

fn draw_number_fb(buf: &mut [u32], buf_w: usize, buf_h: usize, mut x: i32, y: i32, mut num: u32, scale: i32, color: u32) {
    if num == 0 {
        draw_digit_fb(buf, buf_w, buf_h, x, y, 0, scale, color);
        return;
    }
    
    let mut digits = Vec::new();
    while num > 0 {
        digits.push((num % 10) as u8);
        num /= 10;
    }
    
    for digit in digits.iter().rev() {
        draw_digit_fb(buf, buf_w, buf_h, x, y, *digit, scale, color);
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

pub struct Game2048App {
    grid: [[u32; 4]; 4],
    score: u32,
    game_over: bool,
    rng_state: u64,
    needs_redraw: bool,
}

impl Game2048App {
    pub fn new() -> Self {
        let mut app = Self {
            grid: [[0; 4]; 4],
            score: 0,
            game_over: false,
            rng_state: crate::cpu::cpuid::rdtsc(),
            needs_redraw: true,
        };
        app.spawn_tile();
        app.spawn_tile();
        app
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
        for y in 0..4 {
            for x in 0..4 {
                if self.grid[y][x] == 0 { return; }
            }
        }
        for y in 0..4 {
            for x in 0..3 {
                if self.grid[y][x] == self.grid[y][x+1] { return; }
            }
        }
        for y in 0..3 {
            for x in 0..4 {
                if self.grid[y][x] == self.grid[y+1][x] { return; }
            }
        }
        self.game_over = true;
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

pub fn game2048_main() {
    let id = 14;
    let width = 320;
    let height = 360;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("2048"),
        x: 200,
        y: 200,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFFFAF8EF; width * height];
    let mut app = Game2048App::new();
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    if app.game_over { continue; }
                    let mut moved = false;
                    match c {
                        'w' | 'W' | keyboard::KEY_UP => moved = app.move_up(),
                        's' | 'S' | keyboard::KEY_DOWN => moved = app.move_down(),
                        'a' | 'A' | keyboard::KEY_LEFT => moved = app.move_left(),
                        'd' | 'D' | keyboard::KEY_RIGHT => moved = app.move_right(),
                        _ => {}
                    }
                    
                    if moved {
                        app.spawn_tile();
                        app.check_game_over();
                        app.needs_redraw = true;
                    }
                }
                _ => {}
            }
        }
        
        if app.needs_redraw {
            Game2048App::fill(&mut buffer, width, height, 0, 0, width as i32, height as i32, 0xFFFAF8EF);
            
            crate::drivers::video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 4, 4, "2048", 0xFF776E65);
            let score_str = format!("SCORE: {}", app.score);
            crate::drivers::video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 4, 20, &score_str, 0xFF776E65);
            
            let available_dim = (height as i32 - 40).min(width as i32 - 10).max(100);
            let spacing = available_dim / 30;
            let cell_size = (available_dim - (5 * spacing)) / 4;
            let board_size = 4 * cell_size + 5 * spacing;
            
            let start_x = (width as i32 / 2) - (board_size / 2);
            let start_y = (height as i32 / 2) - (board_size / 2) + 15;
            
            Game2048App::fill(&mut buffer, width, height, start_x, start_y, board_size, board_size, 0xFFBBADA0);

            for y in 0..4 {
                for x in 0..4 {
                    let val = app.grid[y][x];
                    let (bg, fg) = get_color_for_value(val);
                    
                    let px = start_x + spacing + (x as i32 * (cell_size + spacing));
                    let py = start_y + spacing + (y as i32 * (cell_size + spacing));
                    
                    Game2048App::fill(&mut buffer, width, height, px, py, cell_size, cell_size, bg);
                    
                    if val > 0 {
                        let mut num_len = 0;
                        let mut temp = val;
                        while temp > 0 { num_len += 1; temp /= 10; }
                        
                        let scale = if num_len >= 4 { (cell_size / 30).max(1) } else { (cell_size / 25).max(1) };
                        let text_w = num_len as i32 * 6 * scale;
                        let text_x = px + (cell_size / 2) - (text_w / 2);
                        let text_y = py + (cell_size / 2) - (5 * scale / 2);
                        
                        draw_number_fb(&mut buffer, width, height, text_x, text_y, val, scale, fg);
                    }
                }
            }

            if app.game_over {
                crate::drivers::video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 4, 40, "GAME OVER", 0xFFFF0000);
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
