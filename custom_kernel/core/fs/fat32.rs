use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::mem;
use crate::fs::vfs::{FileSystem, Inode, FileStat, FileType, VfsResult, VfsError, FileHandle};
use crate::drivers::ata;
use crate::drivers::video;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32Bpb {
    pub jmp: [u8; 3],
    pub oem: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub dir_entries: u16,
    pub total_sectors_16: u16,
    pub media: u8,
    pub fat_size_16: u16,
    pub sectors_per_track: u16,
    pub heads: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
    pub fat_size_32: u32,
    pub ext_flags: u16,
    pub fs_version: u16,
    pub root_cluster: u32,
    pub fs_info: u16,
    pub backup_boot_sector: u16,
    pub reserved: [u8; 12],
    pub drive_number: u8,
    pub reserved1: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub fs_type: [u8; 8],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct FatDirEntry {
    pub name: [u8; 11],
    pub attr: u8,
    pub ntres: u8,
    pub crt_time_tenth: u8,
    pub crt_time: u16,
    pub crt_date: u16,
    pub lst_acc_date: u16,
    pub fst_clus_hi: u16,
    pub wrt_time: u16,
    pub wrt_date: u16,
    pub fst_clus_lo: u16,
    pub file_size: u32,
}

const ATTR_READ_ONLY: u8 = 0x01;
const ATTR_HIDDEN: u8 = 0x02;
const ATTR_SYSTEM: u8 = 0x04;
const ATTR_VOLUME_ID: u8 = 0x08;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_ARCHIVE: u8 = 0x20;
const ATTR_LONG_NAME: u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID;

#[derive(Debug)]
pub struct Fat32FileSystem {
    partition_lba: u32,
    bpb: Fat32Bpb,
    fat_start_lba: u32,
    data_start_lba: u32,
}

impl Fat32FileSystem {
    pub fn new(partition_lba: u32) -> Option<Arc<Self>> {
        let mut buf = [0u16; 256];
        if !ata::read_sectors(&mut buf, partition_lba, 1) {
            unsafe { video::put_str("FAT32: Failed to read BPB\n"); }
            return None;
        }

        let bpb: Fat32Bpb = unsafe {
            let buf_u8 = core::slice::from_raw_parts(buf.as_ptr() as *const u8, 512);
            core::ptr::read_unaligned(buf_u8.as_ptr() as *const Fat32Bpb)
        };

        if bpb.bytes_per_sector != 512 {
            unsafe { video::put_str("FAT32: Unsupported sector size\n"); }
            return None;
        }

        let fat_start_lba = partition_lba + bpb.reserved_sectors as u32;
        let fat_size = if bpb.fat_size_16 != 0 { bpb.fat_size_16 as u32 } else { bpb.fat_size_32 };
        let root_dir_sectors = ((bpb.dir_entries as u32 * 32) + (bpb.bytes_per_sector as u32 - 1)) / bpb.bytes_per_sector as u32;
        let data_start_lba = fat_start_lba + (bpb.fat_count as u32 * fat_size) + root_dir_sectors;

        Some(Arc::new(Fat32FileSystem {
            partition_lba,
            bpb,
            fat_start_lba,
            data_start_lba,
        }))
    }

    fn cluster_to_lba(&self, cluster: u32) -> u32 {
        if cluster < 2 { return 0; }
        self.data_start_lba + ((cluster - 2) * self.bpb.sectors_per_cluster as u32)
    }

    fn read_cluster(&self, cluster: u32, buf: &mut [u16]) -> bool {
        let lba = self.cluster_to_lba(cluster);
        if lba == 0 { return false; }
        ata::read_sectors(buf, lba, self.bpb.sectors_per_cluster)
    }

    fn next_cluster(&self, cluster: u32) -> Option<u32> {
        let fat_offset = cluster * 4;
        let fat_sector = self.fat_start_lba + (fat_offset / self.bpb.bytes_per_sector as u32);
        let ent_offset = (fat_offset % self.bpb.bytes_per_sector as u32) as usize;
        
        let mut buf = [0u16; 256];
        if ata::read_sectors(&mut buf, fat_sector, 1) {
            let buf_u8 = unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u8, 512) };
            let mut next = 0;
            next |= buf_u8[ent_offset] as u32;
            next |= (buf_u8[ent_offset+1] as u32) << 8;
            next |= (buf_u8[ent_offset+2] as u32) << 16;
            next |= (buf_u8[ent_offset+3] as u32) << 24;
            next &= 0x0FFFFFFF;
            if next >= 0x0FFFFFF8 || next == 0 {
                None
            } else {
                Some(next)
            }
        } else {
            None
        }
    }
}

impl FileSystem for Fat32FileSystem {
    fn root_inode(&self) -> Arc<dyn Inode> {
        Arc::new(Fat32Inode {
            fs: Arc::new(Fat32FileSystem {
                partition_lba: self.partition_lba,
                bpb: self.bpb,
                fat_start_lba: self.fat_start_lba,
                data_start_lba: self.data_start_lba,
            }),
            cluster: self.bpb.root_cluster,
            is_dir: true,
            size: 0,
            name: String::from("/"),
        })
    }
}

#[derive(Debug)]
pub struct Fat32Inode {
    fs: Arc<Fat32FileSystem>,
    cluster: u32,
    is_dir: bool,
    size: u32,
    name: String,
}

impl crate::object::KernelObject for Fat32Inode {
    fn object_type(&self) -> &'static str {
        if self.is_dir { "Directory" } else { "File" }
    }
    
    fn name(&self) -> String {
        self.name.clone()
    }
    
    fn id(&self) -> usize {
        self.cluster as usize
    }
    
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &str> {
        Err("Not supported for FAT32 Inodes")
    }
    
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &str> {
        Err("Not supported for FAT32 Inodes")
    }
}

impl Inode for Fat32Inode {
    fn inode_num(&self) -> u32 {
        self.cluster
    }

    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat {
            size: self.size as u64,
            file_type: if self.is_dir { FileType::Directory } else { FileType::File },
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }

    fn lookup(&self, target_name: &str) -> VfsResult<Arc<dyn Inode>> {
        if !self.is_dir { return Err(VfsError::NotADirectory); }

        let sectors = self.fs.bpb.sectors_per_cluster;
        let mut buf = alloc::vec![0u16; (sectors as usize) * 256];

        if !self.fs.read_cluster(self.cluster, &mut buf) {
            return Err(VfsError::IOError);
        }

        let buf_u8 = unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u8, (sectors as usize) * 512) };
        let entries = buf_u8.len() / 32;

        for i in 0..entries {
            let entry_ptr = buf_u8[i * 32..].as_ptr() as *const FatDirEntry;
            let entry = unsafe { core::ptr::read_unaligned(entry_ptr) };

            if entry.name[0] == 0x00 { break; } // End of directory
            if entry.name[0] == 0xE5 { continue; } // Deleted entry
            if entry.attr & ATTR_LONG_NAME == ATTR_LONG_NAME { continue; } // Skip LFN for now

            let mut name_buf = String::new();
            for j in 0..8 {
                if entry.name[j] != b' ' { name_buf.push(entry.name[j] as char); }
            }
            if entry.name[8] != b' ' {
                name_buf.push('.');
                for j in 8..11 {
                    if entry.name[j] != b' ' { name_buf.push(entry.name[j] as char); }
                }
            }

            // A bit of hacky uppercase matching for 8.3 FAT names
            if name_buf.eq_ignore_ascii_case(target_name) {
                let cluster = ((entry.fst_clus_hi as u32) << 16) | (entry.fst_clus_lo as u32);
                let is_dir = (entry.attr & ATTR_DIRECTORY) != 0;
                
                return Ok(Arc::new(Fat32Inode {
                    fs: self.fs.clone(),
                    cluster,
                    is_dir,
                    size: entry.file_size,
                    name: name_buf,
                }));
            }
        }
        
        Err(VfsError::NotFound)
    }

    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        if self.is_dir { return Err(VfsError::IsADirectory); }
        // For read-only, we just need a handle that can read this cluster
        Ok(Arc::new(Fat32FileHandle {
            fs: self.fs.clone(),
            cluster: self.cluster,
            size: self.size,
        }))
    }

    fn read_dir(&self) -> VfsResult<Vec<String>> {
        if !self.is_dir { return Err(VfsError::NotADirectory); }

        let mut result = Vec::new();
        let sectors = self.fs.bpb.sectors_per_cluster;
        let mut buf = alloc::vec![0u16; (sectors as usize) * 256];

        if !self.fs.read_cluster(self.cluster, &mut buf) {
            return Err(VfsError::IOError);
        }

        let buf_u8 = unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u8, (sectors as usize) * 512) };
        let entries = buf_u8.len() / 32;

        for i in 0..entries {
            let entry_ptr = buf_u8[i * 32..].as_ptr() as *const FatDirEntry;
            let entry = unsafe { core::ptr::read_unaligned(entry_ptr) };

            if entry.name[0] == 0x00 { break; } 
            if entry.name[0] == 0xE5 { continue; }
            if entry.attr & ATTR_LONG_NAME == ATTR_LONG_NAME { continue; }

            let mut name_buf = String::new();
            for j in 0..8 {
                if entry.name[j] != b' ' { name_buf.push(entry.name[j] as char); }
            }
            if entry.name[8] != b' ' {
                name_buf.push('.');
                for j in 8..11 {
                    if entry.name[j] != b' ' { name_buf.push(entry.name[j] as char); }
                }
            }
            if !name_buf.is_empty() {
                result.push(name_buf);
            }
        }
        
        Ok(result)
    }

    fn create(&self, _name: &str, _file_type: FileType) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotImplemented) }
    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotImplemented) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::NotImplemented) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::NotImplemented) }
    fn rename(&self, _old_name: &str, _new_parent: Arc<dyn Inode>, _new_name: &str) -> VfsResult<()> { Err(VfsError::NotImplemented) }
    fn link(&self, _name: &str, _inode: Arc<dyn Inode>) -> VfsResult<()> { Err(VfsError::NotImplemented) }
    fn chmod(&self, _mode: u16) -> VfsResult<()> { Err(VfsError::NotImplemented) }
    fn chown(&self, _uid: u16, _gid: u16) -> VfsResult<()> { Err(VfsError::NotImplemented) }
}

#[derive(Debug)]
pub struct Fat32FileHandle {
    fs: Arc<Fat32FileSystem>,
    cluster: u32,
    size: u32,
}

impl FileHandle for Fat32FileHandle {
    fn read(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        if offset >= self.size as u64 { return Ok(0); }
        
        let cluster_size = (self.fs.bpb.sectors_per_cluster as u32 * self.fs.bpb.bytes_per_sector as u32) as u64;
        let start_cluster_idx = (offset / cluster_size) as u32;
        let cluster_offset = (offset % cluster_size) as usize;
        
        let mut current_cluster = self.cluster;
        for _ in 0..start_cluster_idx {
            if let Some(next) = self.fs.next_cluster(current_cluster) {
                current_cluster = next;
            } else {
                return Ok(0); // Unexpected EOF
            }
        }
        
        let sectors = self.fs.bpb.sectors_per_cluster;
        let mut cl_buf = alloc::vec![0u16; (sectors as usize) * 256];
        if !self.fs.read_cluster(current_cluster, &mut cl_buf) {
            return Err(VfsError::IOError);
        }
        
        let cl_buf_u8 = unsafe { core::slice::from_raw_parts(cl_buf.as_ptr() as *const u8, (sectors as usize) * 512) };
        
        let available_in_file = (self.size as u64 - offset) as usize;
        let available_in_cluster = cl_buf_u8.len() - cluster_offset;
        
        let to_read = core::cmp::min(buf.len(), available_in_file);
        let to_read = core::cmp::min(to_read, available_in_cluster);
        
        buf[..to_read].copy_from_slice(&cl_buf_u8[cluster_offset..cluster_offset + to_read]);
        
        Ok(to_read)
    }

    fn write(&self, _buf: &[u8], _offset: u64) -> VfsResult<usize> {
        Err(VfsError::NotImplemented) // Read-only
    }
    
    fn truncate(&self) -> VfsResult<()> { Err(VfsError::NotImplemented) }
    fn close(&self) -> VfsResult<()> { Ok(()) }
}
