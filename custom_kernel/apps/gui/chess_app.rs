use crate::gui::app::App;
use crate::drivers::keyboard;
use alloc::vec::Vec;
use crate::games::chess::logic::get_valid_moves;

fn draw_piece_fb(buf: &mut [u32], buf_w: usize, buf_h: usize, x: i32, y: i32, piece: char, scale: i32, color: u32) {
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
                ChessApp::fill(buf, buf_w, buf_h, x + (col_idx * scale), y + (row_idx as i32 * scale), scale, scale, color);
            }
        }
    }
}

pub struct ChessApp {
    board: [[char; 8]; 8],
    cursor_x: i32,
    cursor_y: i32,
    selected: Option<(i32, i32)>,
    valid_moves: Vec<(i32, i32)>,
    white_turn: bool,
    needs_redraw: bool,
}

impl ChessApp {
    pub fn new() -> Self {
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

    fn handle_move(&mut self, dx: i32, dy: i32, shift: bool) {
        if shift && self.selected.is_none() && self.board[self.cursor_y as usize][self.cursor_x as usize] != ' ' {
            let piece = self.board[self.cursor_y as usize][self.cursor_x as usize];
            if piece.is_uppercase() == self.white_turn {
                self.selected = Some((self.cursor_x, self.cursor_y));
                self.valid_moves = get_valid_moves(&self.board, self.cursor_x, self.cursor_y);
            }
        }
        
        self.cursor_x = (self.cursor_x + dx).clamp(0, 7);
        self.cursor_y = (self.cursor_y + dy).clamp(0, 7);

        if shift && self.selected.is_some() {
            if self.valid_moves.contains(&(self.cursor_x, self.cursor_y)) {
                let (sx, sy) = self.selected.unwrap();
                self.board[self.cursor_y as usize][self.cursor_x as usize] = self.board[sy as usize][sx as usize];
                self.board[sy as usize][sx as usize] = ' ';
                self.selected = Some((self.cursor_x, self.cursor_y));
                self.valid_moves = get_valid_moves(&self.board, self.cursor_x, self.cursor_y);
                self.white_turn = !self.white_turn; 
                self.selected = None; 
                self.valid_moves.clear();
            } else {
                self.cursor_x -= dx;
                self.cursor_y -= dy;
            }
        }
        self.needs_redraw = true;
    }
}

impl App for ChessApp {
    fn update(&mut self) {}

    fn draw(&mut self, buf: &mut [u32], w: usize, h: usize) {
        if !self.needs_redraw { return; }
        
        Self::fill(buf, w, h, 0, 0, w as i32, h as i32, 0xFF0D0D1A);
        
        crate::drivers::video::draw_text_to_buffer(buf, w as i64, h as i64, 4, 4, "CHESS", 0xFFFFFFFF);
        
        let available_dim = (h as i32 - 30).min(w as i32 - 10).max(64);
        let sq_size = available_dim / 8;
        let board_size = sq_size * 8;
        let start_x = (w as i32 / 2) - (board_size / 2);
        let start_y = (h as i32 / 2) - (board_size / 2) + 10;

        for row in 0..8 {
            for col in 0..8 {
                let render_row = if self.white_turn { row } else { 7 - row };
                let render_col = if self.white_turn { col } else { 7 - col };
                
                let px = start_x + (render_col * sq_size);
                let py = start_y + (render_row * sq_size);
                
                let mut bg_color = if (render_row + render_col) % 2 == 0 { 0xFFCCCCCC } else { 0xFF333333 };
                
                let is_selected = self.selected == Some((col, row));
                let is_cursor = self.cursor_x == col && self.cursor_y == row;
                
                let is_valid = self.valid_moves.contains(&(col, row));
                
                if is_selected {
                    bg_color = 0xFF88AA33;
                } else if is_cursor {
                    bg_color = 0xFF666666;
                }
                
                Self::fill(buf, w, h, px, py, sq_size, sq_size, bg_color);

                if is_valid {
                    let target_piece = self.board[row as usize][col as usize];
                    if target_piece != ' ' {
                        Self::fill(buf, w, h, px, py, sq_size, 4, 0xFFFF0000);
                        Self::fill(buf, w, h, px, py + sq_size - 4, sq_size, 4, 0xFFFF0000);
                        Self::fill(buf, w, h, px, py, 4, sq_size, 0xFFFF0000);
                        Self::fill(buf, w, h, px + sq_size - 4, py, 4, sq_size, 0xFFFF0000);
                    } else {
                        Self::fill(buf, w, h, px + (sq_size/2) - 4, py + (sq_size/2) - 4, 8, 8, 0xFF00AAFF);
                    }
                }
                
                let piece = self.board[row as usize][col as usize];
                if piece != ' ' {
                    let fg_color = if piece.is_uppercase() { 0xFFFFFFFF } else { 0xFF000000 };
                    let out_color = if piece.is_uppercase() { 0xFF000000 } else { 0xFFFFFFFF };
                    
                    let piece_scale = (sq_size / 10).max(1);
                    let offset = (sq_size - (8 * piece_scale)) / 2;
                    
                    draw_piece_fb(buf, w, h, px + offset - 2, py + offset, piece, piece_scale, out_color);
                    draw_piece_fb(buf, w, h, px + offset + 2, py + offset, piece, piece_scale, out_color);
                    draw_piece_fb(buf, w, h, px + offset, py + offset - 2, piece, piece_scale, out_color);
                    draw_piece_fb(buf, w, h, px + offset, py + offset + 2, piece, piece_scale, out_color);
                    draw_piece_fb(buf, w, h, px + offset, py + offset, piece, piece_scale, fg_color);
                }
            }
        }
        self.needs_redraw = false;
    }

    fn on_mouse_event(&mut self, _x: i32, _y: i32, _buttons: u8) {}

    fn on_key_event(&mut self, c: char) {
        let shift = keyboard::is_shift_active();
        let up_dy = if self.white_turn { -1 } else { 1 };
        let dn_dy = if self.white_turn { 1 } else { -1 };
        let lf_dx = if self.white_turn { -1 } else { 1 };
        let rt_dx = if self.white_turn { 1 } else { -1 };
        
        match c {
            keyboard::KEY_UP | 'w' | 'W' => self.handle_move(0, up_dy, shift),
            keyboard::KEY_DOWN | 's' | 'S' => self.handle_move(0, dn_dy, shift),
            keyboard::KEY_LEFT | 'a' | 'A' => self.handle_move(lf_dx, 0, shift),
            keyboard::KEY_RIGHT | 'd' | 'D' => self.handle_move(rt_dx, 0, shift),
            '\n' | '\r' | ' ' => {
                if let Some((sx, sy)) = self.selected {
                    if self.valid_moves.contains(&(self.cursor_x, self.cursor_y)) {
                        self.board[self.cursor_y as usize][self.cursor_x as usize] = self.board[sy as usize][sx as usize];
                        self.board[sy as usize][sx as usize] = ' ';
                        self.white_turn = !self.white_turn; // toggle turn
                    }
                    self.selected = None;
                    self.valid_moves.clear();
                    self.needs_redraw = true;
                } else {
                    let piece = self.board[self.cursor_y as usize][self.cursor_x as usize];
                    if piece != ' ' {
                        let is_white_piece = piece.is_uppercase();
                        if is_white_piece == self.white_turn {
                            self.selected = Some((self.cursor_x, self.cursor_y));
                            self.valid_moves = get_valid_moves(&self.board, self.cursor_x, self.cursor_y);
                            self.needs_redraw = true;
                        }
                    }
                }
            },
            _ => {}
        }
    }
}
