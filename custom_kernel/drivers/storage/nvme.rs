use crate::drivers::pci;
use core::fmt::Write;

pub struct NvmeController {
    bar0: usize,
}

impl NvmeController {
    pub fn new(bar0: usize) -> Self {
        Self { bar0 }
    }

    pub fn init(&self) {
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        let _ = write!(serial, "NVMe: Initializing Controller (BAR0: {:#x})\n", self.bar0);
        
        unsafe {
            // Read CAP register
            let cap_low = core::ptr::read_volatile(self.bar0 as *const u32);
            let cap_high = core::ptr::read_volatile((self.bar0 + 4) as *const u32);
            let _ = write!(serial, "NVMe: Capability Register: {:#x}{:x}\n", cap_high, cap_low);
            
            // Further initialization (ASQ, ACQ, CC) would follow
        }
    }
}
