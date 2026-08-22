use crate::drivers::ata;
use crate::drivers::video;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy)]
pub struct MbrPartition {
    pub status: u8,
    pub partition_type: u8,
    pub lba_start: u32,
    pub num_sectors: u32,
}

pub fn parse_mbr() -> Vec<MbrPartition> {
    let mut partitions = Vec::new();
    let mut buf = [0u16; 256];
    
    // Read LBA 0
    if !ata::read_sectors(&mut buf, 0, 1) {
        unsafe { video::put_str("MBR: Failed to read LBA 0\n"); }
        return partitions;
    }
    
    // Check boot signature 0xAA55
    if buf[255] != 0xAA55 {
        unsafe { video::put_str("MBR: Invalid boot signature\n"); }
        return partitions;
    }
    
    // MBR partition table starts at byte 446 (word 223)
    let p_table = &buf[223..223 + 32]; // 4 entries * 16 bytes = 64 bytes = 32 words
    
    for i in 0..4 {
        let offset = i * 8; // 8 words per entry
        
        let status = (p_table[offset] & 0xFF) as u8;
        let p_type = (p_table[offset + 2] & 0xFF) as u8;
        
        let lba_start = (p_table[offset + 4] as u32) | ((p_table[offset + 5] as u32) << 16);
        let num_sectors = (p_table[offset + 6] as u32) | ((p_table[offset + 7] as u32) << 16);
        
        if p_type != 0 {
            partitions.push(MbrPartition {
                status,
                partition_type: p_type,
                lba_start,
                num_sectors,
            });
            
            unsafe {
                video::put_str("MBR: Found partition type 0x");
                crate::drivers::serial::print_hex_8(p_type);
                video::put_str(" Start: ");
                crate::drivers::serial::print_hex_32(lba_start);
                video::put_str(" Sectors: ");
                crate::drivers::serial::print_hex_32(num_sectors);
                video::put_str("\n");
            }
        }
    }
    
    partitions
}
