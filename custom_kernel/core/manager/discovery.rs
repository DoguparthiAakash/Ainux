use crate::drivers::iokit::registry::REGISTRY;
use alloc::vec::Vec;
use alloc::string::String;
use crate::alloc::string::ToString;
use alloc::format;

pub struct DeviceInfo {
    pub name: String,
    pub vendor_id: u16,
    pub device_id: u16,
    pub kind: String, // "PCI", "USB", "ISA"
}

pub fn get_all_devices() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    let registry = REGISTRY.lock();
    
    if let Some(root) = &registry.root {
        for entry in root.children.lock().iter() {
            let name = entry.service.get_name().to_string();
            let mut vendor = 0;
            let mut device = 0;
            
            if let Some(crate::drivers::iokit::types::IOValue::Integer(v)) = entry.service.get_property("vendor-id") {
                vendor = v as u16;
            }
            if let Some(crate::drivers::iokit::types::IOValue::Integer(d)) = entry.service.get_property("device-id") {
                device = d as u16;
            }
            
            let kind = if name.starts_with("PCI") {
                "PCI"
            } else if name.to_lowercase().contains("usb") || name.contains("ehci") || name.contains("xhci") {
                "USB"
            } else {
                "Other"
            };
            
            devices.push(DeviceInfo {
                name,
                vendor_id: vendor,
                device_id: device,
                kind: kind.to_string(),
            });
        }
    }
    
    // Legacy / Retro hardware entries (ISA / I/O mapped)
    devices.push(DeviceInfo {
        name: String::from("ISA: PC Speaker (PIT Ch 2 Port 0x42 / Gate Port 0x61)"),
        vendor_id: 0,
        device_id: 0x0061,
        kind: String::from("ISA"),
    });

    if crate::drivers::audio::sb16::is_present() {
        let sb = crate::drivers::audio::sb16::SB16.lock();
        devices.push(DeviceInfo {
            name: format!("ISA: Sound Blaster 16 & Yamaha OPL3 (DSP {:#x} / OPL3 0x388)", sb.base_port),
            vendor_id: 0x1274, // Creative Ensoniq / SB
            device_id: 0x0016,
            kind: String::from("ISA"),
        });
    }

    if crate::drivers::floppy::is_present() {
        let fdc = crate::drivers::floppy::FLOPPY.lock();
        devices.push(DeviceInfo {
            name: format!("ISA: Intel 82077AA Floppy Disk Controller ({})", fdc.drive0_type.name()),
            vendor_id: 0x8086,
            device_id: 0x8207,
            kind: String::from("ISA"),
        });
    }

    if crate::drivers::parallel::is_present() {
        devices.push(DeviceInfo {
            name: String::from("ISA: IEEE 1284 Bidirectional Parallel Port (LPT1 0x378)"),
            vendor_id: 0,
            device_id: 0x0378,
            kind: String::from("ISA"),
        });
    }

    devices.push(DeviceInfo {
        name: String::from("ISA: UART 16550A Serial Ports (COM1 0x3F8, COM2 0x2F8, COM3 0x3E8, COM4 0x2E8)"),
        vendor_id: 0,
        device_id: 0x03F8,
        kind: String::from("ISA"),
    });

    devices.push(DeviceInfo {
        name: String::from("ISA: Dual-Channel PS/2 Controller (Keyboard 0x60 / Auxiliary Mouse 0x64)"),
        vendor_id: 0,
        device_id: 0x0060,
        kind: String::from("ISA"),
    });

    devices
}

pub fn get_cpu_summary() -> String {
    let cores = crate::cpu::percpu::get_cpu_count();
    let cpu = crate::cpu::cpuid::CPU_FEATURES.lock();
    alloc::format!("CPU: {}, Architecture: {}, {} Logical Cores", cpu.brand_str(), if cpu.has_long_mode { "x86_64" } else { "x86" }, cores)
}
