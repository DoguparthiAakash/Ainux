extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::cmp::{min, max};
use crate::drivers::{video, keyboard, mouse};
use crate::fs::vfs::{ROOT, FileType, ArcInode};

// IDE Aesthetics (PUA characters for boxes used internally by video driver)
const COLOR_BG: u32 = 0x00111122;    // Midnight Blue-Black
const COLOR_FG: u32 = 0x00DDDDDD;    // Soft White
const COLOR_BAR_BG: u32 = 0x002D2D2D;// Charcoal
const COLOR_BAR_FG: u32 = 0x00FFFFFF;// White
const COLOR_HL: u32 = 0x0000AAAA;    // Cyan Select

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Mode {
    Normal,
    Insert,
    Command,
    Menu,
    Dialog,
    Terminal,
    Explorer,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Split {
    None,
    Vertical,
    Horizontal,
}

struct EditorConfig {
    tab_size: usize,
    auto_indent: bool,
    line_numbers: bool,
    auto_save_ms: u64,
    show_output: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self { tab_size: 4, auto_indent: true, line_numbers: true, auto_save_ms: 5000, show_output: true }
    }
}

struct Document {
    path: String,
    buffer: Vec<String>,
    cx: usize,
    cy: usize,
    scroll_top: usize,
    dirty: bool,
    last_change_tick: u64,
}

impl Document {
    fn new(path: &str, content: Vec<String>) -> Self {
        Self {
            path: String::from(path),
            buffer: content,
            cx: 0,
            cy: 0,
            scroll_top: 0,
            dirty: false,
            last_change_tick: 0,
        }
    }
}

struct Workspace {
    documents: Vec<Document>,
    active_idx: usize,
    split_mode: Split,
    focus_pane: usize, // 0 or 1
    output_history: Vec<String>,
}

impl Workspace {
    fn new() -> Self {
        Self {
            documents: Vec::new(),
            active_idx: 0,
            split_mode: Split::None,
            focus_pane: 0,
            output_history: Vec::new(),
        }
    }

    fn current_doc_mut(&mut self) -> Option<&mut Document> {
        let len = self.documents.len();
        if self.split_mode == Split::None {
            self.documents.get_mut(self.active_idx)
        } else {
            self.documents.get_mut(min(self.focus_pane, len - 1))
        }
    }
}

struct FileExplorer {
    current_path: String,
    entries: Vec<(String, FileType)>,
    selected_idx: usize,
    scroll_top: usize,
}

impl FileExplorer {
    fn new(path: &str) -> Self {
        let mut explorer = Self {
            current_path: String::from(path),
            entries: Vec::new(),
            selected_idx: 0,
            scroll_top: 0,
        };
        explorer.refresh();
        explorer
    }

    fn refresh(&mut self) {
        self.entries.clear();
        let root = ROOT.lock();
        if let Some(r) = root.as_ref() {
            if let Ok(inode) = r.lookup(&self.current_path) {
                if let Ok(children) = inode.read_dir() {
                    for child in children {
                        // Stat each child to get its true type
                        let child_path = if self.current_path == "/" { format!("/{}", child) } else { format!("{}/{}", self.current_path.trim_end_matches('/'), child) };
                        let ftype = if let Ok(ci) = r.lookup(&child_path) {
                            ci.stat().map(|s| s.file_type).unwrap_or(FileType::File)
                        } else { FileType::File };
                        self.entries.push((child, ftype));
                    }
                }
            }
        }
    }
}

pub fn cmd_nvix(args: &[&str]) {
    let mut target_path = if args.len() > 1 { args[1] } else { "/" };
    
    // 1. Detect if target is directory
    let is_dir = {
        let root = ROOT.lock();
        if let Some(r) = root.as_ref() {
            if let Ok(inode) = r.lookup(target_path) {
                inode.stat().map(|s| s.file_type == FileType::Directory).unwrap_or(false)
            } else { false }
        } else { false }
    };

    let mut workspace = Workspace::new();
    let mut explorer_path = if is_dir { target_path } else { "/" };
    let mut explorer = FileExplorer::new(explorer_path);
    
    if !is_dir {
        let content = load_file(target_path);
        workspace.documents.push(Document::new(target_path, content));
    }

    // 2. System State
    let mut mode = if is_dir { Mode::Explorer } else { Mode::Normal };
    let mut menu_index = 0;
    let mut submenu_open = false;
    let mut msg_box: Option<String> = None;
    
    // Integrated Terminal State
    let mut term_buffer = String::new();
    let mut term_history: Vec<String> = Vec::new();
    
    let config = EditorConfig::default();
    let (console_w, console_h) = {
        let w = *video::CONSOLE_WIDTH.lock();
        let h = *video::CONSOLE_HEIGHT.lock();
        (w, h)
    };
    
    let sidebar_w = 22;
    let terminal_h = 7;
    let main_w = console_w - sidebar_w;
    let main_h = console_h - terminal_h - 2;

    loop {
        let current_ticks = crate::process::scheduler::get_ticks();
        
        // --- AUTO-SAVE DAEMON ---
        for doc in workspace.documents.iter_mut() {
            if doc.dirty && current_ticks > doc.last_change_tick + (config.auto_save_ms / 10) {
                let _ = save_file(&doc.path, &doc.buffer);
                doc.dirty = false;
            }
        }

        // --- RENDERING PHASE ---
        unsafe {
            video::fast_grid_clear(console_w as u32, console_h as u32, COLOR_FG, COLOR_BG, ' ' as u32);
            // Draw Workspace Layout Boxes
            // Sidebar Border
            for i in 1..(console_h - 1) { video::put_char_at(sidebar_w, i, '║', COLOR_BAR_FG, COLOR_BG); }
            // Horizontal Split (Terminal Section)
            for i in 0..console_w { video::put_char_at(i, console_h - terminal_h - 1, '═', COLOR_BAR_FG, COLOR_BG); }
            video::put_char_at(sidebar_w, console_h - terminal_h - 1, '╬', COLOR_BAR_FG, COLOR_BG);

            // 1. EXPLORER (Sidebar)
            draw_explorer(&explorer, sidebar_w, console_h - 2, mode == Mode::Explorer);

            // 2. WORKSPACE (Active Document or Split)
            if workspace.documents.is_empty() {
                video::draw_tui_title_box(sidebar_w, 1, main_w, main_h + 1, " EMPTY WORKSPACE ", COLOR_HL);
                video::put_str_at(sidebar_w + 5, 5, "Use Explorer (Tab) to open files", COLOR_BAR_BG, COLOR_BG);
            } else {
                match workspace.split_mode {
                    Split::None => {
                        let doc = workspace.current_doc_mut().unwrap();
                        video::draw_tui_title_box(sidebar_w, 1, main_w, main_h + 1, &format!(" {} ", doc.path), COLOR_FG);
                        draw_buffer(&doc.buffer, doc.scroll_top, doc.cy, doc.cx, sidebar_w, 1, main_w, main_h, &config, mode == Mode::Normal || mode == Mode::Insert);
                        if doc.dirty { video::put_str_at(sidebar_w + main_w - 10, 1, "[MODIFIED]", 0xAA0000, COLOR_FG); }
                    },
                    Split::Vertical => {
                        let split_x = sidebar_w + (main_w / 2);
                        // Pane 0
                        if let Some(doc) = workspace.documents.get_mut(0) {
                            video::draw_tui_title_box(sidebar_w, 1, main_w / 2, main_h + 1, &format!(" {} ", doc.path), COLOR_FG);
                            draw_buffer(&doc.buffer, doc.scroll_top, doc.cy, doc.cx, sidebar_w, 1, main_w / 2, main_h, &config, workspace.focus_pane == 0 && (mode == Mode::Normal || mode == Mode::Insert));
                        }
                        // Pane 1
                        let doc_idx1 = min(1, workspace.documents.len() - 1);
                        if let Some(doc) = workspace.documents.get_mut(doc_idx1) {
                            video::draw_tui_title_box(split_x, 1, main_w / 2, main_h + 1, &format!(" {} ", doc.path), COLOR_FG);
                            draw_buffer(&doc.buffer, doc.scroll_top, doc.cy, doc.cx, split_x, 1, main_w / 2, main_h, &config, workspace.focus_pane == 1 && (mode == Mode::Normal || mode == Mode::Insert));
                        }
                    },
                    Split::Horizontal => {
                         let split_y = 1 + (main_h / 2);
                         // Pane 0
                         if let Some(doc) = workspace.documents.get_mut(0) {
                            video::draw_tui_title_box(sidebar_w, 1, main_w, main_h / 2, &format!(" {} ", doc.path), COLOR_FG);
                            draw_buffer(&doc.buffer, doc.scroll_top, doc.cy, doc.cx, sidebar_w, 1, main_w, main_h / 2, &config, workspace.focus_pane == 0 && (mode == Mode::Normal || mode == Mode::Insert));
                         }
                         // Pane 1
                         let doc_idx2 = min(1, workspace.documents.len() - 1);
                         if let Some(doc) = workspace.documents.get_mut(doc_idx2) {
                            video::draw_tui_title_box(sidebar_w, split_y, main_w, main_h / 2, &format!(" {} ", doc.path), COLOR_FG);
                            draw_buffer(&doc.buffer, doc.scroll_top, doc.cy, doc.cx, sidebar_w, split_y, main_w, main_h / 2, &config, workspace.focus_pane == 1 && (mode == Mode::Normal || mode == Mode::Insert));
                         }
                    }
                }
            }

            // 3. SYSTEM PANES (Terminal & Output)
            let term_w = if config.show_output { main_w / 2 } else { main_w };
            let out_x = sidebar_w + term_w;
            let out_w = main_w - term_w;
            let term_y = console_h - terminal_h - 1;

            // TERMINAL Pane
            video::draw_rect_grid(sidebar_w, term_y, term_w, terminal_h, COLOR_FG, 0);
            video::put_str_at(sidebar_w + 1, term_y, " [ SHELL ] ", COLOR_HL, 0);
            
            let term_start = if term_history.len() > 5 { term_history.len() - 5 } else { 0 };
            for (idx, line) in term_history.iter().skip(term_start).enumerate() {
                 video::put_str_at(sidebar_w + 1, term_y + 1 + idx, line, 0x00CCAA, 0);
            }
            let prompt = format!("ainux> {}", term_buffer);
            video::put_str_at(sidebar_w + 1, console_h - 2, &prompt, if mode == Mode::Terminal { 0x00FF00 } else { 0x007700 }, 0);

            // OUTPUT Pane
            if config.show_output {
                video::draw_rect_grid(out_x, term_y, out_w, terminal_h, COLOR_FG, 0);
                video::put_str_at(out_x + 1, term_y, " [ OUTPUT ] ", COLOR_HL, 0);
                let out_start = if workspace.output_history.len() > 5 { workspace.output_history.len() - 5 } else { 0 };
                for (idx, line) in workspace.output_history.iter().skip(out_start).enumerate() {
                    video::put_str_at(out_x + 1, term_y + 1 + idx, line, 0xDDDDDD, 0);
                }
            }

            // Bottom Global Status
            video::draw_rect_grid(0, console_h - 1, console_w, 1, COLOR_BAR_FG, COLOR_BAR_BG);
            if let Some(doc) = workspace.current_doc_mut() {
                draw_status_bar(mode, doc.cy + 1, doc.cx + 1, &doc.path);
            } else {
                draw_status_bar(mode, 0, 0, "No File");
            }

            if let Some(msg) = &msg_box {
                draw_popup("IDE Message", msg, console_w, console_h);
            }
            
            // Mouse Block
            let (mx, my) = mouse::get_grid_position();
            video::put_char_at(mx, my, ' ', COLOR_HL, COLOR_BAR_BG);
        }

        // --- INPUT PHASE ---
        let mut key = keyboard::pop_char();
        let m_event = mouse::pop_event();

        if let Some(c) = key {
            if keyboard::is_alt_active() {
                match c {
                    'f' | 'F' => { mode = Mode::Menu; menu_index = 0; submenu_open = true; },
                    'c' | 'C' => { mode = Mode::Menu; menu_index = 4; submenu_open = true; },
                    'w' | 'W' => { 
                        if workspace.split_mode == Split::None { workspace.split_mode = Split::Vertical; }
                        else { workspace.focus_pane = 1 - workspace.focus_pane; } // Toggle between splits
                    },
                    'u' | 'U' => { workspace.split_mode = Split::None; workspace.focus_pane = 0; }, // Unsplit
                    'x' | 'X' => return,
                    _ => {}
                }
            } else if c == '\t' {
                // Focus Cycle
                mode = match mode {
                    Mode::Explorer => Mode::Normal,
                    Mode::Normal | Mode::Insert => Mode::Terminal,
                    Mode::Terminal => Mode::Explorer,
                    _ => Mode::Normal,
                };
            } else {
                match mode {
                    Mode::Explorer => handle_explorer_key(c, &mut explorer, &mut workspace, &mut mode),
                    Mode::Normal => {
                         if let Some(doc) = workspace.current_doc_mut() {
                             handle_normal_key(c, &mut mode, doc, &mut msg_box);
                         }
                    },
                    Mode::Insert => {
                         if let Some(doc) = workspace.current_doc_mut() {
                             handle_insert_key(c, &mut mode, doc);
                         }
                    },
                    Mode::Terminal => handle_terminal_key(c, &mut mode, &mut term_buffer, &mut term_history, &mut workspace),
                    Mode::Menu => handle_menu_key(c, &mut mode, &mut menu_index, &mut submenu_open, &mut workspace, &mut msg_box),
                    _ => mode = Mode::Normal,
                }
            }
        }
        
        if let Some(mev) = m_event {
            let (gx, gy) = mouse::get_grid_position();
            if gy == 0 && (mev.buttons & 1) != 0 {
                mode = Mode::Menu; submenu_open = true; menu_index = gx / 9;
            }
        }

        // --- SCROLL / SYMLINK ---
        if let Some(doc) = workspace.current_doc_mut() {
            if doc.cy < doc.scroll_top { doc.scroll_top = doc.cy; }
            if doc.cy >= doc.scroll_top + main_h - 1 { doc.scroll_top = doc.cy - (main_h - 2); }
        }
        
        unsafe { core::arch::asm!("hlt"); }
    }
}

// --- RENDERING SUBROUTINES ---

fn draw_explorer(exp: &FileExplorer, w: usize, h: usize, active: bool) {
    let focus_color = if active { COLOR_HL } else { COLOR_BAR_BG };
    video::draw_rect_grid(0, 1, w, h, COLOR_BAR_FG, 0x00111111);
    video::put_str_at(2, 1, " Explorer ", focus_color, 0x00111111);
    
    for (i, (entry, ftype)) in exp.entries.iter().skip(exp.scroll_top).enumerate() {
        if i >= h - 3 { break; }
        let y = i + 3;
        let is_sel = i == exp.selected_idx;
        let (fg, bg) = if is_sel { (COLOR_BAR_FG, COLOR_HL) } else { (COLOR_FG, 0x00111111) };
        
        let icon = match ftype {
            FileType::Directory => "[+]",
            FileType::Device => "[#]",
            _ => " - ",
        };
        video::put_str_at(1, y, &format!("{} {:<16}", icon, entry), fg, bg);
    }
}

fn draw_buffer(buf: &[String], top: usize, cy: usize, cx: usize, off_x: usize, off_y: usize, w: usize, h: usize, cfg: &EditorConfig, active: bool) {
    for row in 0..(h - 1) {
        let idx = top + row;
        if idx < buf.len() {
            let line = &buf[idx];
            // Line Number
            if cfg.line_numbers {
                video::put_str_at(off_x + 1, off_y + 1 + row, &format!("{:3} ", idx + 1), 0x888888, COLOR_BG);
            }
            // Syntax Colorizer
            draw_colored_line(line, off_x + (if cfg.line_numbers { 6 } else { 2 }), off_y + 1 + row);
        }
    }
    // Cursor
    if active {
        let cursor_y = (cy - top) + off_y + 1;
        let cursor_x = if cfg.line_numbers { cx + 6 + off_x } else { cx + 2 + off_x };
        video::draw_rect_grid(cursor_x, cursor_y, 1, 1, 0, COLOR_HL);
    }
}

fn draw_colored_line(line: &str, x: usize, y: usize) {
    if line.is_empty() { return; }
    // Robust simple drawing to ensure visibility
    video::put_str_at(x, y, line, COLOR_FG, COLOR_BG);
    
    // Simple Keyword Over-highlight (safe approach)
    let keywords = ["fn", "let", "mut", "pub", "struct", "enum", "impl", "use", "void", "int", "char", "if", "else", "match", "return", "true", "false"];
    for kw in keywords.iter() {
        let mut start = 0;
        while let Some(pos) = line[start..].find(kw) {
            let actual_pos = start + pos;
            // Check word boundaries
            let before = if actual_pos > 0 { line.chars().nth(actual_pos - 1).unwrap_or(' ') } else { ' ' };
            let after = line.chars().nth(actual_pos + kw.len()).unwrap_or(' ');
            
            if !before.is_alphanumeric() && !after.is_alphanumeric() {
                video::put_str_at(x + actual_pos, y, kw, 0x55FFFF, COLOR_BG);
            }
            start = actual_pos + kw.len();
        }
    }
}

// --- INPUT HANDLERS ---

fn handle_explorer_key(c: char, exp: &mut FileExplorer, ws: &mut Workspace, mode: &mut Mode) {
    match c {
        keyboard::KEY_UP => if exp.selected_idx > 0 { exp.selected_idx -= 1; },
        keyboard::KEY_DOWN => if exp.selected_idx < exp.entries.len() - 1 { exp.selected_idx += 1; },
        '\n' | '\r' => {
            if let Some((entry, ftype)) = exp.entries.get(exp.selected_idx) {
                let mut full_path = if exp.current_path == "/" { format!("/{}", entry) } else { format!("{}/{}", exp.current_path.trim_end_matches('/'), entry) };
                if *ftype == FileType::Directory {
                    exp.current_path = full_path;
                    exp.refresh();
                    exp.selected_idx = 0;
                } else {
                    let content = load_file(&full_path);
                    ws.documents.push(Document::new(&full_path, content));
                    ws.active_idx = ws.documents.len() - 1;
                    *mode = Mode::Normal;
                    ws.output_history.push(format!("Opened: {}", full_path));
                }
            }
        },
        _ => {}
    }
}

fn handle_normal_key(c: char, mode: &mut Mode, doc: &mut Document, msg_box: &mut Option<String>) {
    match c {
        'i' | 'I' => *mode = Mode::Insert,
        keyboard::KEY_F2 => {
            if save_file(&doc.path, &doc.buffer).is_ok() {
                msg_box.replace(String::from("Document Saved"));
                doc.dirty = false;
            }
        },
        keyboard::KEY_UP => if doc.cy > 0 { doc.cy -= 1; doc.cx = min(doc.cx, doc.buffer[doc.cy].len()); },
        keyboard::KEY_DOWN => if doc.cy < doc.buffer.len() - 1 { doc.cy += 1; doc.cx = min(doc.cx, doc.buffer[doc.cy].len()); },
        keyboard::KEY_LEFT => if doc.cx > 0 { doc.cx -= 1; },
        keyboard::KEY_RIGHT => if doc.cx < doc.buffer[doc.cy].len() { doc.cx += 1; },
        _ => {}
    }
}

fn handle_insert_key(c: char, mode: &mut Mode, doc: &mut Document) {
    if c == '\x1B' { *mode = Mode::Normal; return; }
    doc.dirty = true;
    doc.last_change_tick = crate::process::scheduler::get_ticks();
    
    match c {
        '\n' | '\r' => {
            let rest = doc.buffer[doc.cy].split_off(doc.cx);
            doc.buffer.insert(doc.cy + 1, rest);
            doc.cy += 1; doc.cx = 0;
        },
        '\x08' => if doc.cx > 0 { doc.buffer[doc.cy].remove(doc.cx - 1); doc.cx -= 1; },
        _ => { doc.buffer[doc.cy].insert(doc.cx, c); doc.cx += 1; }
    }
}

fn handle_menu_key(c: char, mode: &mut Mode, idx: &mut usize, sub: &mut bool, ws: &mut Workspace, msg_box: &mut Option<String>) {
    match c {
        keyboard::KEY_LEFT => if *idx > 0 { *idx -= 1; } else { *idx = 7; },
        keyboard::KEY_RIGHT => if *idx < 7 { *idx += 1; } else { *idx = 0; },
        '\n' | '\r' => {
            if *sub {
                match (*idx, 0) {
                    (0, 2) => { // Save
                        if let Some(doc) = ws.current_doc_mut() {
                             let _ = save_file(&doc.path, &doc.buffer);
                             doc.dirty = false;
                        }
                    },
                    (7, 0) => ws.split_mode = Split::Vertical, // Options -> Split (Stub)
                    _ => {}
                }
                *sub = false; active_mode_to_normal(mode);
            } else { *sub = true; }
        },
        '\x1B' => { *sub = false; active_mode_to_normal(mode); },
        _ => {}
    }
}

fn active_mode_to_normal(mode: &mut Mode) {
    *mode = Mode::Normal;
}

// --- SUPPORTING FUNCTIONS ---

fn load_file(path: &str) -> Vec<String> {
    let mut buffer = Vec::new();
    if let Ok(inode) = crate::fs::vfs::resolve_path(path) {
        if let Ok(handle) = inode.open(0) {
             let mut data = alloc::vec![0u8; 65536]; // 64KB Max for IDE
             if let Ok(bytes) = handle.read(&mut data, 0) {
                 let s = core::str::from_utf8(&data[..bytes]).unwrap_or("");
                 for line in s.split('\n') {
                     buffer.push(String::from(line.trim_end_matches('\r')));
                 }
             }
        }
    }
    if buffer.is_empty() { buffer.push(String::new()); }
    buffer
}

fn save_file(path: &str, buffer: &[String]) -> Result<(), ()> {
    // 1. Resolve parent directory
    let (parent_path, filename) = if let Some(idx) = path.rfind('/') {
        if idx == 0 { ("/", &path[1..]) }
        else { (&path[..idx], &path[idx+1..]) }
    } else {
        ("/", path)
    };

    if let Ok(parent_inode) = crate::fs::vfs::resolve_path(parent_path) {
        let _ = parent_inode.create(filename, FileType::File);
        if let Ok(inode) = parent_inode.lookup(filename) {
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

fn draw_top_menu(selected: usize, active: bool) {
    let menus = [" File ", " Edit ", " Search ", " Run ", " Comp ", " Debug ", " Proj ", " Opts "];
    let mut x = 1;
    for (i, m) in menus.iter().enumerate() {
        let (fg, bg) = if active && i == selected { (COLOR_BAR_FG, COLOR_HL) } else { (COLOR_BAR_FG, COLOR_BAR_BG) };
        video::put_str_at(x, 0, m, fg, bg);
        // Mnemonic (approx)
        video::put_char_at(x + 1, 0, m.chars().nth(1).unwrap(), 0xAA0000, bg);
        x += m.len() + 1;
    }
}

fn draw_status_bar(mode: Mode, line: usize, col: usize, path: &str) {
    let mode_str = match mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
        Mode::Terminal => "TERMINAL",
        Mode::Explorer => "EXPLORER",
        Mode::Menu => "MENU",
        _ => "VIEW",
    };
    let status = format!(" [{}]  {}  L:{} C:{}  F1 Help  F2 Save  Alt+W Split  Alt+X Exit  Tab Focus", mode_str, path, line, col);
    video::put_str_at(0, *video::CONSOLE_HEIGHT.lock() - 1, &status, COLOR_BAR_FG, COLOR_BAR_BG);
}

fn draw_submenu(idx: usize) {
    let x = idx * 9 + 2;
    let items = vec![" New ", " Open ", " Save ", " Close ", " Quit "];
    unsafe { video::draw_tui_shadow(x as u32, 1, 10, items.len() as u32); }
    video::draw_rect_grid(x, 1, 10, items.len() + 2, COLOR_BAR_FG, COLOR_BAR_BG);
    for (i, item) in items.iter().enumerate() {
        video::put_str_at(x + 1, 2 + i, item, COLOR_BAR_FG, COLOR_BAR_BG);
    }
}

fn draw_popup(title: &str, msg: &str, w: usize, h: usize) {
    let bx = (w - 40) / 2;
    let by = (h - 8) / 2;
    unsafe { video::draw_tui_shadow(bx as u32, by as u32, 40, 8); }
    video::draw_tui_title_box(bx, by, 40, 8, title, COLOR_HL);
    video::put_str_at(bx + 2, by + 3, msg, COLOR_FG, COLOR_BG);
}

fn handle_terminal_key(c: char, mode: &mut Mode, buf: &mut String, history: &mut Vec<String>, ws: &mut Workspace) {
    if c == '\x1B' { *mode = Mode::Normal; return; }
    match c {
        '\n' | '\r' => if !buf.is_empty() {
            history.push(format!("ainux> {}", buf));
            ws.output_history.push(format!("Executing: {}", buf));
            crate::shell::execute_command(buf);
            buf.clear();
        },
        '\x08' => { buf.pop(); },
        _ => { buf.push(c); }
    }
}
