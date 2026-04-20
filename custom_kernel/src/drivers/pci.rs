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

pub fn get_vendor_name(vendor_id: u16) -> &'static str {
    match vendor_id {
        0x8086 => "Intel Corporation",
        0x10DE => "NVIDIA Corporation",
        0x1002 => "AMD/ATI",
        0x10EC => "Realtek Semiconductor",
        0x1AF4 => "VirtIO / Red Hat",
        0x15AD => "VMware",
        0x80EE => "VirtualBox",
        0x1011 => "Digital Equipment Corp",
        0x1043 => "ASUSTeK Computer Inc.",
        0x1462 => "MSI (Micro-Star International)",
        0x103C => "HP / Compaq",
        0x1028 => "Dell Inc.",
        0x17AA => "Lenovo",
        _ => "Unknown Vendor",
    }
}

pub fn get_class_name(class_id: u8) -> &'static str {
    match class_id {
        0x01 => "Mass Storage Controller",
        0x02 => "Network Controller",
        0x03 => "Display Controller",
        0x04 => "Multimedia Controller",
        0x05 => "Memory Controller",
        0x06 => "Bridge Device",
        0x07 => "Simple Communication Controller",
        0x08 => "Base System Peripheral",
        0x09 => "Input Device Controller",
        0x0A => "Docking Station",
        0x0B => "Processor",
        0x0C => "Serial Bus Controller",
        0x0D => "Wireless Controller",
        _ => "Generic Device",
    }
}

pub fn get_subclass_name(class_id: u8, subclass_id: u8) -> &'static str {
    match (class_id, subclass_id) {
        (0x01, 0x01) => "IDE Interface",
        (0x01, 0x06) => "SATA Controller (AHCI)",
        (0x02, 0x00) => "Ethernet Controller",
        (0x03, 0x00) => "VGA Compatible Controller",
        (0x04, 0x01) => "Audio Controller",
        (0x04, 0x03) => "High Definition Audio Controller",
        (0x06, 0x00) => "Host Bridge",
        (0x06, 0x01) => "ISA Bridge",
        (0x06, 0x04) => "PCI-to-PCI Bridge",
        (0x0C, 0x03) => "USB Controller",
        _ => "Unknown Subclass",
    }
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
                }
            }
        }
    }
}
