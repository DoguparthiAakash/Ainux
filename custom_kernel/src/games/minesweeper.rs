use alloc::format;
use crate::drivers::video;
use crate::drivers::keyboard;
use alloc::string::String;
use alloc::vec::Vec;

fn fill_rect_buf(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, width: i32, height: i32, color: u32) {
    let start_x = x.max(0);
    let start_y = y.max(0);
    let end_x = (x + width).min(w as i32);
    let end_y = (y + height).min(h as i32);
    
    for row in start_y..end_y {
        let row_offset = (row as usize) * w;
        for col in start_x..end_x {
            buf[row_offset + (col as usize)] = color;
        }
    }
}

fn draw_digit_buf(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, digit: u8, scale: i32, color: u32) {
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
                fill_rect_buf(buf, w, h, x + (col_idx * scale), y + (row_idx as i32 * scale), scale, scale, color);
            }
        }
    }
}

// Win95 inspired colors
const COLOR_BG: u32 = 0xFFC0C0C0;
const COLOR_SHADOW: u32 = 0xFF808080;
const COLOR_HIGHLIGHT: u32 = 0xFFFFFFFF;
const COLOR_REVEALED: u32 = 0xFFC0C0C0;
const COLOR_TEXT: u32 = 0xFF000000;

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

struct MineCell {
    is_mine: bool,
    is_revealed: bool,
    is_flagged: bool,
    neighbor_mines: u8,
}

pub fn run() {
    video::clear();
    let (w, h) = video::get_resolution();
    
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

    let mut cursor_x = 0;
    let mut cursor_y = 0;
    let mut game_over = false;
    let mut won = false;

    let top_margin = 80;
    let render_h = if h > top_margin { h - top_margin } else { h };
    let mut backbuffer = alloc::vec![COLOR_BG; w * render_h];

    video::fill_rect(0, 0, w as i64, top_margin as i64, COLOR_BG);
    video::put_str_at(2, 1, "MINESWEEPER - AINUX GAMES", 0xFF000000, COLOR_BG);
    video::put_str_at(2, 2, "ARROWS to move, SPACE/ENTER to dig, F or SHIFT+SPACE to flag. Q to quit.", 0xFF555555, COLOR_BG);

    let cell_size = 32;
    let board_w = grid_w as i32 * cell_size;
    let board_h = grid_h as i32 * cell_size;
    let start_x = (w as i32 / 2) - (board_w / 2);
    let start_y = (render_h as i32 / 2) - (board_h / 2);

    loop {
        backbuffer.fill(COLOR_BG);
        
        // Draw Board background recess
        fill_rect_buf(&mut backbuffer, w, render_h, start_x - 4, start_y - 4, board_w + 8, board_h + 8, COLOR_SHADOW);
        fill_rect_buf(&mut backbuffer, w, render_h, start_x, start_y, board_w, board_h, COLOR_REVEALED);

        for y in 0..grid_h {
            for x in 0..grid_w {
                let cell = &grid[y][x];
                let px = start_x + (x as i32 * cell_size);
                let py = start_y + (y as i32 * cell_size);
                
                if !cell.is_revealed {
                    // Draw unrevealed button
                    fill_rect_buf(&mut backbuffer, w, render_h, px, py, cell_size, cell_size, COLOR_BG);
                    fill_rect_buf(&mut backbuffer, w, render_h, px, py, cell_size - 2, 2, COLOR_HIGHLIGHT); // top
                    fill_rect_buf(&mut backbuffer, w, render_h, px, py, 2, cell_size - 2, COLOR_HIGHLIGHT); // left
                    fill_rect_buf(&mut backbuffer, w, render_h, px + cell_size - 2, py, 2, cell_size, COLOR_SHADOW); // right
                    fill_rect_buf(&mut backbuffer, w, render_h, px, py + cell_size - 2, cell_size, 2, COLOR_SHADOW); // bottom
                    
                    if cell.is_flagged {
                        // Draw flag
                        fill_rect_buf(&mut backbuffer, w, render_h, px + cell_size/2, py + 8, 2, 16, 0xFF000000); // pole
                        fill_rect_buf(&mut backbuffer, w, render_h, px + 8, py + 8, cell_size/2 - 8, 8, 0xFFFF0000); // flag
                    }
                } else {
                    // Draw revealed cell
                    fill_rect_buf(&mut backbuffer, w, render_h, px, py, cell_size, cell_size, COLOR_REVEALED);
                    fill_rect_buf(&mut backbuffer, w, render_h, px, py, cell_size, 1, COLOR_SHADOW);
                    fill_rect_buf(&mut backbuffer, w, render_h, px, py, 1, cell_size, COLOR_SHADOW);
                    
                    if cell.is_mine {
                        // Draw bomb
                        fill_rect_buf(&mut backbuffer, w, render_h, px + 10, py + 10, cell_size - 20, cell_size - 20, 0xFF000000);
                        fill_rect_buf(&mut backbuffer, w, render_h, px + 8, py + cell_size/2 - 2, cell_size - 16, 4, 0xFF000000);
                        fill_rect_buf(&mut backbuffer, w, render_h, px + cell_size/2 - 2, py + 8, 4, cell_size - 16, 0xFF000000);
                    } else if cell.neighbor_mines > 0 {
                        draw_digit_buf(&mut backbuffer, w, render_h, px + 10, py + 8, cell.neighbor_mines, 3, get_number_color(cell.neighbor_mines));
                    }
                }
                
                // Cursor overlay
                if x == cursor_x && y == cursor_y {
                    let overlay = if cell.is_revealed { 0x550000FF } else { 0x55FFFF00 };
                    for cy in 0..cell_size {
                        for cx in 0..cell_size {
                            let idx = ((py + cy) as usize) * w + ((px + cx) as usize);
                            let bg = backbuffer[idx];
                            // Alpha blend simple
                            let r = (((bg >> 16) & 0xFF) + ((overlay >> 16) & 0xFF)) / 2;
                            let g = (((bg >> 8) & 0xFF) + ((overlay >> 8) & 0xFF)) / 2;
                            let b = ((bg & 0xFF) + (overlay & 0xFF)) / 2;
                            backbuffer[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
                        }
                    }
                }
            }
        }

        video::copy_buffer_region(&backbuffer, top_margin * w);

        let c = keyboard::get_char();
        
        match c {
            'q' | 'Q' => break,
            'w' | 'W' | keyboard::KEY_UP => if cursor_y > 0 { cursor_y -= 1 },
            's' | 'S' | keyboard::KEY_DOWN => if cursor_y < grid_h - 1 { cursor_y += 1 },
            'a' | 'A' | keyboard::KEY_LEFT => if cursor_x > 0 { cursor_x -= 1 },
            'd' | 'D' | keyboard::KEY_RIGHT => if cursor_x < grid_w - 1 { cursor_x += 1 },
            'f' | 'F' => {
                if !game_over && !won && !grid[cursor_y][cursor_x].is_revealed {
                    grid[cursor_y][cursor_x].is_flagged = !grid[cursor_y][cursor_x].is_flagged;
                }
            }
            ' ' | '\n' | '\r' => {
                if keyboard::is_shift_active() {
                    // alternative flag
                    if !game_over && !won && !grid[cursor_y][cursor_x].is_revealed {
                        grid[cursor_y][cursor_x].is_flagged = !grid[cursor_y][cursor_x].is_flagged;
                    }
                } else if !game_over && !won && !grid[cursor_y][cursor_x].is_flagged {
                    // Reveal
                    if grid[cursor_y][cursor_x].is_mine {
                        game_over = true;
                        // Reveal all mines
                        for row in grid.iter_mut() {
                            for c in row.iter_mut() {
                                if c.is_mine { c.is_revealed = true; }
                            }
                        }
                    } else {
                        // Flood fill reveal
                        let mut queue = Vec::new();
                        queue.push((cursor_x, cursor_y));
                        
                        while let Some((cx, cy)) = queue.pop() {
                            if grid[cy][cx].is_revealed || grid[cy][cx].is_flagged { continue; }
                            grid[cy][cx].is_revealed = true;
                            
                            if grid[cy][cx].neighbor_mines == 0 {
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
                        for row in grid.iter() {
                            for c in row.iter() {
                                if c.is_revealed { revealed_count += 1; }
                            }
                        }
                        if revealed_count == grid_w * grid_h - total_mines {
                            won = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    video::clear();
}
