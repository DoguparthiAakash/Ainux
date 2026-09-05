use crate::drivers::iokit::registry::REGISTRY;
use alloc::vec::Vec;
use alloc::string::String;
use crate::alloc::string::ToString;

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
    
    devices
}

pub fn get_cpu_summary() -> String {
    let cores = crate::cpu::percpu::get_cpu_count();
    alloc::format!("ASOA Sovereign CPU: x86_64, {} Cores, 64-bit Extended Mode", cores)
}
