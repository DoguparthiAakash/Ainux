use crate::drivers::ata;
use core::mem::transmute;

// Superblock is at offset 1024 (Block 0 if blocksize=1024, or inside Block 0 if larger).
// It's always 1024 bytes into the volume.

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub inodes_count: u32,
    pub blocks_count_lo: u32,
    pub r_blocks_count_lo: u32,
    pub free_blocks_count_lo: u32,
    pub free_inodes_count: u32,
    pub first_data_block: u32,
    pub log_block_size: u32,
    pub log_cluster_size: u32,
    pub blocks_per_group: u32,
    pub clusters_per_group: u32,
    pub inodes_per_group: u32,
    pub mtime: u32,
    pub wtime: u32,
    pub mnt_count: u16,
    pub max_mnt_count: u16,
    pub magic: u16, // Offset 0x38 (56)
    pub state: u16,
    pub errors: u16,
    pub minor_rev_level: u16,
    pub lastcheck: u32,
    pub checkinterval: u32,
    pub creator_os: u32,
    pub rev_level: u32,
    pub def_resuid: u16,
    pub def_resgid: u16,
    // ... more fields ...
}

pub fn parse_superblock() -> Option<Superblock> {
    // Superblock starts at byte 1024.
    // LBA 0 = 0-511
    // LBA 1 = 512-1023
    // LBA 2 = 1024-1535.
    
    // We need to read enough to cover the struct (sizeof Superblock is usually 1024 bytes, but minimal is 84).
    // Let's read 2 sectors (1024 bytes) starting at LBA 2.
    
    let mut buffer = [0u16; 512]; // 512 u16 = 1024 bytes
    ata::read_sectors(&mut buffer, 2, 2);
    
    unsafe {
        let ptr = buffer.as_ptr() as *const Superblock;
        let sb = *ptr;
        
        if sb.magic == 0xEF53 {
             return Some(sb);
        }
    }
    
    None
}
