use alloc::vec::Vec;
use crate::gui::app::App;
use crate::drivers::video;

const GRID_W: usize = 9;
const GRID_H: usize = 9;
const CELL: i32 = 24;

// Deterministic mine positions (indices into the 9x9 grid)
const MINE_POS: [usize; 10] = [2, 8, 14, 20, 26, 33, 40, 47, 54, 60];

#[derive(Clone)]
struct Cell {
    is_mine: bool,
    revealed: bool,
    flagged: bool,
    neighbors: u8,
}

pub struct MinesweeperApp {
    cells: Vec<Cell>,
    game_over: bool,
    won: bool,
    flags_left: i32,
}

impl MinesweeperApp {
    pub fn new() -> Self {
        let mut cells = Vec::new();
        for _ in 0..(GRID_W * GRID_H) {
            cells.push(Cell { is_mine: false, revealed: false, flagged: false, neighbors: 0 });
        }

        // Place mines
        for &p in &MINE_POS {
            cells[p].is_mine = true;
        }

        // Compute neighbor counts
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                if cells[y * GRID_W + x].is_mine { continue; }
                let mut cnt = 0u8;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && ny >= 0 && nx < GRID_W as i32 && ny < GRID_H as i32 {
                            if cells[ny as usize * GRID_W + nx as usize].is_mine { cnt += 1; }
                        }
                    }
                }
                cells[y * GRID_W + x].neighbors = cnt;
            }
        }

        Self { cells, game_over: false, won: false, flags_left: MINE_POS.len() as i32 }
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

    fn reveal_flood(&mut self, x: usize, y: usize) {
        if x >= GRID_W || y >= GRID_H { return; }
        let idx = y * GRID_W + x;
        if self.cells[idx].revealed || self.cells[idx].flagged { return; }
        self.cells[idx].revealed = true;
        if self.cells[idx].neighbors == 0 && !self.cells[idx].is_mine {
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && ny >= 0 && nx < GRID_W as i32 && ny < GRID_H as i32 {
                        self.reveal_flood(nx as usize, ny as usize);
                    }
                }
            }
        }
    }

    fn check_win(&self) -> bool {
        self.cells.iter().filter(|c| !c.revealed && !c.is_mine).count() == 0
    }
}

impl App for MinesweeperApp {
    fn update(&mut self) {}

    fn draw(&mut self, buf: &mut [u32], w: usize, h: usize) {
        // Background
        Self::fill(buf, w, h, 0, 0, w as i32, h as i32, 0xFFC0C0C0);

        // Status bar
        let status = if self.game_over    { "   BOOM! You hit a mine. Click to restart." }
                     else if self.won     { "   You Win! All clear. Click to restart.  " }
                     else                 { "   Minesweeper  [L=reveal  R=flag]        " };
        Self::fill(buf, w, h, 0, 0, w as i32, 20, 0xFF808080);
        video::draw_text_to_buffer(buf, w as i64, h as i64, 2, 2, status, 0xFFFFFF00);

        // Flags remaining
        let flags_str = {
            let mut s = alloc::string::String::from("Flags: ");
            if self.flags_left < 0 {
                s.push('-');
                let v = (-self.flags_left) as u32;
                s.push((b'0' + (v % 10) as u8) as char);
            } else {
                s.push((b'0' + (self.flags_left / 10) as u8) as char);
                s.push((b'0' + (self.flags_left % 10) as u8) as char);
            }
            s
        };
        video::draw_text_to_buffer(buf, w as i64, h as i64, (w as i32 - 80) as i64, 2, &flags_str, 0xFF000000);

        // Grid
        let off_x = 10i32;
        let off_y = 24i32;

        for gy in 0..GRID_H {
            for gx in 0..GRID_W {
                let px = off_x + gx as i32 * CELL;
                let py = off_y + gy as i32 * CELL;
                let cell = &self.cells[gy * GRID_W + gx];

                if cell.revealed {
                    if cell.is_mine {
                        // Bomb — red square
                        Self::fill(buf, w, h, px, py, CELL, CELL, 0xFFFF4444);
                        Self::fill(buf, w, h, px+4, py+4, CELL-8, CELL-8, 0xFF000000);
                    } else {
                        Self::fill(buf, w, h, px, py, CELL, CELL, 0xFFDDDDDD);
                        // Recessed border
                        Self::fill(buf, w, h, px, py, CELL, 1, 0xFF808080);
                        Self::fill(buf, w, h, px, py, 1, CELL, 0xFF808080);
                        if cell.neighbors > 0 {
                            let colors = [0xFF0000FF, 0xFF008000, 0xFFFF0000,
                                          0xFF000080, 0xFF800000, 0xFF008080,
                                          0xFF333333, 0xFF808080];
                            let c = colors[(cell.neighbors - 1) as usize % 8];
                            let ch = (b'0' + cell.neighbors) as char;
                            video::draw_char_to_buffer(buf, w, h, (px + 8) as usize, (py + 4) as usize, ch, c);
                        }
                    }
                } else if cell.flagged {
                    // Raised + red flag indicator
                    Self::fill(buf, w, h, px, py, CELL, CELL, 0xFFC0C0C0);
                    Self::fill(buf, w, h, px, py, CELL, 1, 0xFFFFFFFF);
                    Self::fill(buf, w, h, px, py, 1, CELL, 0xFFFFFFFF);
                    Self::fill(buf, w, h, px, py+CELL-1, CELL, 1, 0xFF808080);
                    Self::fill(buf, w, h, px+CELL-1, py, 1, CELL, 0xFF808080);
                    Self::fill(buf, w, h, px+6, py+6, CELL-12, CELL-12, 0xFFFF0000);
                    video::draw_char_to_buffer(buf, w, h, (px + 8) as usize, (py + 4) as usize, 'F', 0xFF000000);
                } else {
                    // Unrevealed — raised 3D look
                    Self::fill(buf, w, h, px, py, CELL, CELL, 0xFFC0C0C0);
                    Self::fill(buf, w, h, px, py, CELL, 1, 0xFFFFFFFF);
                    Self::fill(buf, w, h, px, py, 1, CELL, 0xFFFFFFFF);
                    Self::fill(buf, w, h, px, py+CELL-1, CELL, 1, 0xFF555555);
                    Self::fill(buf, w, h, px+CELL-1, py, 1, CELL, 0xFF555555);
                }
            }
        }
    }

    fn on_mouse_event(&mut self, x: i32, y: i32, buttons: u8) {
        if buttons == 0 { return; }

        if self.game_over || self.won {
            *self = MinesweeperApp::new();
            return;
        }

        let off_x = 10i32;
        let off_y = 24i32;
        let gx = (x - off_x) / CELL;
        let gy = (y - off_y) / CELL;
        if gx < 0 || gy < 0 || gx >= GRID_W as i32 || gy >= GRID_H as i32 { return; }
        let idx = gy as usize * GRID_W + gx as usize;

        if buttons & 1 != 0 {
            // Left click = reveal
            if !self.cells[idx].flagged && !self.cells[idx].revealed {
                if self.cells[idx].is_mine {
                    self.cells[idx].revealed = true;
                    self.game_over = true;
                } else {
                    self.reveal_flood(gx as usize, gy as usize);
                    if self.check_win() { self.won = true; }
                }
            }
        } else if buttons & 2 != 0 {
            // Right click = flag
            if !self.cells[idx].revealed {
                if self.cells[idx].flagged {
                    self.cells[idx].flagged = false;
                    self.flags_left += 1;
                } else {
                    self.cells[idx].flagged = true;
                    self.flags_left -= 1;
                }
            }
        }
    }

    fn on_key_event(&mut self, c: char) {
        if c == 'r' || c == 'R' {
            *self = MinesweeperApp::new();
        }
    }
}
