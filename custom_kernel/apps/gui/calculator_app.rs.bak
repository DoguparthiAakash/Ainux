use alloc::string::{String, ToString};
use crate::drivers::video;

pub struct CalculatorApp {
    display: String,
}

impl CalculatorApp {
    pub fn new() -> Self {
        Self {
            display: String::from("0"),
        }
    }
}

pub fn calculator_main() {
    let id = 8;
    let width = 300;
    let height = 250;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("Calculator"),
        x: 250,
        y: 150,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFFC0C0C0; width * height];
    let mut app = CalculatorApp::new();
    let mut needs_redraw = true;
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::MouseClick { .. } => {}
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    if c == 'c' || c == 'C' {
                        app.display = String::from("0");
                        needs_redraw = true;
                    } else if c >= '0' && c <= '9' || c == '+' || c == '-' || c == '*' || c == '/' {
                        if app.display == "0" {
                            app.display.clear();
                        }
                        app.display.push(c);
                        needs_redraw = true;
                    } else if c == '=' || c == '\n' {
                        app.display.push_str(" = ???");
                        needs_redraw = true;
                    }
                }
            }
        }
        
        if needs_redraw {
            for i in 0..buffer.len() {
                buffer[i] = 0xFFC0C0C0;
            }
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 10, 10, &app.display, 0xFF000000);
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 10, 40, "Press digits, +, -, *, /, =, C", 0xFF000000);
            
            needs_redraw = false;
            
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::yield_now();
    }
}
