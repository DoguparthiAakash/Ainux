use alloc::vec::Vec;

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

pub fn minesweeper_main() {
    let id = 9;
    let width = 240;
    let height = 260;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("Minesweeper"),
        x: 300,
        y: 200,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFFC0C0C0; width * height];
    let mut app = MinesweeperApp::new();
    let mut needs_redraw = true;
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::MouseClick { x, y, button } => {
                    if app.game_over || app.won {
                        app = MinesweeperApp::new();
                        needs_redraw = true;
                        continue;
                    }

                    let off_x = 10i32;
                    let off_y = 24i32;
                    let gx = (x - off_x) / CELL;
                    let gy = (y - off_y) / CELL;
                    if gx < 0 || gy < 0 || gx >= GRID_W as i32 || gy >= GRID_H as i32 { continue; }
                    let idx = gy as usize * GRID_W + gx as usize;

                    if button & 1 != 0 {
                        // Left click = reveal
                        if !app.cells[idx].flagged && !app.cells[idx].revealed {
                            if app.cells[idx].is_mine {
                                app.cells[idx].revealed = true;
                                app.game_over = true;
                            } else {
                                app.reveal_flood(gx as usize, gy as usize);
                                if app.check_win() { app.won = true; }
                            }
                            needs_redraw = true;
                        }
                    } else if button & 2 != 0 {
                        // Right click = flag
                        if !app.cells[idx].revealed {
                            if app.cells[idx].flagged {
                                app.cells[idx].flagged = false;
                                app.flags_left += 1;
                            } else {
                                app.cells[idx].flagged = true;
                                app.flags_left -= 1;
                            }
                            needs_redraw = true;
                        }
                    }
                }
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    if c == 'r' || c == 'R' {
                        app = MinesweeperApp::new();
                        needs_redraw = true;
                    }
                }
            }
        }
        
        if needs_redraw {
            // Background
            MinesweeperApp::fill(&mut buffer, width, height, 0, 0, width as i32, height as i32, 0xFFC0C0C0);

            // Status bar
            let status = if app.game_over    { "   BOOM! You hit a mine." }
                         else if app.won     { "   You Win! All clear.  " }
                         else                 { "   Minesweeper          " };
            MinesweeperApp::fill(&mut buffer, width, height, 0, 0, width as i32, 20, 0xFF808080);
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 2, 2, status, 0xFFFFFF00);

            // Flags remaining
            let flags_str = {
                let mut s = alloc::string::String::from("Flags: ");
                if app.flags_left < 0 {
                    s.push('-');
                    let v = (-app.flags_left) as u32;
                    s.push((b'0' + (v % 10) as u8) as char);
                } else {
                    s.push((b'0' + (app.flags_left / 10) as u8) as char);
                    s.push((b'0' + (app.flags_left % 10) as u8) as char);
                }
                s
            };
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, (width as i32 - 80) as i64, 2, &flags_str, 0xFF000000);

            // Grid
            let off_x = 10i32;
            let off_y = 24i32;

            for gy in 0..GRID_H {
                for gx in 0..GRID_W {
                    let px = off_x + gx as i32 * CELL;
                    let py = off_y + gy as i32 * CELL;
                    let cell = &app.cells[gy * GRID_W + gx];

                    if cell.revealed {
                        if cell.is_mine {
                            // Bomb — red square
                            MinesweeperApp::fill(&mut buffer, width, height, px, py, CELL, CELL, 0xFFFF4444);
                            MinesweeperApp::fill(&mut buffer, width, height, px+4, py+4, CELL-8, CELL-8, 0xFF000000);
                        } else {
                            MinesweeperApp::fill(&mut buffer, width, height, px, py, CELL, CELL, 0xFFDDDDDD);
                            // Recessed border
                            MinesweeperApp::fill(&mut buffer, width, height, px, py, CELL, 1, 0xFF808080);
                            MinesweeperApp::fill(&mut buffer, width, height, px, py, 1, CELL, 0xFF808080);
                            if cell.neighbors > 0 {
                                let colors = [0xFF0000FF, 0xFF008000, 0xFFFF0000,
                                              0xFF000080, 0xFF800000, 0xFF008080,
                                              0xFF333333, 0xFF808080];
                                let c = colors[(cell.neighbors - 1) as usize % 8];
                                let ch = (b'0' + cell.neighbors) as char;
                                video::draw_char_to_buffer(&mut buffer, width, height, (px + 8) as usize, (py + 4) as usize, ch, c);
                            }
                        }
                    } else if cell.flagged {
                        // Raised + red flag indicator
                        MinesweeperApp::fill(&mut buffer, width, height, px, py, CELL, CELL, 0xFFC0C0C0);
                        MinesweeperApp::fill(&mut buffer, width, height, px, py, CELL, 1, 0xFFFFFFFF);
                        MinesweeperApp::fill(&mut buffer, width, height, px, py, 1, CELL, 0xFFFFFFFF);
                        MinesweeperApp::fill(&mut buffer, width, height, px, py+CELL-1, CELL, 1, 0xFF808080);
                        MinesweeperApp::fill(&mut buffer, width, height, px+CELL-1, py, 1, CELL, 0xFF808080);
                        MinesweeperApp::fill(&mut buffer, width, height, px+6, py+6, CELL-12, CELL-12, 0xFFFF0000);
                        video::draw_char_to_buffer(&mut buffer, width, height, (px + 8) as usize, (py + 4) as usize, 'F', 0xFF000000);
                    } else {
                        // Unrevealed — raised 3D look
                        MinesweeperApp::fill(&mut buffer, width, height, px, py, CELL, CELL, 0xFFC0C0C0);
                        MinesweeperApp::fill(&mut buffer, width, height, px, py, CELL, 1, 0xFFFFFFFF);
                        MinesweeperApp::fill(&mut buffer, width, height, px, py, 1, CELL, 0xFFFFFFFF);
                        MinesweeperApp::fill(&mut buffer, width, height, px, py+CELL-1, CELL, 1, 0xFF555555);
                        MinesweeperApp::fill(&mut buffer, width, height, px+CELL-1, py, 1, CELL, 0xFF555555);
                    }
                }
            }

            needs_redraw = false;
            
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::yield_now();
    }
}
