use core::fmt::Write;

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "DesktopManager: Initializing GUI Compositor sub-system...\n");

    // In a microkernel or fully componentized system, the DesktopManager
    // would request the Framebuffer from the DRM system.
    let _ = write!(serial, "DesktopManager: Probing DRM subsystem for primary display...\n");

    // Spawn a kernel thread for the Desktop Manager to listen to window draw requests
    crate::process::scheduler::spawn_kernel_task(desktop_manager_loop as u64, "desktop_mgr");
}

fn desktop_manager_loop() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "DesktopManager: Task running. Waiting for window compositor events.\n");

    loop {
        // Here we would block on an IPC channel or socket waiting for GUI apps
        // to send their rendering buffers.
        
        // Yield CPU
        unsafe { core::arch::asm!("hlt"); }
    }
}
