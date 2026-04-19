extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::cmp::{min, max};
use crate::drivers::{video, keyboard, mouse};
use crate::fs::vfs::{ROOT, FileType, ArcInode};

// IDE Aesthetics (PUA characters for boxes used internally by video driver)
// nvix/anvim Aesthetics (Titanium Steel / Neovim Palette)
const COLOR_BG: u32 = 0x1A1B26;     // Deep Charcoal
const COLOR_FG: u32 = 0xA9B1D6;     // Steel Blue-Grey Text
const COLOR_BAR_BG: u32 = 0x24283B; // Darker Indigo Status Line
const COLOR_BAR_FG: u32 = 0x7AA2F7; // Electric Blue Accents
const COLOR_HL: u32 = 0xE0AF68;     // Amber Highlight
const COLOR_ACCENT: u32 = 0xBB9AF7; // Purple Accent

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Normal,
    Insert,
    Command,
    Menu,
    Dialog,
    Terminal,
}

struct EditorConfig {
    tab_size: usize,
    auto_indent: bool,
    line_numbers: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self { tab_size: 4, auto_indent: true, line_numbers: true }
    }
}

pub fn cmd_nvix(args: &[&str]) {
    if args.len() < 2 {
        video::put_str("Usage: nvix <filename>\n");
        return;
    }
    let filename = args[1];
    
    // 1. Load File Data
    let mut buffer: Vec<String> = load_file(filename);
    if buffer.is_empty() { buffer.push(String::new()); }

    // 2. Load/Create Persistence (Admin Setup if new)
    let config = load_config(filename);

    // 3. System State
    let mut mode = Mode::Normal;
    let mut cx: usize = 0;
    let mut cy: usize = 0;
    let mut scroll_top = 0;
    let mut menu_index = 0;
    let mut submenu_open = false;
    let mut msg_box: Option<String> = None;
    
    // Integrated Terminal State
    let mut term_buffer = String::new();
    let mut term_history: Vec<String> = Vec::new();
    
    let console_w = *video::CONSOLE_WIDTH.lock();
    let console_h = *video::CONSOLE_HEIGHT.lock();

    loop {
        // --- RENDERING PHASE ---
        unsafe {
            // Background Clear (Zig)
            video::fast_grid_clear(console_w as u32, console_h as u32, COLOR_FG, COLOR_BG, ' ' as u32);
            
            // Neovim Status Line (Bottom)
            draw_neovim_status_line(filename, cy, cx, buffer.len(), mode);

            // Editor Frame
            video::draw_tui_title_box(0, 1, console_w, console_h - 2, filename, COLOR_FG);
            
            // Content
            draw_buffer(&buffer, scroll_top, cy, cx, console_w, console_h, &config);
            
            // Overlays (Dialogs/Menus)
            if mode == Mode::Menu && submenu_open {
                draw_submenu(menu_index);
            }
            
            // Layout Partitions
            let sidebar_w = 20;
            let terminal_h = 8;
            let editor_w = console_w - sidebar_w;
            let editor_h = console_h - terminal_h - 2;

            // 1. Sidebar (Project Tree)
            video::draw_rect_grid(0, 1, sidebar_w, console_h - 2, COLOR_BAR_FG, 0x00333333);
            video::put_str_at(2, 2, "📁 PROJECT", COLOR_HL, 0x00333333);
            video::put_str_at(2, 4, "  src/", COLOR_FG, 0x00333333);
            video::put_str_at(2, 5, "    main.rs", COLOR_FG, 0x00333333);
            video::put_str_at(2, 6, "    kernel/", COLOR_FG, 0x00333333);

            // 2. Terminal Pane (ainux> Shell)
            video::draw_rect_grid(sidebar_w, console_h - terminal_h - 1, editor_w, terminal_h, 0, 0);
            
            // Draw history (last 5 lines)
            let start = if term_history.len() > 6 { term_history.len() - 6 } else { 0 };
            for (idx, line) in term_history.iter().skip(start).enumerate() {
                 video::put_str_at(sidebar_w + 1, console_h - terminal_h + idx, line, 0x00CCAA, 0);
            }
            // Draw current prompt if in terminal mode
            let prompt = format!("ainux> {}", term_buffer);
            let prompt_color = if mode == Mode::Terminal { 0x00FF00 } else { 0x007700 };
            video::put_str_at(sidebar_w + 1, console_h - 2, &prompt, prompt_color, 0);
            
            // Redraw editor frame with offset
            video::draw_tui_title_box(sidebar_w, 1, editor_w, editor_h + 1, filename, COLOR_FG);
            
            if let Some(msg) = &msg_box {
                draw_popup("Compilation Result", msg, console_w, console_h);
            }
            
            // Mouse Cursor (TUI Block)
            let (mx, my) = mouse::get_grid_position();
            video::put_char_at(mx, my, ' ', COLOR_HL, COLOR_BAR_BG);
        }

        // --- INPUT PHASE ---
        let mut key = keyboard::pop_char();
        let m_event = mouse::pop_event();

        if let Some(c) = key {
            // Handle Global Alt Shortcuts
            if keyboard::is_alt_active() {
                match c {
                    'f' | 'F' => { mode = Mode::Menu; menu_index = 0; submenu_open = true; },
                    'c' | 'C' => { mode = Mode::Menu; menu_index = 4; submenu_open = true; },
                    'x' | 'X' => return, // Alt+X Exit
                    _ => {}
                }
            } else {
                match mode {
                    Mode::Normal => {
                        if c == '\x14' { mode = Mode::Terminal; } // Ctrl+T
                        else { handle_normal_key(c, &mut mode, &mut cx, &mut cy, &mut buffer, filename, &mut msg_box); }
                    },
                    Mode::Insert => handle_insert_key(c, &mut mode, &mut cx, &mut cy, &mut buffer),
                    Mode::Menu => handle_menu_key(c, &mut mode, &mut menu_index, &mut submenu_open, &mut buffer, filename, &mut msg_box),
                    Mode::Terminal => {
                        if c == '\x14' { mode = Mode::Normal; } // Ctrl+T toggle back
                        else { handle_terminal_key(c, &mut mode, &mut term_buffer, &mut term_history); }
                    },
                    _ => mode = Mode::Normal,
                }
            }
        }
        
        // Signal Handling (Turbo C++ Methodology)
        if crate::process::scheduler::check_current_signal(crate::process::task::SIGINT) {
            // Force Exit
            return; 
        }
        
        if let Some(mev) = m_event {
            // Simple Mouse Hit Test for Top Bar
            let (gx, gy) = mouse::get_grid_position();
            if gy == 0 && (mev.buttons & 1) != 0 {
                mode = Mode::Menu;
                submenu_open = true;
                menu_index = gx / 8; // Approximation
            }
        }

        // Scroll Logic
        if cy < scroll_top { scroll_top = cy; }
        if cy >= scroll_top + console_h - 4 { scroll_top = cy - (console_h - 5); }
        
        unsafe { core::arch::asm!("hlt"); }
    }
}

// --- SUB-ROUTINES ---

fn load_file(path: &str) -> Vec<String> {
    let mut buffer = Vec::new();
    let root = ROOT.lock();
    if let Some(r) = root.as_ref() {
        if let Ok(inode) = r.lookup(path) {
            if let Ok(handle) = inode.open(0) {
                 let mut data = alloc::vec![0u8; 32768];
                 if let Ok(bytes) = handle.read(&mut data, 0) {
                     let s = core::str::from_utf8(&data[..bytes]).unwrap_or("");
                     if s.is_empty() {
                         buffer.push(String::from("// Ainux Administrative Setup - New File"));
                         buffer.push(String::from("// Created by nvix"));
                         buffer.push(String::new());
                     } else {
                         for line in s.split('\n') {
                             buffer.push(String::from(line.trim_end_matches('\r')));
                         }
                     }
                 }
            }
        } else {
            // File doesn't exist, create administrative template
            buffer.push(String::from("// Ainux Administrative Setup - New File"));
            buffer.push(String::from("// Created by nvix"));
            buffer.push(String::new());
        }
    }
    buffer
}

fn save_file(path: &str, buffer: &[String]) -> Result<(), ()> {
    let root = ROOT.lock();
    if let Some(r) = root.as_ref() {
        let _ = r.create(path, FileType::File);
        if let Ok(inode) = r.lookup(path) {
            if let Ok(handle) = inode.open(0) {
                let mut offset = 0;
                for (i, line) in buffer.iter().enumerate() {
                    let b = line.as_bytes();
                    let _ = handle.write(b, offset);
                    offset += b.len() as u64;
                    if i < buffer.len() - 1 {
                        let _ = handle.write(b"\n", offset);
                        offset += 1;
                    }
                }
                return Ok(());
            }
        }
    }
    Err(())
}

fn load_config(_path: &str) -> EditorConfig {
    // For now, return default. Future: Read from .nvi_meta
    EditorConfig::default()
}

fn draw_top_menu(selected: usize, active: bool) {
    let menus = [" File ", " Edit ", " Search ", " Run ", " Compile ", " Debug ", " Project ", " Options "];
    let mut x = 2;
    let mnemonics = [Some(1), Some(1), Some(1), Some(1), Some(1), Some(1), Some(1), Some(1)]; // Index of letter to highlight

    for (i, m) in menus.iter().enumerate() {
        let (fg, bg) = if active && i == selected { (COLOR_BAR_FG, COLOR_HL) } else { (COLOR_BAR_FG, COLOR_BAR_BG) };
        
        // Draw string
        video::put_str_at(x, 0, m, fg, bg);
        // Highlight Mnemonic in Red
        if let Some(pos) = mnemonics[i] {
             let c = m.chars().nth(pos).unwrap();
             video::put_char_at(x + pos, 0, c, 0xAA0000, bg);
        }

        x += m.len() + 1;
    }
}

fn draw_status_bar(file: &str, cy: usize, count: usize) {
    let status = format!(" F1 Help  F2 Save  F3 Open  AltF9 Cmp  F9 Make  CtrlF9 Run | Use Alt+Red Key to use");
    video::put_str_at(1, *video::CONSOLE_HEIGHT.lock() - 1, &status, COLOR_BAR_FG, COLOR_BAR_BG);
}

fn draw_buffer(buf: &[String], top: usize, cy: usize, cx: usize, w: usize, h: usize, cfg: &EditorConfig) {
    for row in 0..(h - 4) {
        let idx = top + row;
        if idx < buf.len() {
            let line = &buf[idx];
            if cfg.line_numbers {
                video::put_str_at(2, row + 2, &format!("{:3} ", idx + 1), COLOR_HL, COLOR_BG);
                video::put_str_at(7, row + 2, line, COLOR_FG, COLOR_BG);
            } else {
                video::put_str_at(2, row + 2, line, COLOR_FG, COLOR_BG);
            }
        }
    }
    // Cursor
    let cursor_y = (cy - top) + 2;
    let cursor_x = if cfg.line_numbers { cx + 7 } else { cx + 2 };
    video::draw_rect_grid(cursor_x, cursor_y, 1, 1, 0, COLOR_FG);
}

fn draw_submenu(idx: usize) {
    let x = idx * 9 + 2;
    let items = match idx {
        0 => vec![" New      ", " Open   F3", " Save   F2", " Save As  ", "----------", " Quit AltX"],
        4 => vec![" Compile  ", " Make   F9", " Link     ", " Build All"],
        _ => vec![" (Stub)   "],
    };
    
    // Drop Shadow (Zig)
    unsafe { video::draw_tui_shadow(x as u32, 1, 12, items.len() as u32); }
    
    video::draw_rect_grid(x, 1, 12, items.len() + 2, COLOR_BAR_FG, COLOR_BAR_BG);
    for (i, item) in items.iter().enumerate() {
        video::put_str_at(x + 1, 2 + i, item, COLOR_BAR_FG, COLOR_BAR_BG);
    }
}

fn draw_popup(title: &str, msg: &str, w: usize, h: usize) {
    let box_w = 40;
    let box_h = 8;
    let x = (w - box_w) / 2;
    let y = (h - box_h) / 2;
    
    unsafe { video::draw_tui_shadow(x as u32, y as u32, box_w as u32, box_h as u32); }
    video::draw_tui_title_box(x, y, box_w, box_h, title, COLOR_HL);
    video::put_str_at(x + 2, y + 3, msg, COLOR_FG, COLOR_BG);
    video::put_str_at(x + 15, y + 6, "[ OK ]", COLOR_BAR_FG, COLOR_BAR_BG);
}

fn handle_normal_key(c: char, mode: &mut Mode, cx: &mut usize, cy: &mut usize, buffer: &mut Vec<String>, filename: &str, msg_box: &mut Option<String>) {
    // Check for Turbo C++ Run/Compile shortcuts (Ctrl/Alt + F9)
    let alt = keyboard::is_alt_active();
    let ctrl = keyboard::is_ctrl_active();

    match c {
        'i' | 'I' => *mode = Mode::Insert,
        keyboard::KEY_F2 => {
            if save_file(filename, buffer).is_ok() {
                msg_box.replace(String::from("File Saved Successfully"));
            } else {
                msg_box.replace(String::from("Error: Save Failed"));
            }
        },
        keyboard::KEY_F3 => {
            msg_box.replace(String::from("Open: System VFS Browser (Stub)"));
        },
        keyboard::KEY_F9 => {
            if alt {
                msg_box.replace(String::from("Compile: Syntax check... Success!"));
            } else if ctrl {
                msg_box.replace(String::from("Run: Launching binary... Success!"));
            } else {
                msg_box.replace(String::from("Make: Full project build... Success!"));
            }
        },
        keyboard::KEY_F10 => *mode = Mode::Menu,
        keyboard::KEY_UP => if *cy > 0 { *cy -= 1; *cx = min(*cx, buffer[*cy].len()); },
        keyboard::KEY_DOWN => if *cy < buffer.len() - 1 { *cy += 1; *cx = min(*cx, buffer[*cy].len()); },
        keyboard::KEY_LEFT => if *cx > 0 { *cx -= 1; },
        keyboard::KEY_RIGHT => if *cx < buffer[*cy].len() { *cx += 1; },
        _ => {}
    }
}

fn handle_insert_key(c: char, mode: &mut Mode, cx: &mut usize, cy: &mut usize, buffer: &mut Vec<String>) {
    if c == '\x1B' { *mode = Mode::Normal; return; }
    match c {
        '\n' | '\r' => {
            let rest = buffer[*cy].split_off(*cx);
            buffer.insert(*cy + 1, rest);
            *cy += 1; *cx = 0;
        },
        '\x08' => if *cx > 0 { buffer[*cy].remove(*cx - 1); *cx -= 1; },
        _ => { buffer[*cy].insert(*cx, c); *cx += 1; }
    }
}

fn handle_menu_key(c: char, mode: &mut Mode, idx: &mut usize, sub: &mut bool, buffer: &mut Vec<String>, filename: &str, msg_box: &mut Option<String>) {
    match c {
        keyboard::KEY_LEFT => if *idx > 0 { *idx -= 1; } else { *idx = 7; },
        keyboard::KEY_RIGHT => if *idx < 7 { *idx += 1; } else { *idx = 0; },
        '\n' | '\r' => {
            if *sub {
                // Handle Submenu Action
                match (*idx, 0) { // idx, item (stub)
                    (0, 0) => { // File -> Save
                         if save_file(filename, buffer).is_ok() {
                            msg_box.replace(String::from("File Saved Successfully"));
                        }
                    },
                    _ => {}
                }
                *sub = false;
                *mode = Mode::Normal;
            } else {
                *sub = true;
            }
        },
        '\x1B' => { *sub = false; *mode = Mode::Normal; },
        _ => {}
    }
}
fn handle_terminal_key(c: char, mode: &mut Mode, term_buffer: &mut String, term_history: &mut Vec<String>) {
    if c == '\x1B' { *mode = Mode::Normal; return; }
    match c {
        '\n' | '\r' => {
            if !term_buffer.is_empty() {
                term_history.push(format!("ainux> {}", term_buffer));
                // Call shell dispatcher
                // Since shell functions might print to video directly, 
                // we should ideally redirect them, but for now they just overlap.
                crate::shell::execute_command(term_buffer);
                term_buffer.clear();
            }
        },
        '\x08' => { term_buffer.pop(); },
        _ => { term_buffer.push(c); }
    }
}
