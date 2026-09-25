use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::video;

pub fn notepad_main() {
    let id = 18;
    let width = 400;
    let height = 300;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: String::from("MithlPad"),
        x: 300,
        y: 300,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFFFFFFFF; width * height];
    let mut content = String::from("Welcome to MithlPad!\nStart typing...\n");
    
    loop {
        let events = crate::gui::wm::pop_events(id);
        let mut dirty = true; // Always update initially
        for ev in events {
            match ev {
                crate::gui::wm::GuiEvent::KeyPress { key } => {
                    if key == '\x08' {
                        if !content.is_empty() {
                            content.pop();
                        }
                    } else if key >= ' ' && key <= '~' || key == '\n' {
                        content.push(key);
                    }
                    dirty = true;
                }
                _ => {}
            }
        }
        
        if dirty {
            for i in 0..buffer.len() {
                buffer[i] = 0xFFFFFFFF;
            }

            let mut y = 4;
            let line_height = 10;
            for line in content.split('\n') {
                if y + line_height > height {
                    break;
                }
                video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 4, y as i64, line, 0xFF000000);
                y += line_height;
            }
            
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        for _ in 0..50 {
            crate::process::scheduler::yield_now();
        }
    }
}
