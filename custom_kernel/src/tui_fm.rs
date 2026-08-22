use crate::drivers::video;
use alloc::string::String;
use alloc::vec::Vec;

pub fn launch_fm() {
    let mut current_path = String::from("/");
    let mut selected_index = 0;

    loop {
        video::clear();
        let width = *video::CONSOLE_WIDTH.lock();
        let height = *video::CONSOLE_HEIGHT.lock();

        // Draw outer box
        video::draw_tui_title_box(0, 0, width, height, " Ainux File Manager ", 0x00AAAAFF);

        // Header
        video::put_str_at(2, 2, &alloc::format!("Path: {}", current_path), 0x00FFFFFF, video::THEME.lock().bg);

        // Read directory contents
        let mut entries = Vec::new();
        if let Ok(inode) = crate::fs::vfs::resolve_path(&current_path) {
            if let Ok(files) = inode.read_dir() {
                entries = files;
            }
        }

        if entries.is_empty() {
            video::put_str_at(2, 4, "(Empty directory)", 0x00888888, video::THEME.lock().bg);
        } else {
            // Draw entries
            let max_visible = height.saturating_sub(6);
            let start_idx = if selected_index >= max_visible { selected_index - max_visible + 1 } else { 0 };

            for (i, entry_name) in entries.iter().skip(start_idx).take(max_visible).enumerate() {
                let y = 4 + i;
                let is_selected = (i + start_idx) == selected_index;
                
                let mut fg = 0x00FFFFFF;
                let mut bg = video::THEME.lock().bg;
                if is_selected {
                    fg = 0x00000000;
                    bg = 0x00AAAAFF; // Highlight
                }

                // Try to get stat to determine if it's a directory
                let full_path = if current_path == "/" { alloc::format!("/{}", entry_name) } else { alloc::format!("{}/{}", current_path, entry_name) };
                let mut suffix = "";
                if let Ok(entry_inode) = crate::fs::vfs::resolve_path(&full_path) {
                    if let Ok(stat) = entry_inode.stat() {
                        if stat.file_type == crate::fs::vfs::FileType::Directory {
                            suffix = "/";
                        }
                    }
                }
                
                let display_str = alloc::format!("{}{}", entry_name, suffix);
                video::put_str_at(2, y, &display_str, fg, bg);
            }
        }

        // Instructions
        video::put_str_at(2, height - 2, " [w/s] Move | [d] Enter | [a] Back | [q] Quit ", 0x00888888, video::THEME.lock().bg);

        // Very basic input loop (mocking blocking input since Ainux shell uses yield_now)
        let mut action = 0;
        loop {
            let c = crate::drivers::keyboard::get_char();
            if c != '\0' {
                match c {
                    'w' => { action = 1; break; }
                    's' => { action = 2; break; }
                    'd' => { action = 3; break; }
                    'a' => { action = 4; break; }
                    'q' => { action = 5; break; }
                    _ => {}
                }
            }
            crate::process::scheduler::yield_now();
        }

        if action == 1 && selected_index > 0 {
            selected_index -= 1;
        } else if action == 2 && selected_index < entries.len().saturating_sub(1) {
            selected_index += 1;
        } else if action == 3 && !entries.is_empty() {
            let entry_name = &entries[selected_index];
            let full_path = if current_path == "/" { alloc::format!("/{}", entry_name) } else { alloc::format!("{}/{}", current_path, entry_name) };
            if let Ok(entry_inode) = crate::fs::vfs::resolve_path(&full_path) {
                if let Ok(stat) = entry_inode.stat() {
                    if stat.file_type == crate::fs::vfs::FileType::Directory {
                        current_path = full_path;
                        selected_index = 0;
                    } else {
                        // Display file contents (simple view)
                        video::clear();
                        video::put_str(&alloc::format!("Viewing file: {}\n", full_path));
                        if let Ok(handle) = entry_inode.open(0) {
                            let mut buf = alloc::vec![0u8; 1024];
                            if let Ok(bytes) = handle.read(&mut buf, 0) {
                                if let Ok(s) = core::str::from_utf8(&buf[..bytes]) {
                                    video::put_str(s);
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
        } else if action == 4 {
            if current_path != "/" {
                let mut parts: Vec<&str> = current_path.split('/').collect();
                parts.pop();
                if parts.is_empty() || (parts.len() == 1 && parts[0] == "") {
                    current_path = String::from("/");
                } else {
                    current_path = parts.join("/");
                }
                selected_index = 0;
            }
        } else if action == 5 {
            break;
        }
    }
    video::clear();
}
