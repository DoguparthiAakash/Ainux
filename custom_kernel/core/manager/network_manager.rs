use core::fmt::Write;

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "NetworkManager: Initializing networking background tasks...\n");

    crate::process::scheduler::spawn_kernel_task(network_manager_loop as u64, "network_mgr");
}

fn network_manager_loop() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "NetworkManager: Task running. Managing DHCP and link state.\n");

    let mut dhcp_configured = false;

    loop {
        let mut iface_lock = crate::net::NET_STACK.lock();
        if let Some(ref mut iface) = *iface_lock {
            // Check if DHCP is configured
            if !dhcp_configured && iface.iface.ipv4_addr().is_none() {
                // Here we would trigger a DHCP discover through smoltcp
                // For now, we simulate success
                dhcp_configured = true;
                let _ = write!(serial, "NetworkManager: DHCP negotiation simulated.\n");
            }
            
            // Poll the interface to process incoming/outgoing packets
            // let timestamp = smoltcp::time::Instant::now();
            // let _ = iface.poll(timestamp);
        }
        drop(iface_lock);

        // Yield CPU to let other tasks run. In a real system, we might wait on an IRQ event.
        unsafe { core::arch::asm!("hlt"); }
    }
}
