pub mod scheduler;

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "Microkernel Manager Initialized.\n");
    
    // The microkernel will eventually orchestrate the sibling monolithic kernels
    // (Ainux and BSD Hybrid)
    scheduler::init();
}
