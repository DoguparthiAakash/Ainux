use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::drivers::video;
use crate::fs::pipe::create_pipe;
use crate::fs::vfs::{ArcHandle, FileHandle};

pub fn terminal_main() {
    let id = crate::gui::wm::generate_window_id();
    let width = 560;
    let height = 360;
    
    // Create window
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: String::from("Mithl Terminal"),
        x: 180,
        y: 160,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFF141722; width * height];
    let mut lines: Vec<String> = Vec::new();
    lines.push(String::from("Mithl OS Terminal [Ainux Core v2.0]"));
    lines.push(String::from("Type 'help' for command directory, 'clock' to launch Clock App."));
    lines.push(String::new());
    
    let mut current_line = String::new();
    let mut history: Vec<String> = Vec::new();
    let mut history_index: usize = 0;
    let mut draft_line = String::new();
    let mut scroll_offset = 0;
    let mut auto_scroll = true;
    let mut blink_counter = 0u32;
    
    let (reader, writer) = create_pipe();
    let reader_arc = reader as ArcHandle;
    let writer_arc = writer as ArcHandle;
    *crate::shell::CURRENT_OUT.lock() = Some(writer_arc.clone());
    
    loop {
        let events = crate::gui::wm::pop_events(id);
        for ev in events {
            match ev {
                crate::gui::wm::GuiEvent::WindowClosed => {
                    return;
                }
                crate::gui::wm::GuiEvent::KeyPress { key } => {
                    if key == '\n' {
                        let cmd = current_line.clone();
                        let trimmed = cmd.trim();
                        if trimmed == "clear" {
                            lines.clear();
                            current_line.clear();
                            scroll_offset = 0;
                            auto_scroll = true;
                        } else {
                            let mut prompt_line = crate::shell::get_prompt();
                            prompt_line.push_str(&cmd);
                            lines.push(prompt_line);
                            
                            if !trimmed.is_empty() {
                                if history.last().map(|s| s.as_str()) != Some(trimmed) {
                                    history.push(cmd.clone());
                                }
                                history_index = history.len();
                                draft_line.clear();
                                
                                // Prepare fresh line for command output
                                lines.push(String::new());
                                
                                *crate::shell::CURRENT_OUT.lock() = Some(writer_arc.clone());
                                crate::shell::execute_command(&cmd);
                                *crate::shell::CURRENT_OUT.lock() = Some(writer_arc.clone());
                            }
                            
                            current_line.clear();
                            auto_scroll = true;
                        }
                    } else if key == '\x08' {
                        current_line.pop();
                    } else if key == '\u{2191}' { // Up arrow: command history previous
                        if !history.is_empty() {
                            if history_index == history.len() {
                                draft_line = current_line.clone();
                            }
                            if history_index > 0 {
                                history_index -= 1;
                                current_line = history[history_index].clone();
                            }
                        }
                    } else if key == '\u{2193}' { // Down arrow: command history next
                        if history_index < history.len() {
                            history_index += 1;
                            if history_index == history.len() {
                                current_line = draft_line.clone();
                            } else {
                                current_line = history[history_index].clone();
                            }
                        }
                    } else if key >= ' ' && key <= '~' {
                        current_line.push(key);
                    }
                }
                _ => {}
            }
        }
        
        // Drain all available shell output from the pipe
        let mut pipe_buf = [0u8; 1024];
        let mut got_output = false;
        while let Ok(n) = reader_arc.read(&mut pipe_buf, 0) {
            if n == 0 { break; }
            got_output = true;
            if let Ok(s) = core::str::from_utf8(&pipe_buf[0..n]) {
                for (i, part) in s.split('\n').enumerate() {
                    if i == 0 {
                        if let Some(last) = lines.last_mut() {
                            last.push_str(part);
                        } else {
                            lines.push(part.to_string());
                        }
                    } else {
                        lines.push(part.to_string());
                    }
                }
            }
        }
        
        if got_output {
            auto_scroll = true;
        }
        
        let line_height = 16;
        let visible_history_lines = (height / line_height).saturating_sub(2);
        let max_scroll = lines.len().saturating_sub(visible_history_lines);
        
        if auto_scroll || scroll_offset > max_scroll {
            scroll_offset = max_scroll;
        }
        
        // Draw background with deep void obsidian tone
        for i in 0..buffer.len() {
            buffer[i] = 0xFF141722;
        }
        
        // Render history lines
        let mut y = 6;
        let start_idx = scroll_offset;
        let end_idx = core::cmp::min(lines.len(), start_idx + visible_history_lines);
        
        for i in start_idx..end_idx {
            let line = &lines[i];
            let fg = if line.starts_with("Mithl OS Terminal") {
                0xFF00FFCC
            } else if line.contains('@') && (line.contains(":#") || line.contains(":$")) {
                0xFF00E5FF
            } else {
                0xFFE0E6ED
            };
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 6, y as i64, line, fg);
            y += line_height;
        }
        
        // Draw active prompt line with cursor at the bottom
        let prompt_y = height.saturating_sub(line_height + 6);
        let prompt_prefix = crate::shell::get_prompt();
        let mut display_line = prompt_prefix.clone();
        display_line.push_str(&current_line);
        
        blink_counter = (blink_counter + 1) % 40;
        if blink_counter < 25 {
            display_line.push('_');
        }
        
        video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 6, prompt_y as i64, &display_line, 0xFF00FFCC);
        
        crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
            id,
            buffer_ptr: buffer.as_ptr() as u64,
        });
        
        for _ in 0..5 {
            crate::process::scheduler::sleep(1);
        }
    }
}
