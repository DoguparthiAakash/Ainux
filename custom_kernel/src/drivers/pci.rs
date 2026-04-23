use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::format;
use core::fmt::Write;
use crate::drivers::iokit::service::IOService;
use crate::drivers::iokit::registry::{self, IORegistryEntry};
use crate::drivers::iokit::types::{IOValue, IOResult};

// Ports
const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

// Helper to write to IO ports (using asm)
fn outl(port: u16, val: u32) {
    unsafe { core::arch::asm!("out dx, eax", in("dx") port, in("eax") val, options(nostack, preserves_flags)); }
}

fn inl(port: u16) -> u32 {
    let mut val: u32;
    unsafe { core::arch::asm!("in eax, dx", out("eax") val, in("dx") port, options(nostack, preserves_flags)); }
    val
}

// PCI Access
fn pci_config_read(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = (1 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | ((offset as u32) & 0xFC);
    outl(CONFIG_ADDRESS, address);
    inl(CONFIG_DATA)
}

// PCI Device Representation
#[derive(Debug)]
pub struct PCIDevice {
    name: alloc::string::String,
    bus: u8,
    slot: u8,
    func: u8,
    vendor_id: u16,
    device_id: u16,
    class_id: u8,
    subclass_id: u8,
    bar0: u32,
    irq_line: u8,
}

impl IOService for PCIDevice {
    fn get_name(&self) -> &str { &self.name }
    
    fn get_property(&self, key: &str) -> Option<IOValue> {
        match key {
            "vendor-id" => Some(IOValue::Integer(self.vendor_id as i64)),
            "device-id" => Some(IOValue::Integer(self.device_id as i64)),
            "class-id" => Some(IOValue::Integer(self.class_id as i64)),
            "bar0" => Some(IOValue::Integer(self.bar0 as i64)),
            "irq" => Some(IOValue::Integer(self.irq_line as i64)),
            _ => None,
        }
    }
    
    fn probe(&self, _provider: &Arc<dyn IOService>) -> i32 { 0 } // PCI Device is a Provider, usually doesn't probe others?
    fn start(&self, _provider: &Arc<dyn IOService>) -> IOResult<()> { Ok(()) }
    fn stop(&self) {}
}

pub fn init() {
    crate::drivers::video::put_str("PCI: Scanning Bus...\n");
    let root = registry::REGISTRY.lock().root.clone();
    
    if let Some(root_entry) = root {
        for bus in 0..=255 {
            for slot in 0..32 {
                let vendor = (pci_config_read(bus, slot, 0, 0) & 0xFFFF) as u16;
                if vendor != 0xFFFF {
                    // Device Exists
                    let device_id = (pci_config_read(bus, slot, 0, 0) >> 16) as u16;
                    let class_rev = pci_config_read(bus, slot, 0, 0x08);
                    let class_id = (class_rev >> 24) as u8;
                    let subclass_id = (class_rev >> 16) as u8;
                    
                    let bar0 = pci_config_read(bus, slot, 0, 0x10);
                    let intr = pci_config_read(bus, slot, 0, 0x3C);
                    let irq_line = (intr & 0xFF) as u8;
                    
                    // Create IOService
                    let name = format!("PCI {:02x}:{:02x}.0", bus, slot);
                    crate::drivers::video::put_str(&name);
                    crate::drivers::video::put_str(&format!(" [{:04x}:{:04x}]\n", vendor, device_id));

                    let pci_dev = Arc::new(PCIDevice {
                        name,
                        bus: bus as u8,
                        slot: slot as u8,
                        func: 0,
                        vendor_id: vendor,
                        device_id,
                        class_id,
                        subclass_id,
                        bar0,
                        irq_line,
                    });
                    
                    let entry = IORegistryEntry::new(pci_dev);
                    IORegistryEntry::add_child(&root_entry, &entry);
                    
                    // Match Drivers (Simulated matching loop)
                    let rtl_driver = Arc::new(crate::drivers::net::rtl8139::RTL8139::new());
                    if rtl_driver.probe(&entry.service) > 0 {
                         let _ = rtl_driver.start(&entry.service);
                    }

                    let ath_driver = crate::drivers::net::atheros::AtherosHAL::new();
                    if ath_driver.probe(&entry.service) > 0 {
                         let _ = ath_driver.start(&entry.service);
                    }

                    let ralink_driver = crate::drivers::net::ralink::RalinkHAL::new();
                    if ralink_driver.probe(&entry.service) > 0 {
                         let _ = ralink_driver.start(&entry.service);
                    }

                    let ehci_driver = crate::drivers::usb::ehci::EHCIController::new();
                    if ehci_driver.probe(&entry.service) > 0 {
                         let _ = ehci_driver.start(&entry.service);
                    }
                }
            }
        }
    }
}
