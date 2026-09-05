extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::cmp::{min, max};
use crate::drivers::{video, keyboard, mouse};
use crate::fs::vfs::{ROOT, FileType};
use crate::shell;

// Stable Nvix - Neovim-style usage
// Optimized to prevent flickering by only redrawing on change.
// BG and FG are derived from video::THEME at render time — adapts to user theme changes.
// Mode indicator colors are intentionally fixed (semantic meaning, not style).

const COLOR_MODE_NORMAL: u32 = 0x005E81AC; // Blue  — Normal
const COLOR_MODE_INSERT: u32 = 0x00A3BE8C; // Green — Insert
const COLOR_MODE_CMD:    u32 = 0x00EBCB8B; // Amber — Command

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

struct Document {
    path: String,
    buffer: Vec<String>,
    cx: usize,
    cy: usize,
    scroll_top: usize,
    dirty: bool,
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
        }
    }
}

pub fn cmd_nvix(args: &[&str]) {
    let raw_path = if args.len() > 1 { args[1] } else { "new_file.c" };
    let target_path = shell::resolve_path(raw_path);
    let content = load_file(&target_path);
    let mut doc = Document::new(&target_path, content);
    
    let mut mode = Mode::Normal;
    let mut command_buffer = String::new();
    let mut msg_box: Option<String> = None;
    let mut should_quit = false;
    
    let (console_w, console_h) = {
        let w = *video::CONSOLE_WIDTH.lock();
        let h = *video::CONSOLE_HEIGHT.lock();
        (w, h)
    };
    
    let main_h = console_h - 2;

    // Initial Full Render
    render_all(&doc, mode, &command_buffer, &msg_box, console_w, console_h);

    while !should_quit {
        let mut needs_render = false;
        let current_ticks = crate::process::scheduler::get_ticks();
        
        // --- CURSOR BLINK (Localized Update) ---
        static mut LAST_BLINK: u64 = 0;
        static mut CURSOR_STATE: bool = false;
        unsafe {
            if current_ticks > LAST_BLINK + 40 {
                LAST_BLINK = current_ticks;
                CURSOR_STATE = !CURSOR_STATE;
                draw_cursor_only(&doc, mode, CURSOR_STATE, main_h);
            }
        }

        // --- INPUT PHASE ---
        if let Some(c) = keyboard::pop_char() {
            needs_render = true;
            if c == '\x1B' { // ESC
                mode = Mode::Normal;
                command_buffer.clear();
                msg_box = None;
            } else {
                match mode {
                    Mode::Normal => handle_normal_input(c, &mut mode, &mut doc, &mut command_buffer),
                    Mode::Insert => handle_insert_input(c, &mut mode, &mut doc),
                    Mode::Command => {
                        if handle_command_input(c, &mut mode, &mut command_buffer, &mut doc, &mut msg_box) {
                            should_quit = true;
                        }
                    }
                }
            }
        }

        // --- MOUSE (Quick Check) ---
        if let Some(_) = mouse::pop_event() {
            // Optional: Handle mouse scrolling
        }

        // --- RENDERING PHASE (Conditional) ---
        if needs_render {
            // Adjust scroll before render
            if doc.cy < doc.scroll_top { doc.scroll_top = doc.cy; }
            if doc.cy >= doc.scroll_top + main_h { doc.scroll_top = doc.cy - (main_h - 1); }
            
            render_all(&doc, mode, &command_buffer, &msg_box, console_w, console_h);
        }

        unsafe { core::arch::asm!("hlt"); }
    }

    // Restore shell screen on exit: fill with theme background, not black
    let theme_bg = video::THEME.lock().bg;
    video::clear(); // fills entire framebuffer with theme bg
    // Reset cursor to top-left so shell prompt redraws from the top
    *video::CONSOLE_X.lock() = 0;
    *video::CONSOLE_Y.lock() = 0;
}

fn render_all(doc: &Document, mode: Mode, cmd_buf: &str, msg: &Option<String>, w: usize, h: usize) {
    // Read theme once per render — always reflects the current user theme
    let (c_bg, c_fg, c_status_bg) = {
        let t = video::THEME.lock();
        let bg   = t.bg;
        let fg   = t.fg;
        let s_bg = crate::apps::dcustom::blend_dark_pub(bg, 0x08);
        (bg, fg, s_bg)
    };
    let c_status_fg = 0x00FFFFFF;
    let c_line_num  = crate::apps::dcustom::blend_dark_pub(c_fg, 0x60);

    let main_h = h - 2;
    unsafe {
        video::fast_grid_clear(w as u32, main_h as u32, c_fg, c_bg, ' ' as u32);

        // 1. Draw Text Buffer
        for row in 0..main_h {
            let idx = doc.scroll_top + row;
            if idx < doc.buffer.len() {
                let line = &doc.buffer[idx];
                let num_str = if idx == doc.cy {
                    format!(" {:2} ", idx + 1)
                } else {
                    format!(" {:2} ", (idx as isize - doc.cy as isize).abs())
                };
                video::put_str_at(0, row, &num_str, c_line_num, c_bg);
                video::put_str_at(4, row, line, c_fg, c_bg);
            }
        }

        // 2. Status Line
        video::draw_rect_grid(0, h - 2, w, 1, c_status_fg, c_status_bg);
        let mode_str = match mode {
            Mode::Normal  => " NORMAL ",
            Mode::Insert  => " INSERT ",
            Mode::Command => " COMMAND ",
        };
        let mode_col = match mode {
            Mode::Normal  => COLOR_MODE_NORMAL,
            Mode::Insert  => COLOR_MODE_INSERT,
            Mode::Command => COLOR_MODE_CMD,
        };
        video::put_str_at(0, h - 2, mode_str, 0, mode_col);
        video::put_str_at(
            mode_str.len() + 1, h - 2,
            &format!(" {} {}", doc.path, if doc.dirty { "[+]" } else { "" }),
            c_status_fg, c_status_bg,
        );
        let pos_str = format!(" {}:{} ", doc.cy + 1, doc.cx + 1);
        video::put_str_at(w - pos_str.len(), h - 2, &pos_str, 0, mode_col);

        // 3. Command Line
        video::draw_rect_grid(0, h - 1, w, 1, c_fg, c_bg);
        if let Some(m) = msg {
            video::put_str_at(0, h - 1, m, c_fg, c_bg);
        } else if mode == Mode::Command {
            video::put_str_at(0, h - 1, &format!(":{}", cmd_buf), c_fg, c_bg);
        }
    }
}

fn draw_cursor_only(doc: &Document, mode: Mode, visible: bool, main_h: usize) {
    if doc.cy < doc.scroll_top || doc.cy >= doc.scroll_top + main_h { return; }
    let y = doc.cy - doc.scroll_top;
    let x = 4 + doc.cx;

    let (c_bg, c_fg) = {
        let t = video::THEME.lock();
        (t.bg, t.fg)
    };

    let col = if visible {
        match mode {
            Mode::Normal  => COLOR_MODE_NORMAL,
            Mode::Insert  => COLOR_MODE_INSERT,
            Mode::Command => COLOR_MODE_CMD,
        }
    } else {
        c_bg
    };

    unsafe {
        video::draw_rect_grid(x, y, 1, 1, 0, col);
        if !visible {
            // Redraw the character that was under the cursor
            if let Some(line) = doc.buffer.get(doc.cy) {
                if let Some(c) = line.chars().nth(doc.cx) {
                    video::put_char_at(x, y, c, c_fg, c_bg);
                }
            }
        }
    }
}

// --- INPUT HANDLERS ---

fn handle_normal_input(c: char, mode: &mut Mode, doc: &mut Document, cmd_buf: &mut String) {
    match c {
        'h' => if doc.cx > 0 { doc.cx -= 1; },
        'l' => if doc.cx < doc.buffer[doc.cy].len() { doc.cx += 1; },
        'j' => if doc.cy < doc.buffer.len() - 1 { doc.cy += 1; doc.cx = min(doc.cx, doc.buffer[doc.cy].len()); },
        'k' => if doc.cy > 0 { doc.cy -= 1; doc.cx = min(doc.cx, doc.buffer[doc.cy].len()); },
        'i' => *mode = Mode::Insert,
        'a' => { if doc.cx < doc.buffer[doc.cy].len() { doc.cx += 1; } *mode = Mode::Insert; },
        'o' => {
            doc.buffer.insert(doc.cy + 1, String::new());
            doc.cy += 1; doc.cx = 0;
            *mode = Mode::Insert;
        },
        'x' => if doc.cx < doc.buffer[doc.cy].len() { doc.buffer[doc.cy].remove(doc.cx); doc.dirty = true; },
        ':' => { *mode = Mode::Command; cmd_buf.clear(); },
        _ => {}
    }
}

fn handle_insert_input(c: char, mode: &mut Mode, doc: &mut Document) {
    doc.dirty = true;
    match c {
        '\n' | '\r' => {
            let rest = doc.buffer[doc.cy].split_off(doc.cx);
            doc.buffer.insert(doc.cy + 1, rest);
            doc.cy += 1; doc.cx = 0;
        },
        '\x08' => { // Backspace
            if doc.cx > 0 {
                doc.buffer[doc.cy].remove(doc.cx - 1);
                doc.cx -= 1;
            } else if doc.cy > 0 {
                let cur = doc.buffer.remove(doc.cy);
                doc.cy -= 1;
                doc.cx = doc.buffer[doc.cy].len();
                doc.buffer[doc.cy].push_str(&cur);
            }
        },
        keyboard::KEY_UP => if doc.cy > 0 { doc.cy -= 1; doc.cx = min(doc.cx, doc.buffer[doc.cy].len()); },
        keyboard::KEY_DOWN => if doc.cy < doc.buffer.len() - 1 { doc.cy += 1; doc.cx = min(doc.cx, doc.buffer[doc.cy].len()); },
        keyboard::KEY_LEFT => if doc.cx > 0 { doc.cx -= 1; },
        keyboard::KEY_RIGHT => if doc.cx < doc.buffer[doc.cy].len() { doc.cx += 1; },
        _ => {
            if !c.is_control() && c < '\u{E000}' && (c < '\u{2190}' || c > '\u{2198}') {
                doc.buffer[doc.cy].insert(doc.cx, c);
                doc.cx += 1;
            }
        }
    }
}

fn handle_command_input(c: char, mode: &mut Mode, buf: &mut String, doc: &mut Document, msg: &mut Option<String>) -> bool {
    match c {
        '\n' | '\r' => {
            let cmd = buf.clone();
            *mode = Mode::Normal;
            if cmd == "q" { return true; }
            if cmd == "w" {
                if save_file(&doc.path, &doc.buffer).is_ok() {
                    msg.replace(format!("\"{}\" saved", doc.path));
                    doc.dirty = false;
                }
            }
            if cmd == "wq" || cmd == "x" {
                let _ = save_file(&doc.path, &doc.buffer);
                return true;
            }
        },
        '\x08' => { buf.pop(); },
        _ => { buf.push(c); }
    }
    false
}

// --- UTILS ---

fn load_file(path: &str) -> Vec<String> {
    let mut buffer = Vec::new();
    if let Ok(inode) = crate::fs::vfs::resolve_path(path) {
        if let Ok(handle) = inode.open(0) {
             let mut data = alloc::vec![0u8; 65536];
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
    let (parent, name) = if let Some(i) = path.rfind('/') {
        if i == 0 { ("/", &path[1..]) } else { (&path[..i], &path[i+1..]) }
    } else { ("/", path) };

    if let Ok(p_node) = crate::fs::vfs::resolve_path(parent) {
        let _ = p_node.create(name, FileType::File);
        if let Ok(inode) = p_node.lookup(name) {
            if let Ok(h) = inode.open(0) {
                let mut off = 0;
                for (i, line) in buffer.iter().enumerate() {
                    let b = line.as_bytes();
                    let _ = h.write(b, off);
                    off += b.len() as u64;
                    if i < buffer.len() - 1 { let _ = h.write(b"\n", off); off += 1; }
                }
                return Ok(());
            }
        }
    }
    Err(())
}
