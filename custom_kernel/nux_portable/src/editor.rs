use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::Command;

pub fn run(filename: &str) {
    let editors = ["vim", "vi", "nano"];
    for editor in editors.iter() {
        if is_executable_in_path(editor) {
            let status = Command::new(editor).arg(filename).status();
            if let Ok(s) = status {
                if s.success() { return; }
            }
        }
    }

    println!("System editor not found. Launching Nux-Vim...");
    let mut editor = MiniVim::new(filename);
    editor.run();
}

fn is_executable_in_path(cmd: &str) -> bool {
    Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false)
}

// --- Mini Vim Implementation ---
#[derive(PartialEq)]
enum Mode {
    Normal,
    Insert,
    Command,
    ConfirmQuit,
}

struct MiniVim {
    filename: String,
    lines: Vec<String>,
    cx: usize,
    cy: usize,
    mode: Mode,
    command_buffer: String,
    msg: String,
    quit: bool,
    dirty: bool,
    esc_seq: usize, // 0=None, 1=seen Esc, 2=seen Esc+[
}

impl MiniVim {
    fn new(filename: &str) -> Self {
        let content = fs::read_to_string(filename).unwrap_or_default();
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|s| s.to_string()).collect()
        };

        Self {
            filename: filename.to_string(),
            lines,
            cx: 0,
            cy: 0,
            mode: Mode::Normal,
            command_buffer: String::new(),
            msg: String::from("HELP: i=Insert, Esc=Normal, :w=Save, :q=Quit"),
            quit: false,
            dirty: false,
            esc_seq: 0,
        }
    }

    fn run(&mut self) {
        self.enable_raw_mode();
        loop {
            self.refresh_screen();
            if self.quit { break; }
            self.process_keypress();
        }
        self.disable_raw_mode();
        println!("Bye!");
    }

    fn enable_raw_mode(&self) {
        let _ = Command::new("stty").arg("raw").arg("-echo").status();
    }

    fn disable_raw_mode(&self) {
        let _ = Command::new("stty").arg("-raw").arg("echo").status();
    }

    fn refresh_screen(&self) {
        print!("\x1b[2J\x1b[H"); 
        for (i, line) in self.lines.iter().enumerate() {
            print!("{}\r\n", line);
        }
        
        print!("\x1b[H\x1b[999B"); 
        let dirty_char = if self.dirty { "[+]" } else { "" };
        let status_text = match self.mode { 
            Mode::Normal => "NORMAL", 
            Mode::Insert => "INSERT", 
            Mode::Command => "COMMAND",
            Mode::ConfirmQuit => "CONFIRM",
        };

        print!("\n-- {} -- {} Pos: {},{}  {}\r\n", status_text, dirty_char, self.cx, self.cy, self.msg);
            
        if matches!(self.mode, Mode::Command) {
             print!(":{}", self.command_buffer);
        } else if matches!(self.mode, Mode::ConfirmQuit) {
             print!("Unsaved changes! Quit anyway? (y/n) ");
        }

        if !matches!(self.mode, Mode::ConfirmQuit) {
            print!("\x1b[{};{}H", self.cy + 1, self.cx + 1);
        }
        io::stdout().flush().unwrap();
    }

    fn process_keypress(&mut self) {
        let b = self.read_byte();
        
        // Escape Sequence State Machine
        if self.esc_seq == 1 {
            if b == 91 { // '['
                self.esc_seq = 2; return;
            } else {
                self.esc_seq = 0; // Cancel logic, treat as Normal Esc + new char
                self.mode = Mode::Normal; // Enforce normal mode if we saw Esc
                self.handle_standard_key(b);
                return;
            }
        } else if self.esc_seq == 2 {
            self.esc_seq = 0;
            match b {
                65 => { if self.cy > 0 { self.cy -= 1; } }, // Up
                66 => { if self.cy < self.lines.len().saturating_sub(1) { self.cy += 1; } }, // Down
                67 => { // Right
                     let len = if self.cy < self.lines.len() { self.lines[self.cy].len() } else { 0 };
                     if self.cx < len { self.cx += 1; }
                },
                68 => { if self.cx > 0 { self.cx -= 1; } }, // Left
                _ => {}
            }
            return;
        }

        if b == 27 {
            self.esc_seq = 1;
            // Hack for non-blocking: We assume if it's Esc, we switch to Normal mode anyway?
            // If we are in Insert Mode, Esc should switch to Normal.
            // If this is start of Arrow, we will see [ next.
            // Problem: If user types Esc then waits, we are in state 1. 
            // Editor might look unresponsive until next key. 
            // This is acceptable constraint for now.
            if matches!(self.mode, Mode::Insert) || matches!(self.mode, Mode::Command) {
                 self.mode = Mode::Normal;
            }
            return;
        }
        
        self.handle_standard_key(b);
    }
    
    fn handle_standard_key(&mut self, b: u8) {
        match self.mode {
            Mode::Normal => match b {
                b'i' => self.mode = Mode::Insert,
                b':' => {
                    self.mode = Mode::Command;
                    self.command_buffer.clear();
                },
                b'h' => if self.cx > 0 { self.cx -= 1 },
                b'j' => if self.cy < self.lines.len() - 1 { self.cy += 1 },
                b'k' => if self.cy > 0 { self.cy -= 1 },
                b'l' => {
                     let len = if self.cy < self.lines.len() { self.lines[self.cy].len() } else { 0 };
                     if self.cx < len { self.cx += 1 }
                },
                b'x' => { self.delete_char(); self.dirty = true; },
                _ => {},
            },
            Mode::Insert => match b {
                13 => { self.insert_newline(); self.dirty = true; },
                127 | 8 => { self.backspace(); self.dirty = true; },
                c => { self.insert_char(c as char); self.dirty = true; },
            },
            Mode::Command => match b {
                13 => self.execute_command(),
                127 | 8 => { self.command_buffer.pop(); },
                c => self.command_buffer.push(c as char),
            },
            Mode::ConfirmQuit => match b {
                b'y' | b'Y' => self.quit = true,
                _ => { self.mode = Mode::Normal; self.msg = String::from("Quit cancelled."); },
            }
        }
        
        // Boundary Check
        if !self.lines.is_empty() {
            if self.cy >= self.lines.len() { self.cy = self.lines.len() - 1; }
            let row_len = self.lines[self.cy].len();
            if self.cx > row_len { self.cx = row_len; }
        }
    }
    
    fn read_byte(&self) -> u8 {
        let mut buf = [0; 1];
        io::stdin().read_exact(&mut buf).unwrap();
        buf[0]
    }
    
    fn insert_char(&mut self, c: char) {
        if self.cy >= self.lines.len() { self.lines.push(String::new()); }
        let line = &mut self.lines[self.cy];
        if self.cx >= line.len() {
            line.push(c);
        } else {
            line.insert(self.cx, c);
        }
        self.cx += 1;
    }
    
    fn insert_newline(&mut self) {
         if self.cy >= self.lines.len() { self.lines.push(String::new()); return; }
         let current = &mut self.lines[self.cy];
         let rest = if self.cx < current.len() { current.split_off(self.cx) } else { String::new() };
         self.lines.insert(self.cy + 1, rest);
         self.cy += 1;
         self.cx = 0;
    }
    
    fn backspace(&mut self) {
        if self.cx > 0 {
             let line = &mut self.lines[self.cy];
             if self.cx <= line.len() { line.remove(self.cx - 1); self.cx -= 1; }
        } else if self.cy > 0 {
             let curr = self.lines.remove(self.cy);
             self.cy -= 1;
             self.cx = self.lines[self.cy].len();
             self.lines[self.cy].push_str(&curr);
        }
    }
    
    fn delete_char(&mut self) {
        if self.cy < self.lines.len() {
            let line = &mut self.lines[self.cy];
            if self.cx < line.len() { line.remove(self.cx); }
        }
    }
    
    fn execute_command(&mut self) {
        match self.command_buffer.as_str() {
            "w" => self.save_file(),
            "q" => {
                if self.dirty {
                    self.mode = Mode::ConfirmQuit;
                    self.msg = String::new();
                    return; 
                }
                self.quit = true; 
            },
            "q!" => self.quit = true,
            "wq" => { self.save_file(); self.quit = true; },
            _ => self.msg = format!("Unknown command: {}", self.command_buffer),
        }
        if !matches!(self.mode, Mode::ConfirmQuit) { self.mode = Mode::Normal; }
        self.command_buffer.clear();
    }
    
    fn save_file(&mut self) {
        let content = self.lines.join("\n");
        if let Err(e) = fs::write(&self.filename, content) {
            self.msg = format!("Error saving: {}", e);
        } else {
            self.msg = "File saved.".to_string();
            self.dirty = false;
        }
    }
}
