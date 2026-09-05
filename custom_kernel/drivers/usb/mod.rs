// src/drivers/usb/mod.rs
pub mod ehci;
pub mod manager;

pub fn init() {
    crate::drivers::video::put_str("USB: Subsystem Initializing...\n");
    // EHCI will be started by the PCI manager
}
