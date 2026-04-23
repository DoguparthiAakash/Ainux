// src/drivers/usb/manager.rs
// Ainux USB Device Manager

use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
use crate::drivers::iokit::service::IOService;
use crate::drivers::iokit::types::{IOValue, IOResult};

#[derive(Debug, Clone)]
pub struct USBDevice {
    pub address: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub class: u8,
}

impl IOService for USBDevice {
    fn get_name(&self) -> &str { "USB Generic Device" }
    
    fn get_property(&self, key: &str) -> Option<IOValue> {
        match key {
            "vendor-id" => Some(IOValue::Integer(self.vendor_id as i64)),
            "device-id" => Some(IOValue::Integer(self.product_id as i64)),
            "usb-address" => Some(IOValue::Integer(self.address as i64)),
            _ => None,
        }
    }
    
    fn probe(&self, _provider: &Arc<dyn IOService>) -> i32 { 0 }
    fn start(&self, _provider: &Arc<dyn IOService>) -> IOResult<()> { Ok(()) }
    fn stop(&self) {}
}

pub struct USBManager {
    pub devices: Vec<Arc<USBDevice>>,
}

pub static MANAGER: Mutex<USBManager> = Mutex::new(USBManager {
    devices: Vec::new(),
});

pub fn register_device(vendor: u16, product: u16, class: u8) {
    let mut mgr = MANAGER.lock();
    let addr = (mgr.devices.len() + 1) as u8;
    
    let usb_dev = Arc::new(USBDevice {
        address: addr,
        vendor_id: vendor,
        product_id: product,
        class,
    });
    
    mgr.devices.push(usb_dev.clone());
    
    crate::drivers::video::put_str(&alloc::format!("USB: Device Connected: [{:04x}:{:04x}] as Address {}\n", 
        vendor, product, addr));
    
    // Attempt to match with a Native Driver (MediaTek MT7601U)
    let mt_driver = crate::drivers::net::mt7601u::MT7601U::new();
    let provider: Arc<dyn IOService> = usb_dev;
    if mt_driver.probe(&provider) > 0 {
         let _ = mt_driver.start(&provider);
    }
}
