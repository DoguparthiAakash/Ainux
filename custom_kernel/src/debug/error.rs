use crate::drivers::video;
use core::fmt::Write;

#[derive(Debug, Clone, Copy)]
pub enum KernelError {
    Generic,
    OutOfMemory,
    PageFault,
    InvalidSyscall,
    IOError,
    ProcessNotFound,
    PermissionDenied,
}

pub fn report_error(err: KernelError, msg: &str) {
    // In strict development mode, we might want to panic.
    // In production, we log and potentially kill the offending process.
    
    // For now, log to Serial and Screen in Red.
    if let Some(mut serial) = crate::drivers::serial::SERIAL.try_lock() {
        let _ = write!(serial, "[KERNEL ERROR] {:?}: {}\n", err, msg);
    }
    
    video::put_str("\n[KERNEL ERROR] ");
    // TODO: video::set_color(RED);
    video::put_str(msg);
    video::put_char('\n');
}
