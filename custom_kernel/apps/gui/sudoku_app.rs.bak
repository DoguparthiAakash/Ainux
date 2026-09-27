
use crate::drivers::keyboard;
use crate::games::sudoku::logic::get_default_grid;

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
                SudokuApp::fill(buf, buf_w, buf_h, x + (col_idx * scale), y + (row_idx as i32 * scale), scale, scale, color);
            }
        }
    }
}

pub struct SudokuApp {
    grid: [[u8; 9]; 9],
    original_grid: [[u8; 9]; 9],
    cursor_x: usize,
    cursor_y: usize,
    needs_redraw: bool,
}

impl SudokuApp {
    pub fn new() -> Self {
        let grid = get_default_grid();
        let original_grid = grid.clone();
        Self {
            grid,
            original_grid,
            cursor_x: 0,
            cursor_y: 0,
            needs_redraw: true,
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

pub fn sudoku_main() {
    let id = 16;
    let width = 360;
    let height = 400;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("Sudoku"),
        x: 220,
        y: 220,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFF0D0D1A; width * height];
    let mut app = SudokuApp::new();
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    match c {
                        'w' | 'W' | keyboard::KEY_UP => { if app.cursor_y > 0 { app.cursor_y -= 1; app.needs_redraw = true; } },
                        's' | 'S' | keyboard::KEY_DOWN => { if app.cursor_y < 8 { app.cursor_y += 1; app.needs_redraw = true; } },
                        'a' | 'A' | keyboard::KEY_LEFT => { if app.cursor_x > 0 { app.cursor_x -= 1; app.needs_redraw = true; } },
                        'd' | 'D' | keyboard::KEY_RIGHT => { if app.cursor_x < 8 { app.cursor_x += 1; app.needs_redraw = true; } },
                        '1'..='9' => {
                            if app.original_grid[app.cursor_y][app.cursor_x] == 0 {
                                if let Some(digit) = c.to_digit(10) {
                                    app.grid[app.cursor_y][app.cursor_x] = digit as u8;
                                    app.needs_redraw = true;
                                }
                            }
                        },
                        '0' | ' ' | '\x08' => {
                            if app.original_grid[app.cursor_y][app.cursor_x] == 0 {
                                app.grid[app.cursor_y][app.cursor_x] = 0;
                                app.needs_redraw = true;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        
        if app.needs_redraw {
            SudokuApp::fill(&mut buffer, width, height, 0, 0, width as i32, height as i32, 0xFF0D0D1A);
            
            crate::drivers::video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 4, 4, "SUDOKU", 0xFFFFFFFF);
            
            let available_dim = (height as i32 - 30).min(width as i32 - 10).max(81);
            let cell_size = available_dim / 9;
            let board_size_2d = cell_size * 9;
            let start_x = (width as i32 / 2) - (board_size_2d / 2);
            let start_y = (height as i32 / 2) - (board_size_2d / 2) + 10;
            
            for y in 0..9 {
                for x in 0..9 {
                    let px = start_x + (x as i32 * cell_size);
                    let py = start_y + (y as i32 * cell_size);
                    
                    let bg_color = if x == app.cursor_x && y == app.cursor_y {
                        0xFF333333
                    } else if ((x / 3) + (y / 3)) % 2 == 0 {
                        0xFF111111
                    } else {
                        0xFF1A1A1A
                    };

                    SudokuApp::fill(&mut buffer, width, height, px, py, cell_size, cell_size, bg_color);
                    
                    // Borders
                    SudokuApp::fill(&mut buffer, width, height, px, py, cell_size, 1, 0xFF444444);
                    SudokuApp::fill(&mut buffer, width, height, px, py, 1, cell_size, 0xFF444444);

                    let val = app.grid[y][x];
                    if val != 0 {
                        let text_color = if app.original_grid[y][x] != 0 { 0xFFFFFFFF } else { 0xFF00FFCC };
                        let scale = (cell_size / 15).max(1);
                        draw_digit_fb(&mut buffer, width, height, px + (cell_size / 2) - (5 * scale / 2), py + (cell_size / 2) - (5 * scale / 2), val, scale, text_color);
                    }
                }
            }
            
            for i in 0..=3 {
                let px = start_x + (i * 3) as i32 * cell_size;
                let py = start_y + (i * 3) as i32 * cell_size;
                SudokuApp::fill(&mut buffer, width, height, px, start_y, 2, board_size_2d, 0xFF888888);
                SudokuApp::fill(&mut buffer, width, height, start_x, py, board_size_2d, 2, 0xFF888888);
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
