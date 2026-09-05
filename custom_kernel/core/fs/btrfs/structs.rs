use core::mem::size_of;

pub const BTRFS_MAGIC: [u8; 8] = *b"_BHRfS_M";
pub const BTRFS_CSUM_SIZE: usize = 32;
pub const BTRFS_FSID_SIZE: usize = 16;
pub const BTRFS_LABEL_SIZE: usize = 256;
pub const BTRFS_UUID_SIZE: usize = 16;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Key {
    pub objectid: u64,
    pub type_: u8,
    pub offset: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub csum: [u8; BTRFS_CSUM_SIZE],
    pub fsid: [u8; BTRFS_FSID_SIZE],
    pub bytenr: u64,
    pub flags: u64,
    pub chunk_tree_uuid: [u8; BTRFS_UUID_SIZE],
    pub generation: u64,
    pub owner: u64,
    pub nritems: u32,
    pub level: u8,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub csum: [u8; BTRFS_CSUM_SIZE],
    pub fsid: [u8; BTRFS_FSID_SIZE],
    pub bytenr: u64,
    pub flags: u64,
    pub magic: [u8; 8],
    pub generation: u64,
    pub root: u64,
    pub chunk_root: u64,
    pub log_root: u64,
    pub log_root_transid: u64,
    pub total_bytes: u64,
    pub bytes_used: u64,
    pub root_dir_objectid: u64,
    pub num_devices: u64,
    pub sectorsize: u32,
    pub nodesize: u32,
    pub leafsize: u32, // Deprecated, same as nodesize usually
    pub stripesize: u32,
    pub sys_chunk_array_size: u32,
    pub chunk_root_generation: u64,
    pub compat_flags: u64,
    pub compat_ro_flags: u64,
    pub incompat_flags: u64,
    pub csum_type: u16,
    pub root_level: u8,
    pub chunk_root_level: u8,
    pub log_root_level: u8,
    pub dev_item: DevItem,
    pub label: [u8; BTRFS_LABEL_SIZE],
    // ... cache_generation, etc. (omitting some trailing fields for now)
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DevItem {
    pub devid: u64,
    pub total_bytes: u64,
    pub bytes_used: u64,
    pub io_align: u32,
    pub io_width: u32,
    pub sector_size: u32,
    pub dev_uuid: [u8; BTRFS_UUID_SIZE],
    pub fsid: [u8; BTRFS_FSID_SIZE],
}
