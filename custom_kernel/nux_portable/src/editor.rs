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

#[derive(Debug, PartialEq)]
enum Key {
    Char(char),
    Esc,
    Enter,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Ctrl(char), // For Ctrl+A, Ctrl+B, etc.
    Unknown(u8),
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
    escape_state: u8, // 0=None, 1=seen Esc, 2=seen Esc+[ or Esc+O
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
            escape_state: 0,
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

    fn read_key(&mut self) -> Key {
        let mut buf = [0; 1];
        if io::stdin().read_exact(&mut buf).is_err() { return Key::Unknown(0); }
        let b = buf[0];

        match self.escape_state {
            0 => {
                match b {
                    27 => { // ESC
                        // Try to read next byte non-blockingly (simulated)
                        // If we can't peek, we simply read.
                        // We support [ and O
                        
                        let mut next_buf = [0; 1];
                        if io::stdin().read_exact(&mut next_buf).is_ok() {
                            match next_buf[0] {
                                b'[' => {
                                    self.escape_state = 2; 
                                    return self.read_escape_sequence();
                                },
                                b'O' => {
                                    self.escape_state = 2; // Treat SS3 (ESC O) same as CSI (ESC [) for Arrows
                                    return self.read_escape_sequence();
                                },
                                _ => {
                                    // Unknown escape. Return Esc, lose the next char :(
                                    // Optimization: Push back? No simple way.
                                    return Key::Esc; 
                                }
                            }
                        } else {
                            return Key::Esc;
                        }
                    },
                    127 | 8 => Key::Backspace,
                    13 => Key::Enter,
                    c => {
                        if c < 32 { Key::Ctrl((c + 64) as char) } else { Key::Char(c as char) }
                    },
                }
            },
            _ => { self.escape_state = 0; Key::Unknown(b) }
        }
    }
    
    fn read_escape_sequence(&mut self) -> Key {
        let mut buf = [0; 1];
        if io::stdin().read_exact(&mut buf).is_err() {
            self.escape_state = 0;
            return Key::Unknown(0);
        }
        let b = buf[0];
        self.escape_state = 0;

        match b {
            b'A' => Key::Up,
            b'B' => Key::Down,
            b'C' => Key::Right,
            b'D' => Key::Left,
            b'H' => Key::Home, 
            b'F' => Key::End,   
            // Handle `3~` etc
             b'1'..=b'6' => {
                let mut next_buf = [0; 1];
                if io::stdin().read_exact(&mut next_buf).is_ok() {
                     if next_buf[0] == b'~' {
                        match b {
                            b'1' => Key::Home,
                            b'3' => Key::Delete,
                            b'4' => Key::End,
                            b'5' => Key::PageUp,
                            b'6' => Key::PageDown,
                            _ => Key::Unknown(b),
                        }
                     } else { Key::Unknown(b) }
                } else { Key::Unknown(b) }
            },
            _ => Key::Unknown(b),
        }
    }

    fn process_keypress(&mut self) {
        let key = self.read_key();
        
        // Correct boundary check
        if !self.lines.is_empty() {
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
