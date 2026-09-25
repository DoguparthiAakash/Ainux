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

pub fn file_manager_main() {
    let id = 2; // Fixed ID for file manager, or passed via parameter? Let's just use 22
    let width = 400;
    let height = 300;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id: 22,
        title: alloc::string::String::from("MithlFS Explorer"),
        x: 150,
        y: 150,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFF1E1E2E; width * height];
    let mut app = FileManagerApp::new();
    
    loop {
        for event in crate::gui::wm::pop_events(22) {
            match event {
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    match c {
                        keyboard::KEY_UP => {
                            if app.selected > 0 { app.selected -= 1; }
                            if app.selected < app.scroll { app.scroll = app.selected; }
                            app.needs_redraw = true;
                        }
                        keyboard::KEY_DOWN => {
                            if app.selected + 1 < app.entries.len() { app.selected += 1; }
                            app.needs_redraw = true;
                        }
                        '\r' | '\n' => { app.navigate_into(); }
                        '\x08' => { // Backspace = go up
                            app.selected = 0;
                            if app.entries.first().map(|e| e.name.as_str()) == Some("..") {
                                app.navigate_into();
                            }
                        }
                        'r' | 'R' => { app.refresh(); }
                        _ => {}
                    }
                }
                crate::gui::wm::GuiEvent::MouseClick { x, y, button } => {
                    if button & 1 != 0 {
                        let list_y = 46;
                        let row_h = 18;
                        if y >= list_y {
                            let idx = ((y - list_y) / row_h) as usize + app.scroll;
                            if idx < app.entries.len() {
                                let tick = crate::process::scheduler::get_ticks();
                                if app.last_click_entry == Some(idx) && tick.saturating_sub(app.last_click_tick) < 50 {
                                    app.navigate_into();
                                } else {
                                    app.selected = idx;
                                    app.last_click_entry = Some(idx);
                                    app.last_click_tick = tick;
                                    app.needs_redraw = true;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        if app.needs_redraw {
            let w = width;
            let h = height;
            let buf = &mut buffer;
            
            // Background
            FileManagerApp::fill(buf, w, h, 0, 0, w as i32, h as i32, 0xFF1E1E2E);
    
            // Toolbar / address bar
            FileManagerApp::fill(buf, w, h, 0, 0, w as i32, 24, 0xFF313244);
            FileManagerApp::draw_text(buf, w, h, 4, 4, "F", 0xFFCBA6F7);
            FileManagerApp::draw_text(buf, w, h, 24, 4, &app.current_path, 0xFFCDD6F4);
    
            // Column headers
            let header_y = 26;
            FileManagerApp::fill(buf, w, h, 0, header_y, w as i32, 18, 0xFF45475A);
            FileManagerApp::draw_text(buf, w, h, 4, header_y + 2, "Name", 0xFFBAC2E8);
            FileManagerApp::draw_text(buf, w, h, w as i32 - 100, header_y + 2, "Size", 0xFFBAC2E8);
            FileManagerApp::draw_text(buf, w, h, w as i32 - 55, header_y + 2, "Type", 0xFFBAC2E8);
    
            // File list
            let list_y = 46;
            let row_h = 18;
            let visible = ((h as i32 - list_y - 24) / row_h).max(0) as usize;
    
            for (i, entry) in app.entries.iter().enumerate().skip(app.scroll) {
                if i - app.scroll >= visible { break; }
                let ry = list_y + ((i - app.scroll) as i32 * row_h);
    
                let bg = if i == app.selected { 0xFF585B70 } else if (i - app.scroll) % 2 == 0 { 0xFF1E1E2E } else { 0xFF24273A };
                FileManagerApp::fill(buf, w, h, 0, ry, w as i32, row_h, bg);
    
                let icon = if entry.is_dir { "D " } else { "F " };
                let icon_color = if entry.is_dir { 0xFF89DCEB } else { 0xFFA6E3A1 };
                FileManagerApp::draw_text(buf, w, h, 4, ry + 2, icon, icon_color);
    
                let name_disp: String = entry.name.chars().take(28).collect();
                FileManagerApp::draw_text(buf, w, h, 22, ry + 2, &name_disp, 0xFFCDD6F4);
    
                if !entry.is_dir && entry.size > 0 {
                    let size_str = if entry.size >= 1048576 {
                        alloc::format!("{:.1}M", entry.size as f64 / 1048576.0)
                    } else if entry.size >= 1024 {
                        alloc::format!("{}K", entry.size / 1024)
                    } else {
                        alloc::format!("{}B", entry.size)
                    };
                    FileManagerApp::draw_text(buf, w, h, w as i32 - 100, ry + 2, &size_str, 0xFFF5E0DC);
                }
    
                let type_str = if entry.is_dir { "DIR" } else {
                    if entry.name.ends_with(".elf") { "ELF" }
                    else if entry.name.ends_with(".rs") { "RS" }
                    else if entry.name.ends_with(".txt") { "TXT" }
                    else { "FILE" }
                };
                FileManagerApp::draw_text(buf, w, h, w as i32 - 55, ry + 2, type_str, 0xFFCBA6F7);
            }
    
            // Status bar
            FileManagerApp::fill(buf, w, h, 0, h as i32 - 20, w as i32, 20, 0xFF313244);
            let status = alloc::format!("{} items  |  {}", app.entries.len(), app.status);
            FileManagerApp::draw_text(buf, w, h, 4, h as i32 - 17, &status, 0xFF6C7086);
            
            app.needs_redraw = false;
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id: 22,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::yield_now();
    }
}
