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
pub fn pci_config_read(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = (1 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | ((offset as u32) & 0xFC);
    outl(CONFIG_ADDRESS, address);
    inl(CONFIG_DATA)
}

pub fn pci_config_write(bus: u8, slot: u8, func: u8, offset: u8, val: u32) {
    let address = (1 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | ((offset as u32) & 0xFC);
    outl(CONFIG_ADDRESS, address);
    outl(CONFIG_DATA, val);
}

// ── Typed accessors used by the KPI layer ────────────────────────────────────

pub fn config_read32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    pci_config_read(bus, slot, func, offset)
}

pub fn config_read16(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
    let val = pci_config_read(bus, slot, func, offset & !2);
    let shift = (offset & 2) * 8;
    (val >> shift) as u16
}

pub fn config_read8(bus: u8, slot: u8, func: u8, offset: u8) -> u8 {
    let val = pci_config_read(bus, slot, func, offset & !3);
    let shift = (offset & 3) * 8;
    (val >> shift) as u8
}

pub fn config_write32(bus: u8, slot: u8, func: u8, offset: u8, val: u32) {
    pci_config_write(bus, slot, func, offset, val);
}

pub fn config_write16(bus: u8, slot: u8, func: u8, offset: u8, val: u16) {
    let aligned = offset & !2;
    let shift = (offset & 2) * 8;
    let old = pci_config_read(bus, slot, func, aligned);
    let mask = !(0xFFFFu32 << shift);
    let new_val = (old & mask) | ((val as u32) << shift);
    pci_config_write(bus, slot, func, aligned, new_val);
}

pub fn config_write8(bus: u8, slot: u8, func: u8, offset: u8, val: u8) {
    let aligned = offset & !3;
    let shift = (offset & 3) * 8;
    let old = pci_config_read(bus, slot, func, aligned);
    let mask = !(0xFFu32 << shift);
    let new_val = (old & mask) | ((val as u32) << shift);
    pci_config_write(bus, slot, func, aligned, new_val);
}

pub fn get_pci_class_desc(class: u8, subclass: u8, prog_if: u8) -> &'static str {
    match (class, subclass) {
        (0x01, 0x00) => "SCSI Bus Controller",
        (0x01, 0x01) => "IDE / PATA Controller (Legacy)",
        (0x01, 0x06) => "SATA Controller (AHCI 3.x)",
        (0x01, 0x08) => "NVMe PCIe SSD Controller",
        (0x02, 0x00) => "Ethernet Controller (Gigabit/Fast)",
        (0x02, 0x80) => "Network Controller (WiFi/Other)",
        (0x03, 0x00) => "VGA Compatible Display Controller",
        (0x04, 0x01) => "Audio Controller (AC'97)",
        (0x04, 0x03) => "Intel High Definition Audio",
        (0x06, 0x00) => "Host Bridge (CPU/PCI)",
        (0x06, 0x01) => "ISA Bridge (Legacy Ports)",
        (0x06, 0x04) => "PCI-to-PCI Bridge",
        (0x0C, 0x03) => match prog_if {
            0x00 => "UHCI USB 1.1 Host Controller",
            0x10 => "OHCI USB 1.1 Host Controller",
            0x20 => "EHCI USB 2.0 Host Controller",
            0x30 => "xHCI USB 3.x SuperSpeed Controller",
            _ => "USB Host Controller",
        },
        _ => "PCI Device",
    }
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
    prog_if: u8,
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
            "subclass-id" => Some(IOValue::Integer(self.subclass_id as i64)),
            "prog-if" => Some(IOValue::Integer(self.prog_if as i64)),
            "bar0" => Some(IOValue::Integer(self.bar0 as i64)),
            "irq" => Some(IOValue::Integer(self.irq_line as i64)),
            _ => None,
        }
    }
    
    fn probe(&self, _provider: &Arc<dyn IOService>) -> i32 { 0 }
    fn start(&self, _provider: &Arc<dyn IOService>) -> IOResult<()> { Ok(()) }
    fn stop(&self) {}
}

pub fn init() {
    crate::drivers::video::put_str("PCI: Scanning Bus Topology...\n");
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
                    let prog_if = (class_rev >> 8) as u8;
                    
                    let bar0 = pci_config_read(bus, slot, 0, 0x10);
                    let bar5 = pci_config_read(bus, slot, 0, 0x24);
                    let intr = pci_config_read(bus, slot, 0, 0x3C);
                    let irq_line = (intr & 0xFF) as u8;
                    
                    // Enable Bus Mastering, Memory Space, and I/O Space in Command Register
                    let cmd_status = pci_config_read(bus, slot, 0, 0x04);
                    pci_config_write(bus, slot, 0, 0x04, cmd_status | 0x07);
                    
                    let desc = get_pci_class_desc(class_id, subclass_id, prog_if);
                    let name = format!("PCI {:02x}:{:02x}.0 ({})", bus, slot, desc);
                    crate::drivers::video::put_str(&name);
                    crate::drivers::video::put_str(&format!(" [{:04x}:{:04x}]\n", vendor, device_id));
                    let _ = write!(
                        crate::drivers::serial::SERIAL.lock(),
                        "PCI {:02x}:{:02x}.0 [{:04x}:{:04x}] {} (Class {:02x}:{:02x} IF {:02x})\n",
                        bus, slot, vendor, device_id, desc, class_id, subclass_id, prog_if
                    );

                    let pci_dev = Arc::new(PCIDevice {
                        name,
                        bus: bus as u8,
                        slot: slot as u8,
                        func: 0,
                        vendor_id: vendor,
                        device_id,
                        class_id,
                        subclass_id,
                        prog_if,
                        bar0,
                        irq_line,
                    });
                    
                    let entry = IORegistryEntry::new(pci_dev.clone());
                    IORegistryEntry::add_child(&root_entry, &entry);
                    
                    // Audio Device Detection (AC97: Intel 82801AA or PCI class 0x04 subclass 0x01)
                    if (pci_dev.vendor_id == 0x8086 && pci_dev.device_id == 0x2415)
                        || (pci_dev.class_id == 0x04 && pci_dev.subclass_id == 0x01) {
                        crate::drivers::audio::ac97::init_device(pci_dev.bus, pci_dev.slot, pci_dev.func);
                    }

                    // NVIDIA GPU Detection
                    if pci_dev.vendor_id == 0x10DE {
                        crate::drivers::drm::nvidia::init_device(pci_dev.bus, pci_dev.slot, pci_dev.func, pci_dev.device_id, pci_dev.bar0);
                    }

                    // SATA Controller (AHCI: Class 0x01, Subclass 0x06)
                    if pci_dev.class_id == 0x01 && pci_dev.subclass_id == 0x06 {
                        crate::drivers::storage::ahci::detect_and_init(pci_dev.bus, pci_dev.slot, pci_dev.func, bar5);
                    }

                    // NVMe Controller (PCIe SSD: Class 0x01, Subclass 0x08)
                    if pci_dev.class_id == 0x01 && pci_dev.subclass_id == 0x08 {
                        crate::drivers::storage::nvme::detect_and_init(pci_dev.bus, pci_dev.slot, pci_dev.func, pci_dev.bar0);
                    }

                    // USB 3.x xHCI Controller (Class 0x0C, Subclass 0x03, Prog-IF 0x30)
                    if pci_dev.class_id == 0x0C && pci_dev.subclass_id == 0x03 && prog_if == 0x30 {
                        crate::drivers::usb::xhci::detect_and_init(pci_dev.bus, pci_dev.slot, pci_dev.func, pci_dev.bar0);
                    }

                    // VirtIO Virtual Devices (Vendor 0x1AF4)
                    if pci_dev.vendor_id == 0x1AF4 {
                        if pci_dev.device_id == 0x1000 || pci_dev.device_id == 0x1041 {
                            crate::drivers::virtio::virtio_net::detect_and_init(pci_dev.bus, pci_dev.slot, pci_dev.func, pci_dev.bar0);
                        } else if pci_dev.device_id == 0x1001 || pci_dev.device_id == 0x1042 {
                            crate::drivers::virtio::virtio_blk::detect_and_init(pci_dev.bus, pci_dev.slot, pci_dev.func, pci_dev.bar0);
                        }
                    }

                    // Match Network Drivers (RTL8139, Atheros, Ralink)
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

                    // EHCI USB 2.0 Driver
                    let ehci_driver = crate::drivers::usb::ehci::EHCIController::new();
                    if ehci_driver.probe(&entry.service) > 0 {
                         let _ = ehci_driver.start(&entry.service);
                    }
                }
            }
        }
    }
    crate::drivers::net::e1000::detect_and_init();
}
