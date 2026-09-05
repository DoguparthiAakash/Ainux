use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::drivers::pci;
use crate::drivers::video;
use core::fmt::Write;

// AHCI Registers & Offsets
const GHC: usize = 0x04;      // Global Host Control
const IS: usize = 0x08;       // Interrupt Status
const PI: usize = 0x0C;       // Ports Implemented
const VS: usize = 0x10;       // Version

const PORT_BASE: usize = 0x100;
const PORT_STRIDE: usize = 0x80;

// Port Registers
const PORT_CLB: usize = 0x00;
const PORT_FB: usize = 0x08;
const PORT_IS: usize = 0x10;
const PORT_IE: usize = 0x14;
const PORT_CMD: usize = 0x18;
const PORT_TFD: usize = 0x20;
const PORT_SSTS: usize = 0x28;

pub struct AhciController {
    abar: usize,
    ports_implemented: u32,
}

impl AhciController {
    pub fn new(abar: usize) -> Self {
        unsafe {
            let pi = core::ptr::read_volatile((abar + PI) as *const u32);
            Self {
                abar,
                ports_implemented: pi,
            }
        }
    }

    pub fn init(&self) {
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        let _ = write!(serial, "AHCI: Initializing Controller at {:#x}\n", self.abar);

        unsafe {
            // Enable AHCI mode
            let ghc = core::ptr::read_volatile((self.abar + GHC) as *const u32);
            core::ptr::write_volatile((self.abar + GHC) as *mut u32, ghc | (1 << 31));

            for i in 0..32 {
                if (self.ports_implemented >> i) & 1 == 1 {
                    self.init_port(i);
                }
            }
        }
    }

    unsafe fn init_port(&self, port_idx: usize) {
        let port_addr = self.abar + PORT_BASE + port_idx * PORT_STRIDE;
        let ssts = core::ptr::read_volatile((port_addr + PORT_SSTS) as *const u32);
        
        let det = ssts & 0x0F;
        let ipm = (ssts >> 8) & 0x0F;

        if det != 3 || ipm != 1 {
            // Port not active or no device
            return;
        }

        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        let _ = write!(serial, "AHCI: Found SATA Device on Port {}\n", port_idx);
        
        // Command List & FIS initialization would go here for functional read/write
    }
}

pub fn init() {
    // Scan PCI for AHCI Controllers (Class 01, Subclass 06)
    // For now, we'll wait for PCI scanner to call start() or manually find it.
    // In our simplified setup, we'll try to find it in registry.
}
