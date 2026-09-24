use alloc::vec::Vec;
use crate::drivers::video;
use crate::drivers::keyboard;
use super::logic::{get_valid_moves};
use spin::Mutex;

pub static ACTIVE_STATE: Mutex<Option<GameChess>> = Mutex::new(None);
pub static SAVED_STATE: Mutex<Option<GameChess>> = Mutex::new(None);

#[derive(Clone)]
pub struct GameChess {
    board: [[char; 8]; 8],
    cursor_x: i32,
    cursor_y: i32,
    selected: Option<(i32, i32)>,
    valid_moves: Vec<(i32, i32)>,
    white_turn: bool,
}

impl GameChess {
    fn new() -> Self {
        Self {
            board: [
                ['r','n','b','q','k','b','n','r'],
                ['p','p','p','p','p','p','p','p'],
                [' ',' ',' ',' ',' ',' ',' ',' '],
                [' ',' ',' ',' ',' ',' ',' ',' '],
                [' ',' ',' ',' ',' ',' ',' ',' '],
                [' ',' ',' ',' ',' ',' ',' ',' '],
                ['P','P','P','P','P','P','P','P'],
                ['R','N','B','Q','K','B','N','R'],
            ],
            cursor_x: 4,
            cursor_y: 6,
            selected: None,
            valid_moves: Vec::new(),
            white_turn: true,
        }
    }
}

// 8x8 font for pieces
fn draw_piece_fb(x: i32, y: i32, piece: char, scale: i32, color: u32) {
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
                video::fill_rect((x + (col_idx * scale)) as i64, (y + (row_idx as i32 * scale)) as i64, scale as i64, scale as i64, color);
            }
        }
    }
}

pub fn run() {
    video::clear();
    let (w, h) = video::get_resolution();
    
    let mut game = ACTIVE_STATE.lock().take().unwrap_or_else(|| GameChess::new());
    
    let top_margin = 80;
    let render_h = if h > top_margin { h - top_margin } else { h };
    let available_dim = (render_h as i32 - 40).min(w as i32 - 40).max(64);
    let sq_size = available_dim / 8;
    let board_size = sq_size * 8;
    let start_x = (w as i32 / 2) - (board_size / 2);
    let start_y = (render_h as i32 / 2) - (board_size / 2) + top_margin as i32;

    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            // Draw page background
            video::fill_rect(0, 0, w as i64, h as i64, 0xFF0D0D1A);
            video::put_str_at(2, 1, "CHESS - AINUX GAMES (2D MODE)", 0xFFFFFFFF, 0xFF0D0D1A);
            video::put_str_at(2, 2, "Use ARROW KEYS/WASD to move cursor. ENTER/SPACE to select/drop. Q to quit.", 0xFFAAAAAA, 0xFF0D0D1A);
            
            // Draw 2D Board
            for row in 0..8 {
                for col in 0..8 {
                    let render_row = if game.white_turn { row } else { 7 - row };
                    let render_col = if game.white_turn { col } else { 7 - col };
                    
                    let px = start_x + (render_col * sq_size);
                    let py = start_y + (render_row * sq_size);
                    
                    let mut bg_color = if (render_row + render_col) % 2 == 0 { 0xFFCCCCCC } else { 0xFF333333 };
                    
                    let is_selected = game.selected == Some((col, row));
                    let is_cursor = game.cursor_x == col && game.cursor_y == row;
                    
                    let is_valid = game.valid_moves.contains(&(col, row));
                    
                    if is_selected {
                        bg_color = 0xFF88AA33;
                    } else if is_cursor {
                        bg_color = 0xFF666666;
                    }
                    
                    video::fill_rect(px as i64, py as i64, sq_size as i64, sq_size as i64, bg_color);

                    // Highlight valid moves
                    if is_valid {
                        let target_piece = game.board[row as usize][col as usize];
                        if target_piece != ' ' {
                            // Capture (Red border)
                            video::fill_rect(px as i64, py as i64, sq_size as i64, 4, 0xFFFF0000);
                            video::fill_rect(px as i64, (py + sq_size - 4) as i64, sq_size as i64, 4, 0xFFFF0000);
                            video::fill_rect(px as i64, py as i64, 4, sq_size as i64, 0xFFFF0000);
                            video::fill_rect((px + sq_size - 4) as i64, py as i64, 4, sq_size as i64, 0xFFFF0000);
                        } else {
                            // Normal move (Blue dot)
                            video::fill_rect((px + (sq_size/2) - 4) as i64, (py + (sq_size/2) - 4) as i64, 8, 8, 0xFF00AAFF);
                        }
                    }
                    
                    let piece = game.board[row as usize][col as usize];
                    if piece != ' ' {
                        let fg_color = if piece.is_uppercase() { 0xFFFFFFFF } else { 0xFF000000 };
                        let out_color = if piece.is_uppercase() { 0xFF000000 } else { 0xFFFFFFFF };
                        
                        let piece_scale = (sq_size / 10).max(1);
                        let offset = (sq_size - (8 * piece_scale)) / 2;
                        
                        // Draw outline
                        draw_piece_fb(px + offset - 2, py + offset, piece, piece_scale, out_color);
                        draw_piece_fb(px + offset + 2, py + offset, piece, piece_scale, out_color);
                        draw_piece_fb(px + offset, py + offset - 2, piece, piece_scale, out_color);
                        draw_piece_fb(px + offset, py + offset + 2, piece, piece_scale, out_color);
                        
                        // Draw core
                        draw_piece_fb(px + offset, py + offset, piece, piece_scale, fg_color);
                    }
                }
            }
            needs_redraw = false;
        }

        let mut handle_move = |dx: i32, dy: i32, shift: bool, game: &mut GameChess| {
            if shift && game.selected.is_none() && game.board[game.cursor_y as usize][game.cursor_x as usize] != ' ' {
                let piece = game.board[game.cursor_y as usize][game.cursor_x as usize];
                if piece.is_uppercase() == game.white_turn {
                    // Quick pickup
                    game.selected = Some((game.cursor_x, game.cursor_y));
                    game.valid_moves = get_valid_moves(&game.board, game.cursor_x, game.cursor_y);
                }
            }
            
            game.cursor_x = (game.cursor_x + dx).clamp(0, 7);
            game.cursor_y = (game.cursor_y + dy).clamp(0, 7);

            if shift && game.selected.is_some() {
                // Quick drag
                if game.valid_moves.contains(&(game.cursor_x, game.cursor_y)) {
                    let (sx, sy) = game.selected.unwrap();
                    game.board[game.cursor_y as usize][game.cursor_x as usize] = game.board[sy as usize][sx as usize];
                    game.board[sy as usize][sx as usize] = ' ';
                    game.selected = Some((game.cursor_x, game.cursor_y));
                    game.valid_moves = get_valid_moves(&game.board, game.cursor_x, game.cursor_y);
                    game.white_turn = !game.white_turn; // toggle turn after drag and drop
                    game.selected = None; // clear selection to prevent multiple moves in one drag
                    game.valid_moves.clear();
                } else {
                    // Revert cursor if drag is invalid
                    game.cursor_x -= dx;
                    game.cursor_y -= dy;
                }
            }
            needs_redraw = true;
        };

        let c = keyboard::get_char();
        let shift = keyboard::is_shift_active();
        let up_dy = if game.white_turn { -1 } else { 1 };
        let dn_dy = if game.white_turn { 1 } else { -1 };
        let lf_dx = if game.white_turn { -1 } else { 1 };
        let rt_dx = if game.white_turn { 1 } else { -1 };
        
        match c {
            'q' | 'Q' | '\x1B' | '\x03' => {
                break;
            },
            keyboard::KEY_UP | 'w' | 'W' => handle_move(0, up_dy, shift, &mut game),
            keyboard::KEY_DOWN | 's' | 'S' => handle_move(0, dn_dy, shift, &mut game),
            keyboard::KEY_LEFT | 'a' | 'A' => handle_move(lf_dx, 0, shift, &mut game),
            keyboard::KEY_RIGHT | 'd' | 'D' => handle_move(rt_dx, 0, shift, &mut game),
            '\n' | '\r' | ' ' => {
                if let Some((sx, sy)) = game.selected {
                    if game.valid_moves.contains(&(game.cursor_x, game.cursor_y)) {
                        game.board[game.cursor_y as usize][game.cursor_x as usize] = game.board[sy as usize][sx as usize];
                        game.board[sy as usize][sx as usize] = ' ';
                        game.white_turn = !game.white_turn; // toggle turn
                    }
                    game.selected = None;
                    game.valid_moves.clear();
                    needs_redraw = true;
                } else {
                    let piece = game.board[game.cursor_y as usize][game.cursor_x as usize];
                    if piece != ' ' {
                        let is_white_piece = piece.is_uppercase();
                        if is_white_piece == game.white_turn {
                            game.selected = Some((game.cursor_x, game.cursor_y));
                            game.valid_moves = get_valid_moves(&game.board, game.cursor_x, game.cursor_y);
                            needs_redraw = true;
                        }
                    }
                }
            },
            _ => {}
        }
    }
    
    *ACTIVE_STATE.lock() = Some(game);
    video::clear();
}
