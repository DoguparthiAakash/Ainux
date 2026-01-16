pub mod structs;

use crate::drivers::ata;
use structs::{Superblock, BTRFS_MAGIC};
use core::mem;

pub fn read_superblock() -> Result<Superblock, &'static str> {
    // Btrfs superblock is at 64KB (Sector 128 if 512b sectors)
    let sector = 65536 / 512;
    // We need to read enough for the superblock. sizeof(Superblock) is large (~4K reserved space).
    // Let's read 8 sectors (4KB).
    let mut buffer = [0u16; 2048]; // 2048 * 2 = 4096 bytes
    
    if !ata::read_sectors(&mut buffer, sector as u32, 8) {
        return Err("Disk Read Error");
    }
    
    unsafe {
        let ptr = buffer.as_ptr() as *const Superblock;
        let sb = *ptr;
        
        if sb.magic != BTRFS_MAGIC {
            // Debug print magic if possible
            return Err("Invalid Magic");
        }
        
        // TODO: Verify Checksum (CRC32C)
        
        Ok(sb)
    }
}
