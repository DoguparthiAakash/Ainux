// src/drivers/net/ralink.rs
// Sovereign Ralink RT2860 / RT3090 PCIe Wireless Driver

use alloc::sync::Arc;
use alloc::string::String;
use spin::Mutex;
use crate::drivers::iokit::service::IOService;
use crate::drivers::iokit::types::{IOValue, IOResult};
use crate::drivers::video;
use super::ralink_regs::*;

#[derive(Debug)]
pub struct RalinkHAL {
    name: String,
    mmio_base: Mutex<u64>,
    pub mac_addr: Mutex<[u8; 6]>,
    fw_loaded: Mutex<bool>,
}

pub static GLOBAL_RALINK: Mutex<Option<Arc<RalinkHAL>>> = Mutex::new(None);

impl RalinkHAL {
    pub fn new() -> Arc<Self> {
        let driver = Arc::new(Self {
            name: String::from("Ralink RT2860/RT3090 Wireless"),
            mmio_base: Mutex::new(0),
            mac_addr: Mutex::new([0; 6]),
            fw_loaded: Mutex::new(false),
        });
        *GLOBAL_RALINK.lock() = Some(driver.clone());
        driver
    }

    fn reg_write(&self, offset: u32, val: u32) {
        let base = *self.mmio_base.lock();
        if base == 0 { return; }
        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let ptr = (base + hhdm + offset as u64) as *mut u32;
        unsafe { core::ptr::write_volatile(ptr, val); }
    }

    fn reg_read(&self, offset: u32) -> u32 {
        let base = *self.mmio_base.lock();
        if base == 0 { return 0; }
        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let ptr = (base + hhdm + offset as u64) as *const u32;
        unsafe { core::ptr::read_volatile(ptr) }
    }

    /// Reverse-Engineered Firmware Loading Sequence
    pub fn load_firmware(&self, fw_data: &[u8]) -> Result<(), String> {
        video::put_str("Ralink: Initializing Firmware Engine...\n");
        
        // 1. Reset MCU
        self.reg_write(H2M_MAILBOX_CSR, 0);
        
        // 2. Upload Data to Firmware RAM (0x2000 offset)
        // We write in 4-byte chunks
        for (i, chunk) in fw_data.chunks(4).enumerate() {
            let mut val = 0u32;
            for (j, &b) in chunk.iter().enumerate() {
                val |= (b as u32) << (j * 8);
            }
            self.reg_write(FW_IMAGE_BASE + (i * 4) as u32, val);
        }
        
        // 3. Trigger MCU Start
        self.reg_write(H2M_MAILBOX_CSR, 1);
        
        // 4. Wait for MCU Ready
        for _ in 0..10000 {
             if self.reg_read(MCU_CMD_RES) == 1 {
                 *self.fw_loaded.lock() = true;
                 video::put_str("Ralink: Firmware Active. Radio Sovereign.\n");
                 return Ok(());
             }
        }
        
        // Forced success for simulation if bin is empty
        if fw_data.is_empty() {
            video::put_str("Ralink: Firmware Engine bypassed (Simulation Mode).\n");
            return Ok(());
        }

        Err(String::from("Firmware timeout."))
    }
}

impl IOService for RalinkHAL {
    fn get_name(&self) -> &str { &self.name }
    
    fn get_property(&self, _key: &str) -> Option<IOValue> { None }
    
    fn probe(&self, provider: &Arc<dyn IOService>) -> i32 {
        if let Some(IOValue::Integer(vid)) = provider.get_property("vendor-id") {
            if let Some(IOValue::Integer(did)) = provider.get_property("device-id") {
                // Ralink Vendor = 0x1814
                if vid == 0x1814 && (did == 0x0601 || did == 0x3090) {
                    return 100;
                }
                // MediaTek Vendor = 0x14C3 (MT7630, MT7610, etc)
                if vid == 0x14c3 && (did == 0x7630 || did == 0x7610 || did == 0x0601) {
                    return 100;
                }
            }
        }
        0
    }
    
    fn start(&self, provider: &Arc<dyn IOService>) -> IOResult<()> {
        let bar0 = match provider.get_property("bar0") {
            Some(IOValue::Integer(b)) => b as u64,
            _ => 0,
        };
        let mmio_addr = bar0 & !0xF;
        *self.mmio_base.lock() = mmio_addr;
        
        video::put_str(&format!("Ralink: Hardware Found @ {:#x}\n", mmio_addr));
        
        // Read Silicon Version
        let sid = self.reg_read(MAC_CSR0);
        video::put_str(&format!("Ralink: Silicon ID: {:#x}\n", sid));
        
        // Read Hardware MAC
        let id0 = self.reg_read(MAC_ADDR_DW0);
        let id1 = self.reg_read(MAC_ADDR_DW1);
        let mut mac = [0u8; 6];
        mac[0] = (id0 & 0xFF) as u8;
        mac[1] = ((id0 >> 8) & 0xFF) as u8;
        mac[2] = ((id0 >> 16) & 0xFF) as u8;
        mac[3] = ((id0 >> 24) & 0xFF) as u8;
        mac[4] = (id1 & 0xFF) as u8;
        mac[5] = ((id1 >> 8) & 0xFF) as u8;
        *self.mac_addr.lock() = mac;
        
        video::put_str(&format!("Ralink: MAC Address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}\n", 
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]));

        // In a real scenario, we would load the 'rt2860.bin' here.
        // For now, we initialize the engine.
        let _ = self.load_firmware(&[]);
        
        Ok(())
    }
    
    fn stop(&self) {
        video::put_str("Ralink: Radio Hibernate.\n");
    }
}
