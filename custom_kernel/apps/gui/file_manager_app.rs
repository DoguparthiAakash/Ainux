use crate::gui::app::App;
use crate::drivers::keyboard;
use crate::fs::vfs;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone)]
struct FileEntry {
    name: String,
    is_dir: bool,
    size: u64,
}

pub struct FileManagerApp {
    current_path: String,
    entries: Vec<FileEntry>,
    selected: usize,
    scroll: usize,
    status: String,
    needs_redraw: bool,
    last_click_entry: Option<usize>,
    last_click_tick: u64,
}

impl FileManagerApp {
    pub fn new() -> Self {
        let mut app = Self {
            current_path: String::from("/"),
            entries: Vec::new(),
            selected: 0,
            scroll: 0,
            status: String::from("Ready"),
            needs_redraw: true,
            last_click_entry: None,
            last_click_tick: 0,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.entries.clear();
        
        // Add parent dir entry unless we are root
        if self.current_path != "/" {
            self.entries.push(FileEntry { name: String::from(".."), is_dir: true, size: 0 });
        }

        match vfs::resolve_path(&self.current_path) {
            Ok(inode) => {
                match inode.read_dir() {
                    Ok(names) => {
                        for name in names {
                            if name == "." || name == ".." { continue; }
                            let child_path = if self.current_path == "/" {
                                alloc::format!("/{}", name)
                            } else {
                                alloc::format!("{}/{}", self.current_path, name)
                            };
                            let (is_dir, size) = if let Ok(child) = vfs::resolve_path(&child_path) {
                                if let Ok(st) = child.stat() {
                                    (st.file_type == vfs::FileType::Directory, st.size)
                                } else { (false, 0) }
                            } else { (false, 0) };
                            self.entries.push(FileEntry { name, is_dir, size });
                        }
                        // Sort: dirs first, then files
                        self.entries.sort_by(|a, b| {
                            if a.name == ".." { return core::cmp::Ordering::Less; }
                            if b.name == ".." { return core::cmp::Ordering::Greater; }
                            b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
                        });
                    }
                    Err(_) => { self.status = String::from("Cannot read directory"); }
                }
            }
            Err(_) => { self.status = alloc::format!("Path not found: {}", self.current_path); }
        }

        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
        self.needs_redraw = true;
    }

    fn navigate_into(&mut self) {
        if self.entries.is_empty() { return; }
        let entry = self.entries[self.selected].clone();
        if entry.is_dir {
            if entry.name == ".." {
                // Go up
                if let Some(pos) = self.current_path.rfind('/') {
                    if pos == 0 {
                        self.current_path = String::from("/");
                    } else {
                        self.current_path.truncate(pos);
                    }
                }
            } else {
                if self.current_path == "/" {
                    self.current_path = alloc::format!("/{}", entry.name);
                } else {
                    self.current_path = alloc::format!("{}/{}", self.current_path, entry.name);
                }
            }
            self.selected = 0;
            self.scroll = 0;
            self.refresh();
        } else {
            self.status = alloc::format!("File: {} ({} bytes)", entry.name, entry.size);
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

    fn draw_text(buf: &mut [u32], bw: usize, bh: usize, x: i32, y: i32, text: &str, fg: u32) {
        crate::drivers::video::draw_text_to_buffer(buf, bw as i64, bh as i64, x as i64, y as i64, text, fg);
    }
}

impl App for FileManagerApp {
    fn update(&mut self) {}

    fn draw(&mut self, buf: &mut [u32], w: usize, h: usize) {
        if !self.needs_redraw { return; }

        // Background
        Self::fill(buf, w, h, 0, 0, w as i32, h as i32, 0xFF1E1E2E);

        // Toolbar / address bar
        Self::fill(buf, w, h, 0, 0, w as i32, 24, 0xFF313244);
        Self::draw_text(buf, w, h, 4, 4, "📁", 0xFFCBA6F7);
        Self::draw_text(buf, w, h, 24, 4, &self.current_path, 0xFFCDD6F4);

        // Column headers
        let header_y = 26;
        Self::fill(buf, w, h, 0, header_y, w as i32, 18, 0xFF45475A);
        Self::draw_text(buf, w, h, 4, header_y + 2, "Name", 0xFFBAC2E8);
        Self::draw_text(buf, w, h, w as i32 - 100, header_y + 2, "Size", 0xFFBAC2E8);
        Self::draw_text(buf, w, h, w as i32 - 55, header_y + 2, "Type", 0xFFBAC2E8);

        // File list
        let list_y = 46;
        let row_h = 18;
        let visible = ((h as i32 - list_y - 24) / row_h).max(0) as usize;

        for (i, entry) in self.entries.iter().enumerate().skip(self.scroll) {
            if i - self.scroll >= visible { break; }
            let ry = list_y + ((i - self.scroll) as i32 * row_h);

            let bg = if i == self.selected { 0xFF585B70 } else if (i - self.scroll) % 2 == 0 { 0xFF1E1E2E } else { 0xFF24273A };
            Self::fill(buf, w, h, 0, ry, w as i32, row_h, bg);

            let icon = if entry.is_dir { "D " } else { "F " };
            let icon_color = if entry.is_dir { 0xFF89DCEB } else { 0xFFA6E3A1 };
            Self::draw_text(buf, w, h, 4, ry + 2, icon, icon_color);

            // Truncate name
            let name_disp: String = entry.name.chars().take(28).collect();
            Self::draw_text(buf, w, h, 22, ry + 2, &name_disp, 0xFFCDD6F4);

            if !entry.is_dir && entry.size > 0 {
                let size_str = if entry.size >= 1048576 {
                    alloc::format!("{:.1}M", entry.size as f64 / 1048576.0)
                } else if entry.size >= 1024 {
                    alloc::format!("{}K", entry.size / 1024)
                } else {
                    alloc::format!("{}B", entry.size)
                };
                Self::draw_text(buf, w, h, w as i32 - 100, ry + 2, &size_str, 0xFFF5E0DC);
            }

            let type_str = if entry.is_dir { "DIR" } else {
                if entry.name.ends_with(".elf") { "ELF" }
                else if entry.name.ends_with(".rs") { "RS" }
                else if entry.name.ends_with(".txt") { "TXT" }
                else { "FILE" }
            };
            Self::draw_text(buf, w, h, w as i32 - 55, ry + 2, type_str, 0xFFCBA6F7);
        }

        // Status bar
        Self::fill(buf, w, h, 0, h as i32 - 20, w as i32, 20, 0xFF313244);
        let status = alloc::format!("{} items  |  {}", self.entries.len(), self.status);
        Self::draw_text(buf, w, h, 4, h as i32 - 17, &status, 0xFF6C7086);

        self.needs_redraw = false;
    }

    fn on_mouse_event(&mut self, _x: i32, y: i32, buttons: u8) {
        if buttons & 1 != 0 {
            let list_y = 46;
            let row_h = 18;
            if y >= list_y {
                let idx = ((y - list_y) / row_h) as usize + self.scroll;
                if idx < self.entries.len() {
                    let tick = crate::process::scheduler::get_ticks();
                    // Double-click detection: same entry within 50 ticks
                    if self.last_click_entry == Some(idx) && tick.saturating_sub(self.last_click_tick) < 50 {
                        self.navigate_into();
                    } else {
                        self.selected = idx;
                        self.last_click_entry = Some(idx);
                        self.last_click_tick = tick;
                        self.needs_redraw = true;
                    }
                }
            }
        }
    }

    fn on_key_event(&mut self, c: char) {
        match c {
            keyboard::KEY_UP => {
                if self.selected > 0 { self.selected -= 1; }
                if self.selected < self.scroll { self.scroll = self.selected; }
                self.needs_redraw = true;
            }
            keyboard::KEY_DOWN => {
                if self.selected + 1 < self.entries.len() { self.selected += 1; }
                self.needs_redraw = true;
            }
            '\r' | '\n' => { self.navigate_into(); }
            '\x08' => { // Backspace = go up
                self.selected = 0;
                if self.entries.first().map(|e| e.name.as_str()) == Some("..") {
                    self.navigate_into();
                }
            }
            'r' | 'R' => { self.refresh(); }
            _ => {}
        }
    }
}
