use alloc::vec::Vec;

pub struct Crtc {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct Connector {
    pub id: u32,
    pub connected: bool,
    pub modes: Vec<DisplayMode>,
}

pub struct DisplayMode {
    pub hdisplay: u16,
    pub vdisplay: u16,
    pub vrefresh: u16,
}

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "KMS: Dummy Kernel Mode Setting initialized.\n");
}
