use crate::drivers::video;
use crate::drivers::keyboard;
use alloc::vec::Vec;
use spin::Mutex;

pub static ACTIVE_STATE: Mutex<Option<GameMinesweeper>> = Mutex::new(None);
pub static SAVED_STATE: Mutex<Option<GameMinesweeper>> = Mutex::new(None);

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

// Win95 inspired colors
const COLOR_BG: u32 = 0xFFC0C0C0;
const COLOR_SHADOW: u32 = 0xFF808080;
const COLOR_HIGHLIGHT: u32 = 0xFFFFFFFF;
const COLOR_REVEALED: u32 = 0xFFC0C0C0;

fn get_number_color(num: u8) -> u32 {
    match num {
        1 => 0xFF0000FF, // Blue
        2 => 0xFF008000, // Green
        3 => 0xFFFF0000, // Red
        4 => 0xFF000080, // Dark Blue
        5 => 0xFF800000, // Dark Red
        6 => 0xFF008080, // Cyan
        7 => 0xFF000000, // Black
        8 => 0xFF808080, // Gray
        _ => 0xFF000000,
    }
}

#[derive(Clone)]
struct MineCell {
    is_mine: bool,
    is_revealed: bool,
    is_flagged: bool,
    neighbor_mines: u8,
}

#[derive(Clone)]
pub struct GameMinesweeper {
    grid: Vec<Vec<MineCell>>,
    cursor_x: usize,
    cursor_y: usize,
    game_over: bool,
    won: bool,
}

impl GameMinesweeper {
    fn new() -> Self {
        let grid_w = 16;
        let grid_h = 16;
        let total_mines = 40;
        
        let mut grid = Vec::with_capacity(grid_h);
        for _ in 0..grid_h {
            let mut row = Vec::with_capacity(grid_w);
            for _ in 0..grid_w {
                row.push(MineCell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    neighbor_mines: 0,
                });
            }
            grid.push(row);
        }

        // Place mines randomly
        let mut rng = crate::cpu::cpuid::rdtsc();
        let mut mines_placed = 0;
        while mines_placed < total_mines {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            
            let r_x = (rng as usize) % grid_w;
            let r_y = ((rng >> 16) as usize) % grid_h;
            
            if !grid[r_y][r_x].is_mine {
                grid[r_y][r_x].is_mine = true;
                mines_placed += 1;
            }
        }
        
        // Calculate neighbors
        for y in 0..grid_h {
            for x in 0..grid_w {
                if grid[y][x].is_mine { continue; }
                let mut count = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let ny = y as i32 + dy;
                        let nx = x as i32 + dx;
                        if ny >= 0 && ny < grid_h as i32 && nx >= 0 && nx < grid_w as i32 {
                            if grid[ny as usize][nx as usize].is_mine {
                                count += 1;
                            }
                        }
                    }
                }
                grid[y][x].neighbor_mines = count;
            }
        }

        Self {
            grid,
            cursor_x: 0,
            cursor_y: 0,
            game_over: false,
            won: false,
        }
    }
}

pub fn run() {
    video::clear();
    let (w, h) = video::get_resolution();
    
    let mut game = ACTIVE_STATE.lock().take().unwrap_or_else(|| GameMinesweeper::new());
    
    let grid_w = 16;
    let grid_h = 16;
    let total_mines = 40;
    let mut needs_redraw = true;

    let top_margin = 80;
    let render_h = if h > top_margin { h - top_margin } else { h };

    let available_dim = (render_h as i32 - 40).min(w as i32 - 40).max(64);
    let cell_size = available_dim / grid_h as i32; // Assuming grid_h == grid_w (16x16)
    
    let board_w = grid_w as i32 * cell_size;
    let board_h = grid_h as i32 * cell_size;
    let start_x = (w as i32 / 2) - (board_w / 2);
    let start_y = (render_h as i32 / 2) - (board_h / 2) + top_margin as i32;

    loop {
        if needs_redraw {
            // Draw Header
            video::fill_rect(0, 0, w as i64, top_margin as i64, COLOR_BG);
            video::put_str_at(2, 1, "MINESWEEPER - AINUX GAMES", 0xFF000000, COLOR_BG);
            video::put_str_at(2, 2, "ARROWS/WASD to move, SPACE/ENTER to dig, F or SHIFT to flag. Q to quit.", 0xFF555555, COLOR_BG);
            
            // bg
            video::fill_rect(0, top_margin as i64, w as i64, render_h as i64, COLOR_BG);
            
            // Draw Board background recess
            video::fill_rect((start_x - 4) as i64, (start_y - 4) as i64, (board_w + 8) as i64, (board_h + 8) as i64, COLOR_SHADOW);
            video::fill_rect(start_x as i64, start_y as i64, board_w as i64, board_h as i64, COLOR_REVEALED);

            for y in 0..grid_h {
                for x in 0..grid_w {
                    let cell = &game.grid[y][x];
                    let px = start_x + (x as i32 * cell_size);
                    let py = start_y + (y as i32 * cell_size);
                    
                    if !cell.is_revealed {
                        // Draw unrevealed button
                        video::fill_rect(px as i64, py as i64, cell_size as i64, cell_size as i64, COLOR_BG);
                        video::fill_rect(px as i64, py as i64, (cell_size - 2) as i64, 2, COLOR_HIGHLIGHT); // top
                        video::fill_rect(px as i64, py as i64, 2, (cell_size - 2) as i64, COLOR_HIGHLIGHT); // left
                        video::fill_rect((px + cell_size - 2) as i64, py as i64, 2, cell_size as i64, COLOR_SHADOW); // right
                        video::fill_rect(px as i64, (py + cell_size - 2) as i64, cell_size as i64, 2, COLOR_SHADOW); // bottom
                        
                        if cell.is_flagged {
                            // Draw flag
                            video::fill_rect((px + cell_size/2) as i64, (py + 8) as i64, (cell_size/8).max(2) as i64, (cell_size - 16).max(2) as i64, 0xFF000000); // pole
                            video::fill_rect((px + 8) as i64, (py + 8) as i64, (cell_size/2 - 8).max(2) as i64, (cell_size/4).max(2) as i64, 0xFFFF0000); // flag
                        }
                    } else {
                        // Draw revealed cell
                        video::fill_rect(px as i64, py as i64, cell_size as i64, cell_size as i64, COLOR_REVEALED);
                        video::fill_rect(px as i64, py as i64, cell_size as i64, 1, COLOR_SHADOW);
                        video::fill_rect(px as i64, py as i64, 1, cell_size as i64, COLOR_SHADOW);
                        
                        if cell.is_mine {
                            // Draw bomb
                            let b_size = (cell_size * 6) / 10;
                            let b_offset = (cell_size - b_size) / 2;
                            video::fill_rect((px + b_offset) as i64, (py + b_offset) as i64, b_size as i64, b_size as i64, 0xFF000000);
                        } else if cell.neighbor_mines > 0 {
                            let scale = (cell_size / 15).max(1);
                            let text_w = 5 * scale;
                            let t_px = px + (cell_size - text_w) / 2;
                            let t_py = py + (cell_size - (5 * scale)) / 2;
                            draw_digit_fb(t_px, t_py, cell.neighbor_mines, scale, get_number_color(cell.neighbor_mines));
                        }
                    }
                    
                    // Cursor overlay
                    if x == game.cursor_x && y == game.cursor_y {
                        let overlay = if cell.is_revealed { 0x550000FF } else { 0x55FFFF00 };
                        // Simplified cursor box overlay
                        video::fill_rect(px as i64, py as i64, cell_size as i64, 4, overlay);
                        video::fill_rect(px as i64, (py + cell_size - 4) as i64, cell_size as i64, 4, overlay);
                        video::fill_rect(px as i64, py as i64, 4, cell_size as i64, overlay);
                        video::fill_rect((px + cell_size - 4) as i64, py as i64, 4, cell_size as i64, overlay);
                    }
                }
            }
            needs_redraw = false;
        }

        let c = keyboard::get_char(); // block
        match c {
            'q' | 'Q' | '\x1B' => break,
            'w' | 'W' | keyboard::KEY_UP => { if game.cursor_y > 0 { game.cursor_y -= 1; } needs_redraw = true; },
            's' | 'S' | keyboard::KEY_DOWN => { if game.cursor_y < grid_h - 1 { game.cursor_y += 1; } needs_redraw = true; },
            'a' | 'A' | keyboard::KEY_LEFT => { if game.cursor_x > 0 { game.cursor_x -= 1; } needs_redraw = true; },
            'd' | 'D' | keyboard::KEY_RIGHT => { if game.cursor_x < grid_w - 1 { game.cursor_x += 1; } needs_redraw = true; },
            'f' | 'F' => {
                if !game.game_over && !game.won && !game.grid[game.cursor_y][game.cursor_x].is_revealed {
                    game.grid[game.cursor_y][game.cursor_x].is_flagged = !game.grid[game.cursor_y][game.cursor_x].is_flagged;
                    needs_redraw = true;
                }
            }
            ' ' | '\n' | '\r' => {
                if keyboard::is_shift_active() {
                    // alternative flag
                    if !game.game_over && !game.won && !game.grid[game.cursor_y][game.cursor_x].is_revealed {
                        game.grid[game.cursor_y][game.cursor_x].is_flagged = !game.grid[game.cursor_y][game.cursor_x].is_flagged;
                        needs_redraw = true;
                    }
                } else if !game.game_over && !game.won && !game.grid[game.cursor_y][game.cursor_x].is_flagged {
                    // Reveal
                    if game.grid[game.cursor_y][game.cursor_x].is_mine {
                        game.game_over = true;
                        // Reveal all mines
                        for row in game.grid.iter_mut() {
                            for c in row.iter_mut() {
                                if c.is_mine { c.is_revealed = true; }
                            }
                        }
                    } else {
                        // Flood fill reveal
                        let mut queue = Vec::new();
                        queue.push((game.cursor_x, game.cursor_y));
                        
                        while let Some((cx, cy)) = queue.pop() {
                            if game.grid[cy][cx].is_revealed || game.grid[cy][cx].is_flagged { continue; }
                            game.grid[cy][cx].is_revealed = true;
                            
                            if game.grid[cy][cx].neighbor_mines == 0 {
                                for dy in -1..=1 {
                                    for dx in -1..=1 {
                                        let ny = cy as i32 + dy;
                                        let nx = cx as i32 + dx;
                                        if ny >= 0 && ny < grid_h as i32 && nx >= 0 && nx < grid_w as i32 {
                                            queue.push((nx as usize, ny as usize));
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Check win condition
                        let mut revealed_count = 0;
                        for row in game.grid.iter() {
                            for c in row.iter() {
                                if c.is_revealed { revealed_count += 1; }
                            }
                        }
                        if revealed_count == grid_w * grid_h - total_mines {
                            game.won = true;
                        }
                    }
                    needs_redraw = true;
                }
            }
            _ => {}
        }
    }
    
    *ACTIVE_STATE.lock() = Some(game);
    video::clear();
}
