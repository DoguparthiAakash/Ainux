use crate::drivers::video;
use alloc::string::String;
use alloc::vec::Vec;
use crate::fs::vfs::{FileType, resolve_path};

// Colors (Catppuccin Macchiato inspired)
const COL_BORDER: u32 = 0x00585B70;
const COL_DIR: u32 = 0x008CAAEE; // Blue
const COL_FILE: u32 = 0x00CAD3F5; // Text
const COL_SEL_BG: u32 = 0x00CBA6F7; // Mauve
const COL_SEL_FG: u32 = 0x00181825; // Mantle (Dark)
const COL_DIM: u32 = 0x006E738D;  // Overlay
const COL_WARN: u32 = 0x00E82424;

struct FmState {
    current_path: String,
    entries: Vec<(String, bool)>, // (name, is_dir)
    parent_entries: Vec<String>,
    selected_index: usize,
    scroll_offset: usize,
    preview_cache_index: Option<usize>,
    preview_cache_lines: Vec<String>,
}

impl FmState {
    fn new(path: &str) -> Self {
        let mut state = Self {
            current_path: String::from(path),
            entries: Vec::new(),
            parent_entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            preview_cache_index: None,
            preview_cache_lines: Vec::new(),
        };
        state.refresh();
        state
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.parent_entries.clear();
        self.preview_cache_index = None;
        self.preview_cache_lines.clear();
        
        if let Ok(inode) = resolve_path(&self.current_path) {
            if let Ok(files) = inode.read_dir() {
                for name in files {
                    let full_path = if self.current_path == "/" {
                        alloc::format!("/{}", name)
                    } else {
                        alloc::format!("{}/{}", self.current_path, name)
                    };
                    let mut is_dir = false;
                    if let Ok(entry_inode) = resolve_path(&full_path) {
                        if let Ok(stat) = entry_inode.stat() {
                            is_dir = stat.file_type == FileType::Directory;
                        }
                    }
                    self.entries.push((name, is_dir));
                }
            }
        }
        
        let parent_path = get_parent_dir(&self.current_path);
        if let Ok(inode) = resolve_path(&parent_path) {
            if let Ok(files) = inode.read_dir() {
                for f in files {
                    self.parent_entries.push(f);
                }
            }
        }
        self.parent_entries.sort();
        
        // Sort: directories first, then files, alphabetically
        self.entries.sort_by(|a, b| {
            if a.1 && !b.1 {
                core::cmp::Ordering::Less
            } else if !a.1 && b.1 {
                core::cmp::Ordering::Greater
            } else {
                a.0.cmp(&b.0)
            }
        });
        
        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len().saturating_sub(1);
        }
    }
    
    fn update_preview(&mut self, visible_items: usize, right_w: usize) {
        if self.entries.is_empty() { return; }
        if self.preview_cache_index == Some(self.selected_index) { return; }
        
        self.preview_cache_lines.clear();
        let (sel_name, is_dir) = &self.entries[self.selected_index];
        let full_path = if self.current_path == "/" { alloc::format!("/{}", sel_name) } else { alloc::format!("{}/{}", self.current_path, sel_name) };
        let max_preview_width = right_w.saturating_sub(3);
        
        if *is_dir {
            let mut count = 0;
            if let Ok(inode) = resolve_path(&full_path) {
                if let Ok(files) = inode.read_dir() {
                    for f in files.iter().take(visible_items.saturating_sub(3)) {
                        self.preview_cache_lines.push(truncate(f, max_preview_width));
                        count += 1;
                    }
                    if files.len() > count {
                        self.preview_cache_lines.push(String::from("... more"));
                    }
                }
            }
        } else {
            if let Ok(inode) = resolve_path(&full_path) {
                if let Ok(handle) = inode.open(0) {
                    let mut buf = alloc::vec![0u8; 1024];
                    if let Ok(bytes) = handle.read(&mut buf, 0) {
                        if let Ok(s) = core::str::from_utf8(&buf[..bytes]) {
                            let lines: Vec<&str> = s.split('\n').collect();
                            for line in lines.iter().take(visible_items.saturating_sub(3)) {
                                self.preview_cache_lines.push(truncate(line.trim_end(), max_preview_width));
                            }
                        } else {
                            self.preview_cache_lines.push(String::from("<Binary File>"));
                        }
                    }
                }
            }
        }
        self.preview_cache_index = Some(self.selected_index);
    }
}

fn get_parent_dir(path: &str) -> String {
    if path == "/" {
        return String::from("/");
    }
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    if parts.is_empty() || (parts.len() == 1 && parts[0] == "") {
        String::from("/")
    } else {
        parts.join("/")
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let mut truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        truncated.push('~');
        truncated
    } else {
        String::from(s)
    }
}

pub fn launch_fm() {
    let mut state = FmState::new("/");
    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            video::clear();
            let width = *video::CONSOLE_WIDTH.lock();
            let height = *video::CONSOLE_HEIGHT.lock();
            let bg = video::THEME.lock().bg;

            let left_w = (width * 20) / 100;
            let center_w = (width * 30) / 100;
            let right_w = width.saturating_sub(left_w + center_w);

            let pane_h = height.saturating_sub(2); // Leave 2 lines for header and footer

            // Draw Top Header
            video::put_str_at(0, 0, &alloc::format!(" Ainux FileManager 🚀 | Path: {} ", state.current_path), COL_SEL_BG, bg);

            // Draw Pane Borders (Top and Bottom)
            for i in 0..width {
                video::put_char_at(i, 1, '─', COL_BORDER, bg);
                video::put_char_at(i, height - 2, '─', COL_BORDER, bg);
            }
            
            // Pane Separators
            for j in 1..(height - 1) {
                video::put_char_at(left_w, j, '│', COL_BORDER, bg);
                video::put_char_at(left_w + center_w, j, '│', COL_BORDER, bg);
            }
            video::put_char_at(left_w, 1, '┬', COL_BORDER, bg);
            video::put_char_at(left_w + center_w, 1, '┬', COL_BORDER, bg);
            video::put_char_at(left_w, height - 2, '┴', COL_BORDER, bg);
            video::put_char_at(left_w + center_w, height - 2, '┴', COL_BORDER, bg);

            // --- PANE 1: Parent Directory ---
            for (i, name) in state.parent_entries.iter().take(pane_h.saturating_sub(1)).enumerate() {
                let is_current = name == state.current_path.split('/').last().unwrap_or("");
                let display_str = truncate(name, left_w.saturating_sub(2));
                let fg = if is_current { COL_DIR } else { COL_DIM };
                video::put_str_at(1, 2 + i, &display_str, fg, bg);
            }

            // --- PANE 2: Current Directory ---
            let visible_items = pane_h.saturating_sub(1);
            if state.selected_index < state.scroll_offset {
                state.scroll_offset = state.selected_index;
            } else if state.selected_index >= state.scroll_offset + visible_items {
                state.scroll_offset = state.selected_index - visible_items + 1;
            }

            if state.entries.is_empty() {
                video::put_str_at(left_w + 2, 2, "Empty", COL_DIM, bg);
            } else {
                for (i, (name, is_dir)) in state.entries.iter().skip(state.scroll_offset).take(visible_items).enumerate() {
                    let y = 2 + i;
                    let is_selected = (i + state.scroll_offset) == state.selected_index;
                    
                    let mut fg = if *is_dir { COL_DIR } else { COL_FILE };
                    let mut item_bg = bg;
                    
                    if is_selected {
                        fg = COL_SEL_FG;
                        item_bg = COL_SEL_BG;
                    }

                    let prefix = if *is_dir { "[DIR] " } else { "[FILE] " };
                    let max_name_len = center_w.saturating_sub(prefix.chars().count() + 1);
                    let display_name = truncate(name, max_name_len);
                    
                    // Pad with spaces for full selection highlight
                    let current_len = display_name.chars().count() + prefix.chars().count();
                    let pad_len = center_w.saturating_sub(current_len + 1);
                    let padding = alloc::string::String::from_utf8(alloc::vec![b' '; pad_len]).unwrap_or_default();
                    
                    let display_str = alloc::format!("{}{}{}", prefix, display_name, padding);
                    video::put_str_at(left_w + 1, y, &display_str, fg, item_bg);
                }
            }

            // --- PANE 3: Preview ---
            if !state.entries.is_empty() {
                state.update_preview(visible_items, right_w);
                let (sel_name, is_dir) = &state.entries[state.selected_index];
                let preview_x = left_w + center_w + 2;
                let max_preview_width = right_w.saturating_sub(3);
                
                if *is_dir {
                    video::put_str_at(preview_x, 2, &truncate(&alloc::format!("Directory: {}", sel_name), max_preview_width), COL_DIR, bg);
                } else {
                    video::put_str_at(preview_x, 2, &truncate(&alloc::format!("File: {}", sel_name), max_preview_width), COL_FILE, bg);
                }
                video::put_char_at(preview_x, 3, '─', COL_BORDER, bg);
                
                for (i, line) in state.preview_cache_lines.iter().enumerate() {
                    let fg = if *is_dir { COL_DIM } else if line == "<Binary File>" { COL_WARN } else { COL_FILE };
                    video::put_str_at(preview_x, 4 + i, line, fg, bg);
                }
            }

            // Draw Footer
            video::put_str_at(0, height - 1, " [h] Back | [j] Down | [k] Up | [l/Ent] Enter | [q] Quit ", COL_DIM, bg);
            needs_redraw = false;
        }

        // Input loop
        let c = crate::drivers::keyboard::get_char();
        if c != '\0' {
            needs_redraw = true;
            match c {
                'j' | 's' => {
                    if state.selected_index < state.entries.len().saturating_sub(1) {
                        state.selected_index += 1;
                    }
                }
                'k' | 'w' => {
                    if state.selected_index > 0 {
                        state.selected_index -= 1;
                    }
                }
                'l' | 'd' | '\n' | '\r' => {
                    if !state.entries.is_empty() {
                        let (name, is_dir) = state.entries[state.selected_index].clone();
                        let full_path = if state.current_path == "/" { alloc::format!("/{}", name) } else { alloc::format!("{}/{}", state.current_path, name) };
                        
                        if is_dir {
                            state.current_path = full_path;
                            state.selected_index = 0;
                            state.scroll_offset = 0;
                            state.refresh();
                        } else if c == '\n' || c == '\r' {
                            // Full-screen file view
                            video::clear();
                            video::put_str(&alloc::format!("Viewing file: {}\n", full_path));
                            video::put_str("──────────────────────────────────────────────────\n");
                            if let Ok(inode) = resolve_path(&full_path) {
                                if let Ok(handle) = inode.open(0) {
                                    let mut buf = alloc::vec![0u8; 4096];
                                    if let Ok(bytes) = handle.read(&mut buf, 0) {
                                        if let Ok(s) = core::str::from_utf8(&buf[..bytes]) {
                                            video::put_str(s);
                                        } else {
                                            video::put_str("<Binary File Content>");
                                        }
                                    }
                                }
                            }
                            video::put_str("\n\nPress 'q' to return...");
                            loop {
                                if crate::drivers::keyboard::get_char() == 'q' { break; }
                                crate::process::scheduler::yield_now();
                            }
                        }
                    }
                }
                'h' | 'a' => {
                    if state.current_path != "/" {
                        state.current_path = get_parent_dir(&state.current_path);
                        state.selected_index = 0;
                        state.scroll_offset = 0;
                        state.refresh();
                    }
                }
                'q' | '\x03' => break,
                _ => { needs_redraw = false; }
            }
        }
        crate::process::scheduler::yield_now();
    }
    video::clear();
}
