use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::drivers::video;
use crate::fs::pipe::create_pipe;
use crate::fs::vfs::{ArcHandle, FileHandle};
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

pub fn terminal_main() {
    let id = 1;
    let width = 500;
    let height = 350;
    
    // Create window
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: String::from("Mithl Terminal"),
        x: 200,
        y: 200,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFF000000; width * height];
    let mut lines: Vec<String> = Vec::new();
    lines.push(String::from("Mithl OS Terminal (MithlUI Subsystem)"));
    lines.push(String::from("Type 'help' for commands"));
    
    let mut current_line = String::new();
    let mut scroll_offset = 0;
    
    let (reader, writer) = create_pipe();
    let reader_arc = reader as ArcHandle;
    *crate::shell::CURRENT_OUT.lock() = Some(writer as ArcHandle);
    
    loop {
        let events = crate::gui::wm::pop_events(id);
        for ev in events {
            match ev {
                crate::gui::wm::GuiEvent::KeyPress { key } => {
                    if key == '\n' {
                        let cmd = current_line.clone();
                        let mut prompt_line = crate::shell::get_prompt();
                        prompt_line.push_str(&cmd);
                        lines.push(prompt_line);
                        current_line.clear();
                        
                        if !cmd.trim().is_empty() {
                            lines.push(String::new());
                            // Execute synchronously for now (or offload to worker later)
                            crate::shell::execute_command(&cmd);
                        } else {
                            lines.push(crate::shell::get_prompt());
                        }
                    } else if key == '\x08' {
                        current_line.pop();
                    } else if key == '\u{2191}' { // Up arrow
                        if scroll_offset > 0 { scroll_offset -= 1; }
                    } else if key == '\u{2193}' { // Down arrow
                        scroll_offset += 1;
                    } else if key >= ' ' && key <= '~' {
                        current_line.push(key);
                    }
                }
                _ => {}
            }
        }
        
        // Read shell output pipe
        let mut pipe_buf = [0u8; 1024];
        if let Ok(n) = reader_arc.read(&mut pipe_buf, 0) {
            if n > 0 {
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
        }
        
        let line_height = 16;
        let max_lines = height / line_height - 1;
        if lines.len() > max_lines {
            // Auto scroll down unless manually scrolled back
            scroll_offset = lines.len() - max_lines;
        } else {
            scroll_offset = 0;
        }

        // Draw background
        for i in 0..buffer.len() {
            buffer[i] = 0xFF1E1E1E; // Dark grey background like modern terminals
        }
        
        let mut y = 4;
        let start_idx = scroll_offset;
        let end_idx = core::cmp::min(lines.len(), start_idx + max_lines);
        
        for i in start_idx..end_idx {
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 4, y as i64, &lines[i], 0xFFFFFFFF);
            y += line_height;
        }
        
        if y + line_height <= height {
            let mut display_line = crate::shell::get_prompt();
            display_line.push_str(&current_line);
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 4, y as i64, &display_line, 0xFFFFFFFF);
        }
        
        crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
            id,
            buffer_ptr: buffer.as_ptr() as u64,
        });
        
        for _ in 0..10 {
            crate::process::scheduler::yield_now();
        }
    }
}
