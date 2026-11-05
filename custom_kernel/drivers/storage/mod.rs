pub mod ahci;
pub mod nvme;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

#[derive(Clone, Debug)]
pub struct UnifiedDiskInfo {
    pub name: String,
    pub bus_type: String,
    pub model: String,
    pub capacity_mb: u64,
    pub total_sectors: u64,
    pub sector_size: u32,
    pub status: String,
}

pub fn get_all_disks() -> Vec<UnifiedDiskInfo> {
    let mut disks = Vec::new();

    // 1. Check ATA / IDE (Legacy PATA)
    let mut ident_buf = [0u16; 256];
    if crate::drivers::ata::identify_buffer(&mut ident_buf) {
        // Read model from words 27..46
        let mut model_bytes = [0u8; 40];
        for i in 0..20 {
            let word = ident_buf[27 + i];
            model_bytes[i * 2] = (word >> 8) as u8;
            model_bytes[i * 2 + 1] = (word & 0xFF) as u8;
        }
        let model_str = core::str::from_utf8(&model_bytes).unwrap_or("PATA Hard Disk").trim();

        // LBA28 sectors from words 60..61
        let sectors = ((ident_buf[61] as u32) << 16) | (ident_buf[60] as u32);
        let cap_mb = ((sectors as u64) * 512) / (1024 * 1024);

        disks.push(UnifiedDiskInfo {
            name: String::from("hda"),
            bus_type: String::from("PATA / IDE (Legacy PIO)"),
            model: String::from(model_str),
            capacity_mb: cap_mb,
            total_sectors: sectors as u64,
            sector_size: 512,
            status: String::from("Mounted (Boot Root)"),
        });
    }

    // 2. Check SATA AHCI (Modern Serial ATA)
    if let Some(ctrl) = crate::drivers::storage::ahci::ACTIVE_AHCI.lock().as_ref() {
        for drive in &ctrl.drives {
            disks.push(UnifiedDiskInfo {
                name: format!("sda{}", drive.port),
                bus_type: format!("SATA Revision 3.x AHCI (Port {})", drive.port),
                model: drive.model.clone(),
                capacity_mb: drive.capacity_mb,
                total_sectors: drive.total_sectors,
                sector_size: 512,
                status: String::from("Online"),
            });
        }
    }

    // 3. Check NVMe (Modern PCIe SSD)
    if let Some(ctrl) = crate::drivers::storage::nvme::ACTIVE_NVME.lock().as_ref() {
        if let Some(info) = &ctrl.info {
            disks.push(UnifiedDiskInfo {
                name: String::from("nvme0n1"),
                bus_type: String::from("PCIe Gen3/4/5 NVM Express"),
                model: info.model.clone(),
                capacity_mb: info.capacity_mb,
                total_sectors: info.total_sectors,
                sector_size: info.sector_size,
                status: String::from("Online"),
            });
        }
    }

    // 4. Check VirtIO-Blk (Virtual Block Device)
    if let Some(dev) = crate::drivers::virtio::virtio_blk::ACTIVE_VIRTIO_BLK.lock().as_ref() {
        disks.push(UnifiedDiskInfo {
            name: String::from("vda"),
            bus_type: String::from("VirtIO PCI Virtual Disk"),
            model: String::from("QEMU/KVM/Limbo VirtIO-Blk"),
            capacity_mb: dev.capacity_mb,
            total_sectors: dev.capacity_sectors,
            sector_size: 512,
            status: String::from("Online"),
        });
    }

    // 5. Check Floppy Controller (Legacy 82077AA)
    let fdc = crate::drivers::floppy::FLOPPY.lock();
    if fdc.drive0_type != crate::drivers::floppy::FloppyDriveType::None {
        disks.push(UnifiedDiskInfo {
            name: String::from("fd0"),
            bus_type: String::from("Legacy FDC 82077AA (ISA)"),
            model: String::from(fdc.drive0_type.name()),
            capacity_mb: 1, // 1.44 MB
            total_sectors: crate::drivers::floppy::FLOPPY_TOTAL_SECTORS as u64,
            sector_size: 512,
            status: if fdc.initialized { String::from("Ready") } else { String::from("No Media") },
        });
    }
    if fdc.drive1_type != crate::drivers::floppy::FloppyDriveType::None {
        disks.push(UnifiedDiskInfo {
            name: String::from("fd1"),
            bus_type: String::from("Legacy FDC 82077AA (ISA)"),
            model: String::from(fdc.drive1_type.name()),
            capacity_mb: 1,
            total_sectors: crate::drivers::floppy::FLOPPY_TOTAL_SECTORS as u64,
            sector_size: 512,
            status: if fdc.initialized { String::from("Ready") } else { String::from("No Media") },
        });
    }

    disks
}
