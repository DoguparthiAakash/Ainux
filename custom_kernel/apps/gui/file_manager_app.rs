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

struct SidebarItem {
    name: &'static str,
    path: &'static str,
    icon: &'static str,
}

const SIDEBAR_ITEMS: &[SidebarItem] = &[
    SidebarItem { name: "This PC",   path: "/",     icon: "[PC]" },
    SidebarItem { name: "Home",      path: "/home", icon: "[~]" },
    SidebarItem { name: "System",    path: "/bin",  icon: "[SYS]" },
    SidebarItem { name: "Devices",   path: "/dev",  icon: "[DEV]" },
    SidebarItem { name: "Config",    path: "/etc",  icon: "[CFG]" },
    SidebarItem { name: "Temp",      path: "/tmp",  icon: "[TMP]" },
];

pub struct FileManagerApp {
    current_path: String,
    entries: Vec<FileEntry>,
    selected: usize,
    scroll: usize,
    status: String,
    needs_redraw: bool,
    last_click_entry: Option<usize>,
    last_click_tick: u64,
    history: Vec<String>,
    history_idx: usize,
}

impl FileManagerApp {
    pub fn new() -> Self {
        let initial_path = String::from("/");
        let mut app = Self {
            current_path: initial_path.clone(),
            entries: Vec::new(),
            selected: 0,
            scroll: 0,
            status: String::from("Ready"),
            needs_redraw: true,
            last_click_entry: None,
            last_click_tick: 0,
            history: alloc::vec![initial_path],
            history_idx: 0,
        };
        app.refresh();
        app
    }

    pub fn change_directory(&mut self, new_path: String) {
        if self.current_path != new_path {
            if self.history_idx + 1 < self.history.len() {
                self.history.truncate(self.history_idx + 1);
            }
            self.history.push(new_path.clone());
            self.history_idx = self.history.len() - 1;
            self.current_path = new_path;
            self.selected = 0;
            self.scroll = 0;
            self.refresh();
        }
    }

    pub fn go_back(&mut self) {
        if self.history_idx > 0 {
            self.history_idx -= 1;
            self.current_path = self.history[self.history_idx].clone();
            self.selected = 0;
            self.scroll = 0;
            self.refresh();
        }
    }

    pub fn go_forward(&mut self) {
        if self.history_idx + 1 < self.history.len() {
            self.history_idx += 1;
            self.current_path = self.history[self.history_idx].clone();
            self.selected = 0;
            self.scroll = 0;
            self.refresh();
        }
    }

    pub fn go_up(&mut self) {
        if self.current_path != "/" {
            if let Some(pos) = self.current_path.rfind('/') {
                let parent = if pos == 0 {
                    String::from("/")
                } else {
                    String::from(&self.current_path[..pos])
                };
                self.change_directory(parent);
            }
        }
    }

    pub fn refresh(&mut self) {
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
                        self.status = alloc::format!("{} items", self.entries.len());
                    }
                    Err(_) => { self.status = String::from("Cannot read folder"); }
                }
            }
            Err(_) => { self.status = alloc::format!("Path not found: {}", self.current_path); }
        }

        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
        self.needs_redraw = true;
    }

    pub fn navigate_into(&mut self) {
        if self.entries.is_empty() { return; }
        if self.selected >= self.entries.len() { return; }
        let entry = self.entries[self.selected].clone();
        if entry.is_dir {
            if entry.name == ".." {
                self.go_up();
            } else {
                let target = if self.current_path == "/" {
                    alloc::format!("/{}", entry.name)
                } else {
                    alloc::format!("{}/{}", self.current_path, entry.name)
                };
                self.change_directory(target);
            }
        } else {
            self.status = alloc::format!("Selected: {} ({} bytes)", entry.name, entry.size);
            self.needs_redraw = true;
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

    fn draw_rect_border(buf: &mut [u32], bw: usize, bh: usize, x: i32, y: i32, w: i32, h: i32, c: u32) {
        Self::fill(buf, bw, bh, x, y, w, 1, c);
        Self::fill(buf, bw, bh, x, y + h - 1, w, 1, c);
        Self::fill(buf, bw, bh, x, y, 1, h, c);
        Self::fill(buf, bw, bh, x + w - 1, y, 1, h, c);
    }

    fn draw_text(buf: &mut [u32], bw: usize, bh: usize, x: i32, y: i32, text: &str, fg: u32) {
        crate::drivers::video::draw_text_to_buffer(buf, bw as i64, bh as i64, x as i64, y as i64, text, fg);
    }
}

pub fn file_manager_main() {
    let id = crate::gui::wm::generate_window_id();
    let mut width = 640usize;
    let mut height = 440usize;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("File Explorer"),
        x: 100,
        y: 60,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFF1E1E2E; width * height];
    let mut app = FileManagerApp::new();
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::WindowClosed => { return; }
                crate::gui::wm::GuiEvent::Resize { w, h } => {
                    let nw = (w as usize).max(360);
                    let nh = (h as usize).max(240);
                    if nw != width || nh != height {
                        width = nw;
                        height = nh;
                        buffer.resize(width * height, 0xFF1E1E2E);
                        crate::gui::wm::send_message(crate::gui::wm::GuiMessage::ResizeBuffer {
                            id,
                            w: width as i32,
                            h: height as i32,
                            buffer_ptr: buffer.as_ptr() as u64,
                        });
                        app.needs_redraw = true;
                    }
                }
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
                        '\x08' => { app.go_up(); }
                        'r' | 'R' => { app.refresh(); }
                        _ => {}
                    }
                }
                crate::gui::wm::GuiEvent::MouseClick { x, y, button } => {
                    if button & 1 != 0 {
                        // 1. Navigation Buttons
                        if y >= 6 && y <= 32 {
                            if x >= 8 && x <= 32 {
                                app.go_back();
                            } else if x >= 36 && x <= 60 {
                                app.go_forward();
                            } else if x >= 64 && x <= 88 {
                                app.go_up();
                            } else if x >= 92 && x <= 116 {
                                app.refresh();
                            }
                        }
                        // 2. Sidebar Items (Quick Access)
                        else if x < 140 && y >= 64 && y < height as i32 - 24 {
                            let s_idx = ((y - 64) / 24) as usize;
                            if s_idx < SIDEBAR_ITEMS.len() {
                                app.change_directory(String::from(SIDEBAR_ITEMS[s_idx].path));
                            }
                        }
                        // 3. File List Items
                        else if x >= 140 && y >= 62 && y < height as i32 - 22 {
                            let list_y = 62;
                            let row_h = 22;
                            let idx = ((y - list_y) / row_h) as usize + app.scroll;
                            if idx < app.entries.len() {
                                let tick = crate::process::scheduler::get_ticks();
                                if app.last_click_entry == Some(idx) && tick.saturating_sub(app.last_click_tick) < 60 {
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
            }
        }
        
        if app.needs_redraw {
            let w = width;
            let h = height;
            let buf = &mut buffer;
            
            // Background
            FileManagerApp::fill(buf, w, h, 0, 0, w as i32, h as i32, 0xFF1E1E2E);
    
            // ── 1. Modern Windows Command & Navigation Ribbon (Height: 38px) ───
            FileManagerApp::fill(buf, w, h, 0, 0, w as i32, 38, 0xFF24273A);
            FileManagerApp::fill(buf, w, h, 0, 37, w as i32, 1, 0xFF363A4F);

            // Nav buttons: Back [<], Forward [>], Up [^], Refresh [R]
            let back_fg = if app.history_idx > 0 { 0xFFCDD6F4 } else { 0xFF585B70 };
            FileManagerApp::fill(buf, w, h, 8, 7, 24, 24, 0xFF1E1E2E);
            FileManagerApp::draw_rect_border(buf, w, h, 8, 7, 24, 24, 0xFF363A4F);
            FileManagerApp::draw_text(buf, w, h, 16, 11, "<", back_fg);

            let fwd_fg = if app.history_idx + 1 < app.history.len() { 0xFFCDD6F4 } else { 0xFF585B70 };
            FileManagerApp::fill(buf, w, h, 36, 7, 24, 24, 0xFF1E1E2E);
            FileManagerApp::draw_rect_border(buf, w, h, 36, 7, 24, 24, 0xFF363A4F);
            FileManagerApp::draw_text(buf, w, h, 44, 11, ">", fwd_fg);

            let up_fg = if app.current_path != "/" { 0xFFCDD6F4 } else { 0xFF585B70 };
            FileManagerApp::fill(buf, w, h, 64, 7, 24, 24, 0xFF1E1E2E);
            FileManagerApp::draw_rect_border(buf, w, h, 64, 7, 24, 24, 0xFF363A4F);
            FileManagerApp::draw_text(buf, w, h, 72, 11, "^", up_fg);

            FileManagerApp::fill(buf, w, h, 92, 7, 24, 24, 0xFF1E1E2E);
            FileManagerApp::draw_rect_border(buf, w, h, 92, 7, 24, 24, 0xFF363A4F);
            FileManagerApp::draw_text(buf, w, h, 100, 11, "R", 0xFF89B4FA);

            // Address Breadcrumb Bar
            let addr_w = (w as i32 - 250).max(120);
            FileManagerApp::fill(buf, w, h, 122, 7, addr_w, 24, 0xFF181825);
            FileManagerApp::draw_rect_border(buf, w, h, 122, 7, addr_w, 24, 0xFF3A3D52);
            FileManagerApp::draw_text(buf, w, h, 128, 11, "This PC >", 0xFF89B4FA);
            let display_path = if app.current_path == "/" { " Root" } else { &app.current_path };
            FileManagerApp::draw_text(buf, w, h, 204, 11, display_path, 0xFFCDD6F4);

            // Search Box
            let search_x = 126 + addr_w;
            let search_w = (w as i32 - search_x - 8).max(50);
            if search_w > 60 {
                FileManagerApp::fill(buf, w, h, search_x, 7, search_w, 24, 0xFF181825);
                FileManagerApp::draw_rect_border(buf, w, h, search_x, 7, search_w, 24, 0xFF3A3D52);
                FileManagerApp::draw_text(buf, w, h, search_x + 8, 11, "Search...", 0xFF6C7086);
            }

            // ── 2. Left Navigation Pane (Sidebar - Quick Access, Width: 140px) ─
            let sidebar_w = 140i32;
            FileManagerApp::fill(buf, w, h, 0, 38, sidebar_w, h as i32 - 38 - 22, 0xFF181825);
            FileManagerApp::fill(buf, w, h, sidebar_w - 1, 38, 1, h as i32 - 38 - 22, 0xFF313244);

            FileManagerApp::draw_text(buf, w, h, 12, 46, "QUICK ACCESS", 0xFF6C7086);
            for (i, item) in SIDEBAR_ITEMS.iter().enumerate() {
                let sy = 64 + (i as i32 * 24);
                let is_active = app.current_path == item.path;
                if is_active {
                    FileManagerApp::fill(buf, w, h, 6, sy, sidebar_w - 12, 22, 0xFF2D3045);
                    FileManagerApp::fill(buf, w, h, 6, sy + 2, 3, 18, 0xFF89B4FA);
                    FileManagerApp::draw_text(buf, w, h, 14, sy + 3, item.icon, 0xFF89DCEB);
                    FileManagerApp::draw_text(buf, w, h, 54, sy + 3, item.name, 0xFFFFFFFF);
                } else {
                    FileManagerApp::draw_text(buf, w, h, 14, sy + 3, item.icon, 0xFF6C7086);
                    FileManagerApp::draw_text(buf, w, h, 54, sy + 3, item.name, 0xFFBAC2DE);
                }
            }

            // ── 3. Main Content Pane — Column Headers (Height: 24px) ───────────
            let main_x = sidebar_w;
            let main_w = w as i32 - main_x;
            FileManagerApp::fill(buf, w, h, main_x, 38, main_w, 24, 0xFF212334);
            FileManagerApp::fill(buf, w, h, main_x, 61, main_w, 1, 0xFF313244);

            FileManagerApp::draw_text(buf, w, h, main_x + 12, 42, "Name", 0xFFBAC2E8);
            FileManagerApp::draw_text(buf, w, h, w as i32 - 130, 42, "Type", 0xFFBAC2E8);
            FileManagerApp::draw_text(buf, w, h, w as i32 - 65, 42, "Size", 0xFFBAC2E8);

            // ── 4. Main Content Pane — File & Folder List ─────────────────────
            let list_y = 62i32;
            let row_h = 22i32;
            let list_bottom = h as i32 - 22;
            let visible = ((list_bottom - list_y) / row_h).max(0) as usize;

            for (i, entry) in app.entries.iter().enumerate().skip(app.scroll) {
                if i - app.scroll >= visible { break; }
                let ry = list_y + ((i - app.scroll) as i32 * row_h);
                let is_sel = i == app.selected;

                let row_bg = if is_sel {
                    0xFF363A4F
                } else if (i - app.scroll) % 2 == 0 {
                    0xFF1E1E2E
                } else {
                    0xFF222436
                };

                FileManagerApp::fill(buf, w, h, main_x, ry, main_w, row_h, row_bg);
                if is_sel {
                    FileManagerApp::fill(buf, w, h, main_x, ry + 2, 3, row_h - 4, 0xFF89B4FA);
                }

                // File/Folder icon & color
                let (icon_tag, icon_color) = if entry.is_dir {
                    ("[DIR] ", 0xFFF9E2AF) // Vibrant Gold
                } else if entry.name.ends_with(".elf") {
                    ("[EXE] ", 0xFFA6E3A1) // Emerald Executable
                } else if entry.name.ends_with(".txt") || entry.name.ends_with(".rs") || entry.name.ends_with(".c") {
                    ("[DOC] ", 0xFFCBA6F7) // Lavender Document
                } else {
                    ("[FIL] ", 0xFF89DCEB) // Cyan General File
                };

                FileManagerApp::draw_text(buf, w, h, main_x + 8, ry + 3, icon_tag, icon_color);

                let max_chars = ((main_w - 180) / 8).max(10) as usize;
                let name_disp: String = entry.name.chars().take(max_chars).collect();
                let name_fg = if is_sel { 0xFFFFFFFF } else { 0xFFCDD6F4 };
                FileManagerApp::draw_text(buf, w, h, main_x + 58, ry + 3, &name_disp, name_fg);

                // Type column
                let type_str = if entry.is_dir {
                    "Folder"
                } else if entry.name.ends_with(".elf") {
                    "Application"
                } else if entry.name.ends_with(".txt") {
                    "Text Document"
                } else if entry.name.ends_with(".rs") {
                    "Rust Source"
                } else {
                    "File"
                };
                FileManagerApp::draw_text(buf, w, h, w as i32 - 130, ry + 3, type_str, 0xFFBAC2DE);

                // Size column
                if !entry.is_dir && entry.size > 0 {
                    let size_str = if entry.size >= 1048576 {
                        alloc::format!("{:.1}MB", entry.size as f64 / 1048576.0)
                    } else if entry.size >= 1024 {
                        alloc::format!("{}KB", entry.size / 1024)
                    } else {
                        alloc::format!("{}B", entry.size)
                    };
                    FileManagerApp::draw_text(buf, w, h, w as i32 - 65, ry + 3, &size_str, 0xFFF5E0DC);
                }
            }

            // ── 5. Status Bar (Height: 22px) ──────────────────────────────────
            let status_y = h as i32 - 22;
            FileManagerApp::fill(buf, w, h, 0, status_y, w as i32, 22, 0xFF181825);
            FileManagerApp::fill(buf, w, h, 0, status_y, w as i32, 1, 0xFF313244);

            let status_left = alloc::format!("{} items  |  {}", app.entries.len(), app.status);
            FileManagerApp::draw_text(buf, w, h, 12, status_y + 3, &status_left, 0xFFBAC2DE);

            let status_right = "MithlFS | Ext2/RAMFS";
            FileManagerApp::draw_text(buf, w, h, w as i32 - 170, status_y + 3, status_right, 0xFF6C7086);
            
            app.needs_redraw = false;
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::sleep(1);
    }
}
