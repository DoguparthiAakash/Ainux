use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

#[derive(Clone)]
pub enum GuiEvent {
    MouseClick { x: i32, y: i32, button: u8 },
    KeyPress { key: char },
}

pub enum GuiMessage {
    CreateWindow {
        id: u32,
        title: String,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    },
    UpdateBuffer {
        id: u32,
        buffer_ptr: u64,
    },
    CloseWindow {
        id: u32,
    },
}

pub static GUI_MESSAGES: Mutex<Vec<GuiMessage>> = Mutex::new(Vec::new());

// Temporary mapping to route events back to the right app
pub static WINDOW_EVENTS: Mutex<Vec<(u32, GuiEvent)>> = Mutex::new(Vec::new());

pub fn send_message(msg: GuiMessage) {
    GUI_MESSAGES.lock().push(msg);
}

pub fn pop_events(window_id: u32) -> Vec<GuiEvent> {
    let mut events = WINDOW_EVENTS.lock();
    let mut my_events = Vec::new();
    events.retain(|(wid, ev)| {
        if *wid == window_id {
            my_events.push(ev.clone());
            false
        } else {
            true
        }
    });
    my_events
}
