use alloc::vec::Vec;
use crate::drivers::video;
use crate::drivers::keyboard;
use super::logic::{get_valid_moves};

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

// 8x8 font for pieces
fn draw_piece_buf(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, piece: char, scale: i32, color: u32) {
    let p_idx = match piece.to_ascii_lowercase() {
        'p' => 0,
        'r' => 1,
        'n' => 2,
        'b' => 3,
        'q' => 4,
        'k' => 5,
        _ => return,
    };
    
    let font: [[u8; 8]; 6] = [
        // Pawn
        [0b00000000, 0b00011000, 0b00111100, 0b00011000, 0b00011000, 0b00111100, 0b01111110, 0b01111110], 
        // Rook
        [0b01010100, 0b01111100, 0b00111100, 0b00111100, 0b00111100, 0b00111100, 0b01111110, 0b01111110],
        // Knight
        [0b00011000, 0b00111100, 0b01111100, 0b01110000, 0b00111000, 0b00011100, 0b01111110, 0b01111110],
        // Bishop
        [0b00010000, 0b00010000, 0b00111000, 0b00111000, 0b01101100, 0b00111000, 0b01111110, 0b01111110],
        // Queen
        [0b01010100, 0b01010100, 0b01111100, 0b00111000, 0b00111000, 0b00111000, 0b01111110, 0b01111110],
        // King
        [0b00010000, 0b00111000, 0b00010000, 0b01111100, 0b01111100, 0b00111000, 0b01111110, 0b01111110],
    ];
    
    let bitmap = font[p_idx];
    for (row_idx, row_val) in bitmap.iter().enumerate() {
        for col_idx in 0..8 {
            if (row_val & (1 << (7 - col_idx))) != 0 {
                fill_rect_buf(buf, w, h, x + (col_idx * scale), y + (row_idx as i32 * scale), scale, scale, color);
            }
        }
    }
}

use crate::gui::compositor::Compositor;
use crate::gui::window::Window;

pub fn run() {
    video::clear();
    let (w, h) = video::get_resolution();
    
    let mut board = [
        ['r','n','b','q','k','b','n','r'],
        ['p','p','p','p','p','p','p','p'],
        [' ',' ',' ',' ',' ',' ',' ',' '],
        [' ',' ',' ',' ',' ',' ',' ',' '],
        [' ',' ',' ',' ',' ',' ',' ',' '],
        [' ',' ',' ',' ',' ',' ',' ',' '],
        ['P','P','P','P','P','P','P','P'],
        ['R','N','B','Q','K','B','N','R'],
    ];

    let mut cursor_x = 4;
    let mut cursor_y = 6;
    let mut selected: Option<(i32, i32)> = None;
    let mut valid_moves: Vec<(i32, i32)> = Vec::new();
    let mut white_turn = true;
    
    let sq_size = 60;
    let board_size = sq_size * 8;
    
    Compositor::init();
    let mut win = Window::new(99, 100, 100, board_size as usize, (board_size + 36) as usize, "Ainux Chess");
    win.allocate_content();
    Compositor::add_window(win);

    loop {
        let mut wins = crate::gui::compositor::WINDOWS.lock();
        let win = wins.iter_mut().find(|w| w.id == 99).unwrap();
        
        let w_width = win.width;
        let w_height = win.height;
        let mut backbuffer = &mut win.content;
        backbuffer.fill(0xFF0D0D1A);

        let start_x = 0;
        let start_y = 0;

        // Draw 2D Board
        for row in 0..8 {
            for col in 0..8 {
                let px = start_x + (col * sq_size);
                let py = start_y + (row * sq_size);
                
                let mut bg_color = if (row + col) % 2 == 0 { 0xFFCCCCCC } else { 0xFF333333 };
                
                let is_selected = selected == Some((col, row));
                let is_cursor = cursor_x == col && cursor_y == row;
                
                let is_valid = valid_moves.contains(&(col, row));
                
                if is_selected {
                    bg_color = 0xFF88AA33;
                } else if is_cursor {
                    bg_color = 0xFF666666;
                }
                
                fill_rect_buf(&mut backbuffer, w_width, w_height, px, py, sq_size, sq_size, bg_color);

                // Highlight valid moves
                if is_valid {
                    let target_piece = board[row as usize][col as usize];
                    if target_piece != ' ' {
                        // Capture (Red border)
                        fill_rect_buf(&mut backbuffer, w_width, w_height, px, py, sq_size, 4, 0xFFFF0000);
                        fill_rect_buf(&mut backbuffer, w_width, w_height, px, py + sq_size - 4, sq_size, 4, 0xFFFF0000);
                        fill_rect_buf(&mut backbuffer, w_width, w_height, px, py, 4, sq_size, 0xFFFF0000);
                        fill_rect_buf(&mut backbuffer, w_width, w_height, px + sq_size - 4, py, 4, sq_size, 0xFFFF0000);
                    } else {
                        // Normal move (Blue dot)
                        fill_rect_buf(&mut backbuffer, w_width, w_height, px + (sq_size/2) - 4, py + (sq_size/2) - 4, 8, 8, 0xFF00AAFF);
                    }
                }
                
                let piece = board[row as usize][col as usize];
                if piece != ' ' {
                    let fg_color = if piece.is_uppercase() { 0xFFFFFFFF } else { 0xFF000000 };
                    let out_color = if piece.is_uppercase() { 0xFF000000 } else { 0xFFFFFFFF };
                    
                    let piece_scale = 5;
                    let offset = (sq_size - (8 * piece_scale)) / 2;
                    
                    // Draw outline
                    draw_piece_buf(&mut backbuffer, w_width, w_height, px + offset - 2, py + offset, piece, piece_scale, out_color);
                    draw_piece_buf(&mut backbuffer, w_width, w_height, px + offset + 2, py + offset, piece, piece_scale, out_color);
                    draw_piece_buf(&mut backbuffer, w_width, w_height, px + offset, py + offset - 2, piece, piece_scale, out_color);
                    draw_piece_buf(&mut backbuffer, w_width, w_height, px + offset, py + offset + 2, piece, piece_scale, out_color);
                    
                    // Draw core
                    draw_piece_buf(&mut backbuffer, w_width, w_height, px + offset, py + offset, piece, piece_scale, fg_color);
                }
            }
        }
        
        // Drop lock before render
        drop(wins);
        Compositor::render();

        let mut handle_move = |dx: i32, dy: i32, shift: bool| {
            if shift && selected.is_none() && board[cursor_y as usize][cursor_x as usize] != ' ' {
                let piece = board[cursor_y as usize][cursor_x as usize];
                if piece.is_uppercase() == white_turn {
                    // Quick pickup
                    selected = Some((cursor_x, cursor_y));
                    valid_moves = get_valid_moves(&board, cursor_x, cursor_y);
                }
            }
            
            cursor_x = (cursor_x + dx).clamp(0, 7);
            cursor_y = (cursor_y + dy).clamp(0, 7);

            if shift && selected.is_some() {
                // Quick drag
                if valid_moves.contains(&(cursor_x, cursor_y)) {
                    let (sx, sy) = selected.unwrap();
                    board[cursor_y as usize][cursor_x as usize] = board[sy as usize][sx as usize];
                    board[sy as usize][sx as usize] = ' ';
                    selected = Some((cursor_x, cursor_y));
                    valid_moves = get_valid_moves(&board, cursor_x, cursor_y);
                    white_turn = !white_turn; // toggle turn after drag and drop
                    selected = None; // clear selection to prevent multiple moves in one drag
                    valid_moves.clear();
                } else {
                    // Revert cursor if drag is invalid
                    cursor_x -= dx;
                    cursor_y -= dy;
                }
            }
        };

        if let Some(c) = keyboard::pop_char() {
            let shift = keyboard::is_shift_active();
            match c {
                'q' | 'Q' | '\x1B' => {
                    // Remove window from compositor
                    let mut wins = crate::gui::compositor::WINDOWS.lock();
                    wins.retain(|w| w.id != 99);
                    break;
                },
                keyboard::KEY_UP => handle_move(0, -1, shift),
                keyboard::KEY_DOWN => handle_move(0, 1, shift),
                keyboard::KEY_LEFT => handle_move(-1, 0, shift),
                keyboard::KEY_RIGHT => handle_move(1, 0, shift),
                '\n' | '\r' | ' ' => {
                    if let Some((sx, sy)) = selected {
                        if valid_moves.contains(&(cursor_x, cursor_y)) {
                            board[cursor_y as usize][cursor_x as usize] = board[sy as usize][sx as usize];
                            board[sy as usize][sx as usize] = ' ';
                            white_turn = !white_turn; // toggle turn
                        }
                        selected = None;
                        valid_moves.clear();
                    } else {
                        let piece = board[cursor_y as usize][cursor_x as usize];
                        if piece != ' ' {
                            let is_white_piece = piece.is_uppercase();
                            if is_white_piece == white_turn {
                                selected = Some((cursor_x, cursor_y));
                                valid_moves = get_valid_moves(&board, cursor_x, cursor_y);
                            }
                        }
                    }
                },
                _ => {}
            }
        }
    }
    video::clear();
}
