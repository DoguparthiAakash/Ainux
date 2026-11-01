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
        // Sleep while waiting for compositor events
        crate::process::scheduler::sleep(500);
    }
}
