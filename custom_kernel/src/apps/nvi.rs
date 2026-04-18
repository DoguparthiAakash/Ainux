use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::{min, max};
use crate::drivers::video;
use crate::drivers::keyboard;
use crate::fs::vfs::{ROOT, FileType, FileHandle};

#[derive(PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

pub fn cmd_nvi(args: &[&str]) {
    if args.len() < 2 {
        video::put_str("Usage: nvi <filename>\n");
        return;
    }
    let filename = args[1];
    
    // Attempt to load existing file content
    let mut buffer: Vec<String> = Vec::new();
    let root = ROOT.lock();
    if let Some(r) = root.as_ref() {
        if let Ok(inode) = r.lookup(filename) {
            if let Ok(handle) = inode.open(0) {
                 let mut data = alloc::vec![0u8; 8192];
                 if let Ok(bytes) = handle.read(&mut data, 0) {
                     if bytes > 0 {
                         let s = core::str::from_utf8(&data[..bytes]).unwrap_or("");
                         for line in s.split('\n') {
                             buffer.push(String::from(line.trim_end_matches('\r')));
                         }
                     }
                 }
            }
        }
    }
    core::mem::drop(root);
    
    if buffer.is_empty() {
        buffer.push(String::new());
    }

    let mut mode = Mode::Normal;
    let mut cx: usize = 0;
    let mut cy: usize = 0;
    let mut cmd_buf = String::new();
    
    let w = *video::CONSOLE_WIDTH.lock();
    let h = *video::CONSOLE_HEIGHT.lock();
    let mut scroll_top = 0;

    loop {
        if cy < scroll_top { scroll_top = cy; }
        if cy >= scroll_top + h - 1 { scroll_top = cy - (h - 2); }

        video::clear();
        
        for row in 0..(h - 1) {
            let buf_i = scroll_top + row;
            unsafe { *video::CONSOLE_X.lock() = 0; *video::CONSOLE_Y.lock() = row; }
            if buf_i < buffer.len() {
                video::put_str(&buffer[buf_i]);
            } else if buf_i > buffer.len() {
                video::put_char('~');
            }
        }
        
        unsafe { *video::CONSOLE_X.lock() = 0; *video::CONSOLE_Y.lock() = h - 1; }
        let mode_str = match mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
        };
        video::put_str(&format!("-- {} -- {} [{}/{}]", mode_str, filename, cy + 1, buffer.len()));
        if mode == Mode::Command {
            video::put_str(&format!(" :{}", cmd_buf));
        }
        
        unsafe {
            let cursor_drawn_y = if mode == Mode::Command { h - 1 } else { cy - scroll_top };
            let cursor_drawn_x = if mode == Mode::Command { 
                let base_len = format!("-- {} -- {} [{}/{}] :", mode_str, filename, cy + 1, buffer.len()).len();
                base_len + cmd_buf.len()
            } else { cx };
            
            let px = (cursor_drawn_x * 8) as i64;
            let py = (cursor_drawn_y * 12) as i64;
            video::draw_rect(px, py, 8, 12, 0xFFFFFF); 
        }
        
        let mut key = None;
        while key.is_none() {
            key = keyboard::pop_char();
            if key.is_none() { unsafe { core::arch::asm!("hlt"); } }
        }
        let c = key.unwrap();
        
        match mode {
            Mode::Normal => {
                match c {
                    'i' => mode = Mode::Insert,
                    ':' => { mode = Mode::Command; cmd_buf.clear(); },
                    'h' | '\u{2190}' => if cx > 0 { cx -= 1; },
                    'l' | '\u{2192}' => if cx < buffer[cy].len() { cx += 1; },
                    'k' | '\u{2191}' => if cy > 0 { cy -= 1; cx = min(cx, buffer[cy].len()); },
                    'j' | '\u{2193}' => if cy < buffer.len() - 1 { cy += 1; cx = min(cx, buffer[cy].len()); },
                    '$' => cx = buffer[cy].len(),
                    '0' => cx = 0,
                    'x' => {
                        if cx < buffer[cy].len() {
                            buffer[cy].remove(cx);
                        }
                    },
                    _ => {}
                }
            },
            Mode::Insert => {
                if c == '\x1B' {
                    mode = Mode::Normal;
                    if cx > 0 && cx == buffer[cy].len() { cx -= 1; }
                }
                else if c == '\u{2191}' { if cy > 0 { cy -= 1; cx = min(cx, buffer[cy].len()); } }
                else if c == '\u{2193}' { if cy < buffer.len() - 1 { cy += 1; cx = min(cx, buffer[cy].len()); } }
                else if c == '\u{2190}' { if cx > 0 { cx -= 1; } }
                else if c == '\u{2192}' { if cx < buffer.len() && cx < buffer[cy].len() { cx += 1; } }
                else if c == '\n' || c == '\r' {
                    let rest = buffer[cy].split_off(cx);
                    buffer.insert(cy + 1, rest);
                    cy += 1;
                    cx = 0;
                }
                else if c == '\x08' || c == '\x7F' {
                    if cx > 0 {
                        buffer[cy].remove(cx - 1);
                        cx -= 1;
                    } else if cy > 0 {
                        let rest = buffer.remove(cy);
                        cy -= 1;
                        cx = buffer[cy].len();
                        buffer[cy].push_str(&rest);
                    }
                }
                else {
                    buffer[cy].insert(cx, c);
                    cx += 1;
                }
            },
            Mode::Command => {
                if c == '\n' || c == '\r' {
                    if cmd_buf == "w" || cmd_buf == "wq" {
                        let root = ROOT.lock();
                        if let Some(r) = root.as_ref() {
                            let _ = r.create(filename, FileType::File);
                            if let Ok(inode) = r.lookup(filename) {
                                if let Ok(handle) = inode.open(0) {
                                    let mut fs_offset = 0;
                                    for (i, line) in buffer.iter().enumerate() {
                                        let bytes = line.as_bytes();
                                        let _ = handle.write(bytes, fs_offset);
                                        fs_offset += bytes.len() as u64;
                                        if i < buffer.len() - 1 {
                                            let _ = handle.write(b"\n", fs_offset);
                                            fs_offset += 1;
                                        }
                                    }
                                }
                            }
                        }
                        if cmd_buf == "wq" {
                            video::clear();
                            return;
                        }
                    } else if cmd_buf == "q" || cmd_buf == "q!" {
                        video::clear();
                        return;
                    }
                    mode = Mode::Normal;
                } else if c == '\x08' || c == '\x7F' {
                    if cmd_buf.is_empty() { mode = Mode::Normal; }
                    else { cmd_buf.pop(); }
                } else if c == '\x1B' {
                    mode = Mode::Normal;
                } else {
                    cmd_buf.push(c);
                }
            }
        }
    }
}
