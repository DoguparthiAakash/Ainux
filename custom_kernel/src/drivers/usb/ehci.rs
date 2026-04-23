// src/drivers/usb/ehci.rs
// Ainux EHCI (USB 2.0) Host Controller Driver

use alloc::sync::Arc;
use alloc::string::String;
use spin::Mutex;
use crate::drivers::iokit::service::IOService;
use crate::drivers::iokit::types::{IOValue, IOResult};
use crate::drivers::video;

// EHCI Operational Registers
pub const USBCMD: u32 = 0x00;
pub const USBSTS: u32 = 0x04;
pub const USBINTR: u32 = 0x08;
pub const ASYNCLISTADDR: u32 = 0x18;
pub const CONFIGFLAG: u32 = 0x40;
pub const PORTSC: u32 = 0x44;

#[derive(Debug)]
pub struct EHCIController {
    name: String,
    cap_base: Mutex<u64>,
    op_base: Mutex<u64>,
}

impl EHCIController {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            name: String::from("Intel EHCI USB 2.0 Controller"),
            cap_base: Mutex::new(0),
            op_base: Mutex::new(0),
        })
    }

    fn op_write(&self, offset: u32, val: u32) {
        let base = *self.op_base.lock();
        if base == 0 { return; }
        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let ptr = (base + hhdm + offset as u64) as *mut u32;
        unsafe { core::ptr::write_volatile(ptr, val); }
    }

    fn op_read(&self, offset: u32) -> u32 {
        let base = *self.op_base.lock();
        if base == 0 { return 0; }
        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let ptr = (base + hhdm + offset as u64) as *const u32;
        unsafe { core::ptr::read_volatile(ptr) }
    }

    fn cap_read8(&self, offset: u32) -> u8 {
        let base = *self.cap_base.lock();
        if base == 0 { return 0; }
        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let ptr = (base + hhdm + offset as u64) as *const u8;
        unsafe { core::ptr::read_volatile(ptr) }
    }
}

impl IOService for EHCIController {
    fn get_name(&self) -> &str { &self.name }
    
    fn get_property(&self, _key: &str) -> Option<IOValue> { None }
    
    fn probe(&self, provider: &Arc<dyn IOService>) -> i32 {
        if let Some(IOValue::Integer(class)) = provider.get_property("class-id") {
            // USB Controller Class = 0x0C, Subclass = 0x03, Interface = 0x20 (EHCI)
            if class == 0x0C {
                return 100;
            }
        }
        0
    }
    
    fn start(&self, provider: &Arc<dyn IOService>) -> IOResult<()> {
        let bar0 = match provider.get_property("bar0") {
            Some(IOValue::Integer(b)) => b as u64,
            _ => 0,
        };
        let cap_addr = bar0 & !0xF;
        *self.cap_base.lock() = cap_addr;
        
        let cap_length = self.cap_read8(0x00) as u64;
        *self.op_base.lock() = cap_addr + cap_length;
        
        video::put_str(&format!("USB: EHCI Controller @ {:#x} (OpBase: {:#x})\n", cap_addr, cap_addr + cap_length));
        
        // 1. Reset Controller
        self.op_write(USBCMD, 0x02); // HCRESET
        while self.op_read(USBCMD) & 0x02 != 0 { core::hint::spin_loop(); }
        
        // 2. Clear Interrupts and Status
        self.op_write(USBSTS, 0x3F);
        
        // 3. Enable Controller
        self.op_write(CONFIGFLAG, 1);
        self.op_write(USBCMD, 0x01 | (0x40 << 16)); // Run + Interrupt Threshold
        
        video::put_str("USB: EHCI Operational. Seeking Root Hub Devices...\n");
        
        Ok(())
    }
    
    fn stop(&self) {
        self.op_write(USBCMD, 0); // Stop
    }
}
