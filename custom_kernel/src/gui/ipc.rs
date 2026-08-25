// GUI IPC Primitives
// These define the structures used to communicate between userspace apps and the Window Manager

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum GuiCommand {
    CreateWindow,
    DestroyWindow,
    UpdateContent, // Uses shared memory or messages
    MoveWindow,
    SetTitle,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiMessage {
    pub cmd: GuiCommand,
    pub window_id: usize,
    pub arg1: u64,
    pub arg2: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum GuiEvent {
    MouseClick { x: u32, y: u32, button: u8 },
    MouseMove { x: u32, y: u32 },
    KeyPress { keycode: u8 },
    WindowResize { width: u32, height: u32 },
    WindowClose,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiEventMessage {
    pub window_id: usize,
    pub event: GuiEvent,
}
