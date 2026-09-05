use alloc::vec::Vec;

pub fn is_white(p: char) -> bool { p != ' ' && p.is_uppercase() }
pub fn is_black(p: char) -> bool { p != ' ' && p.is_lowercase() }

pub fn get_valid_moves(board: &[[char; 8]; 8], sx: i32, sy: i32) -> Vec<(i32, i32)> {
    let mut moves = Vec::new();
    let piece = board[sy as usize][sx as usize];
    if piece == ' ' { return moves; }
    let white = is_white(piece);
    let p_type = piece.to_ascii_lowercase();

    let mut add_if_valid = |nx: i32, ny: i32| -> bool {
        if nx >= 0 && nx < 8 && ny >= 0 && ny < 8 {
            let target = board[ny as usize][nx as usize];
            if target == ' ' {
                moves.push((nx, ny));
                return true; 
            } else if is_white(target) != white {
                moves.push((nx, ny));
                return false; 
            } else {
                return false; 
            }
        }
        false
    };

    match p_type {
        'p' => {
            let dir = if white { -1 } else { 1 };
            let start_row = if white { 6 } else { 1 };
            
            // Forward 1
            if sy + dir >= 0 && sy + dir < 8 && board[(sy + dir) as usize][sx as usize] == ' ' {
                moves.push((sx, sy + dir));
                // Forward 2
                if sy == start_row && board[(sy + 2*dir) as usize][sx as usize] == ' ' {
                    moves.push((sx, sy + 2*dir));
                }
            }
            // Captures
            for dx in [-1, 1] {
                if sx + dx >= 0 && sx + dx < 8 && sy + dir >= 0 && sy + dir < 8 {
                    let target = board[(sy + dir) as usize][(sx + dx) as usize];
                    if target != ' ' && is_white(target) != white {
                        moves.push((sx + dx, sy + dir));
                    }
                }
            }
        },
        'n' => {
            let jumps = [(-2,-1), (-2,1), (-1,-2), (-1,2), (1,-2), (1,2), (2,-1), (2,1)];
            for (dx, dy) in jumps { add_if_valid(sx + dx, sy + dy); }
        },
        'r' => {
            for (dx, dy) in [(0,1), (0,-1), (1,0), (-1,0)] {
                for step in 1..8 { if !add_if_valid(sx + dx * step, sy + dy * step) { break; } }
            }
        },
        'b' => {
            for (dx, dy) in [(1,1), (1,-1), (-1,1), (-1,-1)] {
                for step in 1..8 { if !add_if_valid(sx + dx * step, sy + dy * step) { break; } }
            }
        },
        'q' => {
            for (dx, dy) in [(0,1), (0,-1), (1,0), (-1,0), (1,1), (1,-1), (-1,1), (-1,-1)] {
                for step in 1..8 { if !add_if_valid(sx + dx * step, sy + dy * step) { break; } }
            }
        },
        'k' => {
            for (dx, dy) in [(0,1), (0,-1), (1,0), (-1,0), (1,1), (1,-1), (-1,1), (-1,-1)] {
                add_if_valid(sx + dx, sy + dy);
            }
        },
        _ => {}
    }
    moves
}
