use crate::drivers::ata;
use core::mem::transmute;
use crate::drivers::video;
use alloc::format;

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

pub fn parse_superblock() -> Result<Superblock, u16> {
    // Superblock starts at byte 1024.
    // LBA 0 = 0-511
    // LBA 1 = 512-1023
    // LBA 2 = 1024-1535.
    
    // We need to read enough to cover the struct (sizeof Superblock is usually 1024 bytes, but minimal is 84).
    // Let's read 2 sectors (1024 bytes) starting at LBA 2.
    
    let mut buffer = [0u16; 512]; // 512 u16 = 1024 bytes
    if !ata::read_sectors(&mut buffer, 2, 2) {
        return Err(0); // Read failed
    }
    
    unsafe {
        let ptr = buffer.as_ptr() as *const Superblock;
        let sb = *ptr;
        
        if sb.magic == 0xEF53 {
             Ok(sb)
        } else {
             Err(sb.magic)
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BlockGroupDescriptor {
    pub block_bitmap_lo: u32,
    pub inode_bitmap_lo: u32,
    pub inode_table_lo: u32,
    pub free_blocks_count_lo: u16,
    pub free_inodes_count_lo: u16,
    pub used_dirs_count_lo: u16,
    pub flags: u16,
    pub exclude_bitmap_lo: u32,
    pub block_bitmap_hi: u32,
    pub inode_bitmap_hi: u32,
    pub inode_table_hi: u32,
    pub free_blocks_count_hi: u16,
    pub free_inodes_count_hi: u16,
    pub used_dirs_count_hi: u16,
    pub pad: u16,
    pub reserved: [u32; 3],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DiskInode {
    pub mode: u16,
    pub uid: u16,
    pub size_lo: u32,
    pub atime: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub dtime: u32,
    pub gid: u16,
    pub links_count: u16,
    pub blocks_lo: u32,
    pub flags: u32,
    pub osd1: u32,
    pub block: [u32; 15], // Pointers (Direct/Indirect) or Extents
    pub generation: u32,
    pub file_acl_lo: u32,
    pub size_hi: u32,
    pub obso_faddr: u32,
    // ... OS specific ...
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DirEntry2 {
    pub inode: u32,
    pub rec_len: u16,
    pub name_len: u8,
    pub file_type: u8,
    // name follows
}

use crate::fs::vfs::{self, FileSystem, Inode, FileHandle, VfsResult, VfsError, FileType, FileStat};
use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;

pub struct Ext4FileSystem {
    inner: Arc<Ext4FsInner>,
}

#[derive(Debug)]
pub struct Ext4FsInner {
    sb: Superblock,
    block_size: u64,
}

impl Ext4FileSystem {
    pub fn new(sb: Superblock) -> Self {
        let block_size = 1024u64 << sb.log_block_size;
        Self {
            inner: Arc::new(Ext4FsInner { sb, block_size })
        }
    }
}

impl FileSystem for Ext4FileSystem {
    fn root_inode(&self) -> Arc<dyn Inode> {
        let disk_inode = self.inner.read_inode(2).unwrap();
        Arc::new(Ext4Inode {
            fs: self.inner.clone(),
            inode_num: 2,
            disk_inode,
        })
    }
}

impl Ext4FsInner {
    fn read_block(&self, block_id: u32, buf: &mut [u8]) {
        let sectors_per_block = (self.block_size / 512) as u32;
        let start_lba = block_id * sectors_per_block;
        let ptr = buf.as_ptr() as *mut u16;
        let u16_len = buf.len() / 2;
        
        unsafe {
             let u16_slice = core::slice::from_raw_parts_mut(ptr, u16_len);
             
             // DEBUG: Print block read
             // crate::drivers::video::put_str("Ext4: Read LBA ");
             // crate::drivers::video::put_int(start_lba as usize);
             // crate::drivers::video::put_char('\n');
             
             for i in 0..sectors_per_block {
                 let sector_lba = start_lba + i;
                 let slice_offset = (i as usize) * 256;
                 
                 if !ata::read_sectors(&mut u16_slice[slice_offset..slice_offset+256], sector_lba, 1) {
                     crate::drivers::video::put_str("Ext4: read_block failed (single sector)!\n");
                 }
             }
        }
    }
    
    fn write_block(&self, block_id: u32, buf: &[u8]) -> bool {
        let sectors_per_block = (self.block_size / 512) as u32;
        let start_lba = block_id * sectors_per_block;
        let ptr = buf.as_ptr() as *const u16;
        let u16_len = buf.len() / 2;
        
        unsafe {
             let u16_slice = core::slice::from_raw_parts(ptr, u16_len);
             
             for i in 0..sectors_per_block {
                 let sector_lba = start_lba + i;
                 let slice_offset = (i as usize) * 256;
                 
                 // Check bounds
                 let sector_data = if slice_offset + 256 <= u16_slice.len() {
                      &u16_slice[slice_offset..slice_offset+256]
                 } else {
                      return false; // Buffer too small?
                 };
                 
                 if !ata::write_sectors(sector_data, sector_lba, 1) {
                     return false;
                 }
             }
        }
        true
    }

    fn read_inode(&self, inode_num: u32) -> VfsResult<DiskInode> {
        // Inode numbering starts at 1. Index = inode_num - 1.
        let index = inode_num - 1;
        let group_id = index / self.sb.inodes_per_group;
        let index_in_group = index % self.sb.inodes_per_group;
        
        // BGDT Block
        let bgdt_block = if self.block_size == 1024 { 2 } else { 1 };
        
        let mut block_buf = alloc::vec![0u8; self.block_size as usize];
        self.read_block(bgdt_block, &mut block_buf);
        
        let bgd_size = core::mem::size_of::<BlockGroupDescriptor>(); 
        let bgd_offset = (group_id as usize) * bgd_size;
        
        let bgd: BlockGroupDescriptor = unsafe {
             let ptr = block_buf.as_ptr().add(bgd_offset) as *const BlockGroupDescriptor;
             *ptr
        };
        
        // Inode Table
        let inode_table_block = bgd.inode_table_lo;
        let inode_size = 256; // Assume 256 for now (should from SB)
        
        let inode_offset_in_table = (index_in_group as usize) * inode_size;
        let block_offset = inode_offset_in_table / (self.block_size as usize);
        let byte_offset_in_block = inode_offset_in_table % (self.block_size as usize);
        
        let target_block = inode_table_block + block_offset as u32;
        
        self.read_block(target_block, &mut block_buf);
        
        let disk_inode = unsafe {
            let ptr = block_buf.as_ptr().add(byte_offset_in_block) as *const DiskInode;
            *ptr
        };
        
        Ok(disk_inode)
    }
    // --- Allocation Logic (Naive: Group 0 Only) ---
    
    fn read_block_bitmap(&self, bgd: &BlockGroupDescriptor) -> Option<alloc::vec::Vec<u8>> {
        let mut buf = alloc::vec![0u8; self.block_size as usize];
        self.read_block(bgd.block_bitmap_lo, &mut buf);
        Some(buf) // TODO: Handle large bitmaps? Block size usually covers it for small groups.
    }
    
    fn write_block_bitmap(&self, bgd: &BlockGroupDescriptor, buf: &[u8]) {
        self.write_block(bgd.block_bitmap_lo, buf);
    }
    
    fn read_inode_bitmap(&self, bgd: &BlockGroupDescriptor) -> Option<alloc::vec::Vec<u8>> {
        let mut buf = alloc::vec![0u8; self.block_size as usize];
        self.read_block(bgd.inode_bitmap_lo, &mut buf);
        Some(buf)
    }

    fn write_inode_bitmap(&self, bgd: &BlockGroupDescriptor, buf: &[u8]) {
        self.write_block(bgd.inode_bitmap_lo, buf);
    }
    
    fn get_bgd(&self, group: u32) -> BlockGroupDescriptor {
        // Read BGDT (Block 1 or 2)
        // Stub: Assume Group 0
        let bgdt_block = if self.block_size == 1024 { 2 } else { 1 };
        let mut block_buf = alloc::vec![0u8; self.block_size as usize];
        self.read_block(bgdt_block, &mut block_buf);
        
        unsafe {
            let ptr = block_buf.as_ptr() as *const BlockGroupDescriptor;
            *ptr // Group 0
        }
    }
    
    fn update_bgd(&self, group: u32, bgd: BlockGroupDescriptor) {
         // Write back Group 0 descriptor
         let bgdt_block = if self.block_size == 1024 { 2 } else { 1 };
         let mut block_buf = alloc::vec![0u8; self.block_size as usize];
         self.read_block(bgdt_block, &mut block_buf); // RMW
         
         unsafe {
             let ptr = block_buf.as_ptr() as *mut BlockGroupDescriptor;
             *ptr = bgd;
         }
         self.write_block(bgdt_block, &block_buf);
    }

    pub fn alloc_inode(&self) -> VfsResult<u32> {
        let mut bgd = self.get_bgd(0);
        if bgd.free_inodes_count_lo == 0 { return Err(VfsError::NoSpace); }
        
        let mut bitmap = self.read_inode_bitmap(&bgd).ok_or(VfsError::IOError)?;
        
        // Find 0 bit
        for i in 0..bitmap.len() {
            if bitmap[i] != 0xFF {
                for bit in 0..8 {
                    if (bitmap[i] & (1 << bit)) == 0 {
                        // Found free
                        let inode_idx = (i * 8 + bit) as u32;
                        let inode_num = inode_idx + 1; // 1-based
                        
                        // Reserved check
                        if inode_num < 11 { continue; } // First 10 usually reserved
                        
                        bitmap[i] |= 1 << bit;
                        self.write_inode_bitmap(&bgd, &bitmap);
                        
                        bgd.free_inodes_count_lo -= 1;
                        bgd.used_dirs_count_lo += 1; // Approx?
                        self.update_bgd(0, bgd);
                        
                        // Init Inode Table Entry to clean state?
                        // Let caller do it via write_inode
                        
                        return Ok(inode_num);
                    }
                }
            }
        }
        
        Err(VfsError::NoSpace)
    }

    pub fn alloc_block(&self) -> VfsResult<u32> {
        let mut bgd = self.get_bgd(0);
        if bgd.free_blocks_count_lo == 0 { return Err(VfsError::NoSpace); }
        
        let mut bitmap = self.read_block_bitmap(&bgd).ok_or(VfsError::IOError)?;
        
        for i in 0..bitmap.len() {
            if bitmap[i] != 0xFF {
                for bit in 0..8 {
                    if (bitmap[i] & (1 << bit)) == 0 {
                        let block_idx = (i * 8 + bit) as u32;
                        
                        // Offset by First Data Block?
                        // If block_idx is relative to group?
                        // For Group 0, it's absolute + first_data_block usually?
                        // Block Bitmap tracks blocks in the group.
                        // Group 0 starts at self.sb.first_data_block.
                        let actual_block = self.sb.first_data_block + block_idx;
                        
                        bitmap[i] |= 1 << bit;
                        self.write_block_bitmap(&bgd, &bitmap);
                        
                        bgd.free_blocks_count_lo -= 1;
                        self.update_bgd(0, bgd);
                        
                        return Ok(actual_block);
                    }
                }
            }
        }
        Err(VfsError::NoSpace)
    }

    pub fn write_inode(&self, inode_num: u32, inode: DiskInode) -> VfsResult<()> {
        let index = inode_num - 1;
        // Group logic (Stub Group 0)
        let _group = index / self.sb.inodes_per_group; 
        let index_in_group = index % self.sb.inodes_per_group;
        
        let bgd = self.get_bgd(0);
        let inode_table_block = bgd.inode_table_lo;
        let inode_size = 256; // Fixed size for now
        
        let inode_offset_in_table = (index_in_group as usize) * inode_size;
        let block_offset = inode_offset_in_table / (self.block_size as usize);
        let byte_offset_in_block = inode_offset_in_table % (self.block_size as usize);
        
        let target_block = inode_table_block + block_offset as u32;
        
        let mut block_buf = alloc::vec![0u8; self.block_size as usize];
        self.read_block(target_block, &mut block_buf); // RMW
        
        unsafe {
             let ptr = block_buf.as_ptr().add(byte_offset_in_block) as *mut DiskInode;
             *ptr = inode;
        }
        
        if self.write_block(target_block, &block_buf) {
            Ok(())
        } else {
            Err(VfsError::IOError)
        }
    }
}

pub struct Ext4Inode {
    fs: Arc<Ext4FsInner>,
    inode_num: u32,
    disk_inode: DiskInode,
}

impl Inode for Ext4Inode {
    fn inode_num(&self) -> u32 {
        self.inode_num
    }

    fn stat(&self) -> VfsResult<FileStat> {
        let mode = self.disk_inode.mode;
        let file_type = if (mode & 0x4000) != 0 { FileType::Directory } else { FileType::File };
        Ok(FileStat {
            size: self.disk_inode.size_lo as u64, // Todo: use size_hi
            file_type,
            mode: self.disk_inode.mode,
            uid: self.disk_inode.uid,
            gid: self.disk_inode.gid,
            mtime: self.disk_inode.mtime,
        })
    }
    
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        if (self.disk_inode.mode & 0x4000) == 0 {
            return Err(VfsError::NotADirectory);
        }
        
        // Read Directory content.
        // Simplified: Read direct blocks.
        // Assuming linear directory (no H-tree).
        
        let mut buf = alloc::vec![0u8; self.fs.block_size as usize];
        
        // Iterate over blocks (Only direct blocks 0-11 for now)
        for i in 0..12 {
            let block_id = self.disk_inode.block[i];
            if block_id == 0 { break; }
            
            self.fs.read_block(block_id, &mut buf);
            
            let mut offset = 0;
            while offset < buf.len() {
                if offset + core::mem::size_of::<DirEntry2>() > buf.len() { break; }
                let entry_ptr = unsafe { buf.as_ptr().add(offset) as *const DirEntry2 };
                let entry = unsafe { *entry_ptr };
                
                let entry_header_size = core::mem::size_of::<DirEntry2>();
                if entry.rec_len == 0 { break; } 
                
                if entry.inode != 0 {
                    let name_len = entry.name_len as usize;
                    let name_slice = unsafe { core::slice::from_raw_parts( 
                        buf.as_ptr().add(offset + entry_header_size), 
                        name_len 
                    ) };
                
                    if let Ok(s) = core::str::from_utf8(name_slice) {
                        if s == name {
                            // Found!
                            match self.fs.read_inode(entry.inode) {
                                Ok(child_inode) => {
                                    return Ok(Arc::new(Ext4Inode {
                                        fs: self.fs.clone(),
                                        inode_num: entry.inode,
                                        disk_inode: child_inode,
                                    }));
                                },
                                Err(_) => return Err(VfsError::IOError),
                            }
                        }
                    }
                }
                
                offset += entry.rec_len as usize;
            }
        }
        
        Err(VfsError::NotFound)
    }
    
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        Ok(Arc::new(Ext4File {
            fs: self.fs.clone(),
            inode: self.disk_inode, // Copy
            inode_num: self.inode_num,
        }))
    }
    

    
    fn read_dir(&self) -> VfsResult<Vec<String>> {
        if (self.disk_inode.mode & 0x4000) == 0 {
            return Err(VfsError::NotADirectory);
        }
        
        let mut names = Vec::new();
        let mut buf = alloc::vec![0u8; self.fs.block_size as usize];
        
        // Iterate over blocks (Only direct blocks 0-11 for now)
        for i in 0..12 {
            let block_id = self.disk_inode.block[i];
            if block_id == 0 { break; }
            
            self.fs.read_block(block_id, &mut buf);
            
            let mut offset = 0;
            while offset < buf.len() {
                if offset + core::mem::size_of::<DirEntry2>() > buf.len() { break; }
                let entry_ptr = unsafe { buf.as_ptr().add(offset) as *const DirEntry2 };
                let entry = unsafe { *entry_ptr };
                
                if entry.rec_len == 0 { break; } 
                
                if entry.inode != 0 {
                    let name_len = entry.name_len as usize;
                    let entry_header_size = core::mem::size_of::<DirEntry2>();
                    
                    // Bounds Check
                    if offset + entry_header_size + name_len > buf.len() {
                         crate::drivers::video::put_str("LS: OOB Name!\n");
                         break;
                    }

                    let name_slice = unsafe { core::slice::from_raw_parts( 
                        buf.as_ptr().add(offset + entry_header_size), 
                        name_len 
                    ) };
                    
                    if let Ok(s) = core::str::from_utf8(name_slice) {
                        names.push(String::from(s));
                    }
                }
                
                offset += entry.rec_len as usize;
            }
        }
        
        Ok(names)
    }

    fn create(&self, name: &str, file_type: FileType) -> VfsResult<Arc<dyn Inode>> {
        if let Ok(_) = self.lookup(name) { return Err(VfsError::AlreadyExists); }
        self.make_entry(name, file_type)
    }

    fn mkdir(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        self.make_entry(name, FileType::Directory)
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        if (self.disk_inode.mode & 0x4000) == 0 {
            return Err(VfsError::NotADirectory);
        }
        
        // 1. Find inode of name
        let mut buf = alloc::vec![0u8; self.fs.block_size as usize];
        let mut found = false;
        
        for i in 0..12 {
            let block_id = self.disk_inode.block[i];
            if block_id == 0 { break; }
            self.fs.read_block(block_id, &mut buf);
            
            let mut offset = 0;
            let mut modified = false;
            while offset < buf.len() {
                let entry_ptr = unsafe { buf.as_ptr().add(offset) as *mut DirEntry2 };
                let entry = unsafe { &mut *entry_ptr };
                if entry.rec_len == 0 { break; }
                
                if entry.inode != 0 {
                    let name_len = entry.name_len as usize;
                    let entry_header_size = core::mem::size_of::<DirEntry2>();
                    if offset + entry_header_size + name_len <= buf.len() {
                        let name_slice = unsafe { core::slice::from_raw_parts( 
                           buf.as_ptr().add(offset + entry_header_size), 
                           name_len 
                        ) };
                        if let Ok(s) = core::str::from_utf8(name_slice) {
                            if s == name {
                                entry.inode = 0; // Mark deleted
                                found = true;
                                modified = true;
                                break;
                            }
                        }
                    }
                }
                offset += entry.rec_len as usize;
            }
            if modified {
                self.fs.write_block(block_id, &buf);
            }
            if found { break; }
        }
        
        if found { Ok(()) } else { Err(VfsError::NotFound) }
    }

    fn remove_dir(&self, name: &str) -> VfsResult<()> {
        self.unlink(name)
    }

    fn rename(&self, old_name: &str, new_parent: Arc<dyn Inode>, new_name: &str) -> VfsResult<()> {
        if (self.disk_inode.mode & 0x4000) == 0 {
            return Err(VfsError::NotADirectory);
        }
        
        // 1. Find inode of old_name in current dir (self)
        let child = self.lookup(old_name)?;
        
        // 2. Link in new parent
        new_parent.link(new_name, child.clone())?;
        
        // 3. Unlink from old (self)
        self.unlink(old_name)
    }

    fn link(&self, name: &str, inode: Arc<dyn Inode>) -> VfsResult<()> {
        if (self.disk_inode.mode & 0x4000) == 0 {
            return Err(VfsError::NotADirectory);
        }
        
        let stat = inode.stat()?;
        if !self.add_dir_entry(self.inode_num, name, inode.inode_num(), stat.file_type) {
             return Err(VfsError::NoSpace);
        }
        Ok(())
    }

    fn chmod(&self, mode: u16) -> VfsResult<()> {
        video::put_str("Ext4: chmod called\n");
        match self.fs.read_inode(self.inode_num) {
            Ok(mut disk_inode) => {
                 let type_mask = 0xF000;
                 let new_mode = (disk_inode.mode & type_mask) | (mode & !type_mask);
                 disk_inode.mode = new_mode;
                 match self.fs.write_inode(self.inode_num, disk_inode) {
                     Ok(_) => Ok(()),
                     Err(e) => {
                         video::put_str("Ext4: Write Inode Failed\n");
                         Err(e)
                     }
                 }
            },
            Err(e) => {
                video::put_str("Ext4: Read Inode Failed\n");
                Err(e)
            }
        }
    }

    fn chown(&self, uid: u16, gid: u16) -> VfsResult<()> {
        video::put_str(&format!("Ext4: chown inode {} to {}:{}\n", self.inode_num, uid, gid));
        match self.fs.read_inode(self.inode_num) {
            Ok(mut disk_inode) => {
                disk_inode.uid = uid;
                disk_inode.gid = gid;
                 match self.fs.write_inode(self.inode_num, disk_inode) {
                     Ok(_) => Ok(()),
                     Err(e) => {
                         video::put_str("Ext4: Write Inode Failed\n");
                         Err(e)
                     }
                 }
            },
            Err(e) => {
                video::put_str("Ext4: Read Inode Failed\n");
                Err(e)
            }
        }
    }
}

impl Ext4Inode {
    fn make_entry(&self, name: &str, file_type: FileType) -> VfsResult<Arc<dyn Inode>> {
        crate::drivers::video::put_str("Ext4: make_entry start\n");
        // 1. Allocate Inode
        let inode_num = self.fs.alloc_inode()?;
        crate::drivers::video::put_str("Ext4: Inode Allocated\n");
        
        // 2. Init Inode
        let mode = match file_type {
            FileType::Directory => 0x41ED, // Dir + 755 (approx)
            FileType::File => 0x81ED,      // File + 755
            _ => 0,
        };
        
        let mut new_inode = DiskInode {
            mode,
            uid: 0,
            size_lo: 0, // 0 for empty file, 4096 for dir (usually has . and ..)
            atime: 0, ctime: 0, mtime: 0, dtime: 0,
            gid: 0,
            links_count: 1,
            blocks_lo: 0,
            flags: 0,
            osd1: 0,
            block: [0; 15],
            generation: 0,
            file_acl_lo: 0,
            size_hi: 0,
            obso_faddr: 0,
        };
        
        if file_type == FileType::Directory {
            // Allocate 1 block for directory data (. and ..)
            let block = self.fs.alloc_block()?;
            crate::drivers::video::put_str("Ext4: Dir Block Allocated\n");
            
            new_inode.block[0] = block;
            new_inode.blocks_lo = (self.fs.block_size / 512) as u32; 
            new_inode.size_lo = self.fs.block_size as u32; // Standard dir size?
            new_inode.links_count = 2; // . and parent ref?
            
            // Initialize Directory Block with . and ..
            let mut buf = alloc::vec![0u8; self.fs.block_size as usize];
            unsafe {
                let dot_ptr = buf.as_mut_ptr();
                let dotdot_ptr = buf.as_mut_ptr().add(12);
                
                // .
                let entry_header_size = core::mem::size_of::<DirEntry2>();
                let dot = dot_ptr as *mut DirEntry2;
                (*dot).inode = inode_num;
                (*dot).rec_len = 12;
                (*dot).name_len = 1;
                (*dot).file_type = 2; // Directory
                core::ptr::copy_nonoverlapping(".".as_ptr(), dot_ptr.add(entry_header_size), 1);
                
                // ..
                let dotdot = dotdot_ptr as *mut DirEntry2;
                (*dotdot).inode = self.inode_num;
                (*dotdot).rec_len = (self.fs.block_size - 12) as u16;
                (*dotdot).name_len = 2;
                (*dotdot).file_type = 2;
                core::ptr::copy_nonoverlapping("..".as_ptr(), dotdot_ptr.add(entry_header_size), 2);
            }
            if !self.fs.write_block(block, &buf) { return Err(VfsError::IOError); }
        }
        
        self.fs.write_inode(inode_num, new_inode)?;
        crate::drivers::video::put_str("Ext4: Inode Written\n");
        
        // 3. Add Entry to Parent (self)
        // We need to write a DirEntry2 to 'self's blocks.
        if !self.add_dir_entry(self.inode_num, name, inode_num, file_type) {
             crate::drivers::video::put_str("Ext4: Failed to add dir entry\n");
             return Err(VfsError::IOError);
        }
        crate::drivers::video::put_str("Ext4: Dir Entry Added\n");
        
        Ok(Arc::new(Ext4Inode {
            fs: self.fs.clone(),
            inode_num,
            disk_inode: new_inode,
        }))
    }

    fn add_dir_entry(&self, _parent_inode_num: u32, name: &str, child_inode: u32, ftype: FileType) -> bool {
        let mut buf = alloc::vec![0u8; self.fs.block_size as usize];
        let block_id = self.disk_inode.block[0]; 
        if block_id == 0 { return false; }
        
        self.fs.read_block(block_id, &mut buf);
        
        let mut offset = 0;
        crate::drivers::video::put_str("Ext4: add_dir_entry start\n");

        while offset < buf.len() {
             let entry_ptr = unsafe { buf.as_ptr().add(offset) as *mut DirEntry2 };
             let entry = unsafe { &mut *entry_ptr };
             // if entry.inode == 0 { break; } 
             if entry.rec_len == 0 { 
                 if offset == 0 {
                     // Block is all zeros. Initialize as one big empty entry.
                     entry.rec_len = self.fs.block_size as u16;
                     entry.inode = 0;
                 } else {
                     crate::drivers::video::put_str("Ext4: Zero RecLen! Break.\n");
                     break; 
                 }
             }
             
             let entry_header_size = core::mem::size_of::<DirEntry2>();
             let real_len = (entry_header_size as u16 + entry.name_len as u16 + 3) & !3;
             let available = entry.rec_len - real_len;
             let needed = (entry_header_size as u16 + name.len() as u16 + 3) & !3;
                 
             if available >= needed {
                 crate::drivers::video::put_str("Ext4: Found Space! Splitting.\n");
                 // Split!
                 entry.rec_len = real_len; // Shrink current
                     
                 let new_offset = offset + real_len as usize;
                 let new_ptr = unsafe { buf.as_ptr().add(new_offset) as *mut DirEntry2 };
                 let new_entry = unsafe { &mut *new_ptr };
                     
                 new_entry.inode = child_inode;
                 new_entry.rec_len = available; // Take rest
                 new_entry.name_len = name.len() as u8;
                 new_entry.file_type = match ftype { FileType::Directory => 2, _ => 1 };

                 // Write Name
                 unsafe {
                     core::ptr::copy_nonoverlapping(
                         name.as_ptr(), 
                         buf.as_ptr().add(new_offset + 8) as *mut u8, 
                         name.len()
                     );
                 }
                     
                 // Write back block
                 return self.fs.write_block(block_id, &buf);
             }
             
             offset += entry.rec_len as usize;
        }
        
        false // No space in first block
    }


    


    fn remove_dir(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotImplemented)
    }

    fn rename(&self, old_name: &str, new_name: &str) -> VfsResult<()> {
        if (self.disk_inode.mode & 0x4000) == 0 {
            return Err(VfsError::NotADirectory);
        }
        
        // 1. Find inode of old_name
        let mut buf = alloc::vec![0u8; self.fs.block_size as usize];
        let mut found_inode = 0;
        let mut found = false;
        
        for i in 0..12 {
            let block_id = self.disk_inode.block[i];
            if block_id == 0 { break; }
            self.fs.read_block(block_id, &mut buf);
            
            let mut offset = 0;
            while offset < buf.len() {
                let entry_ptr = unsafe { buf.as_ptr().add(offset) as *mut DirEntry2 };
                let entry = unsafe { &mut *entry_ptr };
                if entry.rec_len == 0 { break; }
                
                if entry.inode != 0 {
                    let name_len = entry.name_len as usize;
                     if offset + 8 + name_len <= buf.len() {
                         let name_slice = unsafe { core::slice::from_raw_parts( 
                            buf.as_ptr().add(offset + 8), 
                            name_len 
                         ) };
                         if let Ok(s) = core::str::from_utf8(name_slice) {
                             if s == old_name {
                                 found_inode = entry.inode;
                                 found = true;
                                 break;
                             }
                         }
                     }
                }
                offset += entry.rec_len as usize;
            }
            if found { break; }
        }
        
        if !found { return Err(VfsError::NotFound); }
        
        // 2. Add new entry
        // Get type from inode? Reading inode.
        let inode_struct = self.fs.read_inode(found_inode)?;
        let ftype = if (inode_struct.mode & 0x4000) != 0 { FileType::Directory } else { FileType::File };
        
        if !self.add_dir_entry(self.inode_num, new_name, found_inode, ftype) {
             return Err(VfsError::NoSpace);
        }
        
        // 3. Unlink old
        self.unlink(old_name)
    }

    fn chmod(&self, mode: u16) -> VfsResult<()> {
        video::put_str("Ext4: chmod called\n");
        match self.fs.read_inode(self.inode_num) {
            Ok(mut disk_inode) => {
                 let type_mask = 0xF000;
                 let new_mode = (disk_inode.mode & type_mask) | (mode & !type_mask);
                 disk_inode.mode = new_mode;
                 match self.fs.write_inode(self.inode_num, disk_inode) {
                     Ok(_) => Ok(()),
                     Err(e) => {
                         video::put_str("Ext4: Write Inode Failed\n");
                         Err(e)
                     }
                 }
            },
            Err(e) => {
                video::put_str("Ext4: Read Inode Failed\n");
                Err(e)
            }
        }
    }

    fn chown(&self, uid: u16, gid: u16) -> VfsResult<()> {
         video::put_str(&format!("Ext4: chown inode {} to {}:{}\n", self.inode_num, uid, gid));
        match self.fs.read_inode(self.inode_num) {
            Ok(mut disk_inode) => {
                disk_inode.uid = uid;
                disk_inode.gid = gid;
                 match self.fs.write_inode(self.inode_num, disk_inode) {
                     Ok(_) => Ok(()),
                     Err(e) => {
                         video::put_str("Ext4: Write Inode Failed\n");
                         Err(e)
                     }
                 }
            },
            Err(e) => {
                video::put_str("Ext4: Read Inode Failed\n");
                Err(e)
            }
        }
    }
}

#[derive(Debug)]
pub struct Ext4File {
    fs: Arc<Ext4FsInner>,
    inode: DiskInode,
    inode_num: u32,
}

impl FileHandle for Ext4File {
    fn read(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        let size = self.inode.size_lo as u64;
        if offset >= size { return Ok(0); }
        
        let mut read_len = buf.len();
        if offset + read_len as u64 > size {
            read_len = (size - offset) as usize;
        }
        
        let block_size = self.fs.block_size;
        let start_block = (offset / block_size) as usize;
        // let start_offset = (offset % block_size) as usize; // inside block
        
        // Simplified: Read one block at a time.
        // Does not handle cross-block reads efficiently or Indirect blocks yet.
        
        // Verify direct block range
        if start_block >= 12 { return Err(VfsError::IOError); } // TODO: Indirect
        
        let block_id = self.inode.block[start_block];
        if block_id == 0 {
            // Sparse? Return 0s
            for b in &mut buf[0..read_len] { *b = 0; }
            return Ok(read_len);
        }
        
        let mut temp_buf = alloc::vec![0u8; block_size as usize];
        self.fs.read_block(block_id, &mut temp_buf);
        
        let block_offset = (offset % block_size) as usize;
        let copy_len = core::cmp::min(read_len, (block_size as usize) - block_offset);
        
        buf[0..copy_len].copy_from_slice(&temp_buf[block_offset .. block_offset+copy_len]);
        
        Ok(copy_len)
    }
    
    fn write(&self, buf: &[u8], offset: u64) -> VfsResult<usize> {
        // Reload inode to ensure we have latest state (and for modification)
        let mut inode = match self.fs.read_inode(self.inode_num) {
            Ok(i) => i,
            Err(_) => return Err(VfsError::IOError),
        };

        let current_size = inode.size_lo as u64;
        let mut bytes_written = 0;
        let mut current_offset = offset;
        let block_size = self.fs.block_size;
        
        while bytes_written < buf.len() {
            // Allocate if extending
            if current_offset >= current_size {
                 let block_idx = (current_offset / block_size) as usize;
                 if block_idx >= 12 { return Err(VfsError::IOError); } 
                 
                 if inode.block[block_idx] == 0 {
                     let new_block = match self.fs.alloc_block() {
                        Ok(b) => b,
                        Err(_) => return Err(VfsError::NoSpace),
                     };
                     inode.block[block_idx] = new_block;
                     
                     // Clear new block (Zero it out)
                     let zeros = alloc::vec![0u8; block_size as usize];
                     self.fs.write_block(new_block, &zeros);
                 }
            }
            
            // Now we have a valid block (or should)
            let block_idx = (current_offset / block_size) as usize;
            
            // Handle sparse holes inside file size
            if inode.block[block_idx] == 0 {
                 let new_block = match self.fs.alloc_block() {
                    Ok(b) => b,
                    Err(_) => return Err(VfsError::NoSpace),
                 };
                 inode.block[block_idx] = new_block;
                 let zeros = alloc::vec![0u8; block_size as usize];
                 self.fs.write_block(new_block, &zeros);
            }
            
            let actual_block_id = inode.block[block_idx];
            
            let offset_in_block = (current_offset % block_size) as usize;
            let space_in_block = (block_size as usize) - offset_in_block;
            let remaining_data = buf.len() - bytes_written;
            let to_copy = core::cmp::min(space_in_block, remaining_data);
            
            // Read-Modify-Write
            let mut block_buf = alloc::vec![0u8; block_size as usize];
            self.fs.read_block(actual_block_id, &mut block_buf);
            
            unsafe {
                core::ptr::copy_nonoverlapping(
                    buf.as_ptr().add(bytes_written),
                    block_buf.as_mut_ptr().add(offset_in_block),
                    to_copy
                );
            }
            
            if !self.fs.write_block(actual_block_id, &block_buf) {
                return Err(VfsError::IOError);
            }
            
            bytes_written += to_copy;
            current_offset += to_copy as u64;
            
            // Update size logic:
            // The size is max(current_size, current_offset) at the end.
            // But we can update intermediate size if we want?
            // Let's rely on final update.
        }
        
        // Final Size Update
        if current_offset > current_size {
            inode.size_lo = current_offset as u32;
        }
        
        // Persist Inode
        self.fs.write_inode(self.inode_num, inode)?;
        
        Ok(bytes_written)
    }
    
    fn truncate(&self) -> VfsResult<()> {
        let mut inode = self.fs.read_inode(self.inode_num)?;
        inode.size_lo = 0;
        inode.size_hi = 0;
        // Simple truncate: just clear blocks. In a real OS, we'd free them.
        for i in 0..15 {
            inode.block[i] = 0;
        }
        self.fs.write_inode(self.inode_num, inode)?;
        Ok(())
    }

    fn close(&self) -> VfsResult<()> {
        Ok(())
    }
}

impl Ext4FileSystem {
    pub fn format(_device: u8) -> bool {
        // Simple MKFS implementation for Ainux (Ext4)
        // 1. Write Superblock
        // 2. Write BGDT
        // 3. Write Root Inode (Inode 2)
        // 4. Write Root Directory Block
        
        crate::drivers::video::put_str("MKFS: Formatting... ");
        
        // 1. Superblock (Block 0/1 depending on size, offset 1024)
        // LBA 2 (bytes 1024-1535) and LBA 3.
        
        let sb = Superblock {
            inodes_count: 1024,
            blocks_count_lo: 8192,
            r_blocks_count_lo: 0,
            free_blocks_count_lo: 8100, // Approx
            free_inodes_count: 1013, // 1024 - 11 reserved
            first_data_block: 1, // 1024 block size
            log_block_size: 0,   // 1024 bytes (2 ^ (10+0))
            log_cluster_size: 0,
            blocks_per_group: 8192,
            clusters_per_group: 8192,
            inodes_per_group: 1024,
            mtime: 0, wtime: 0,
            mnt_count: 0, max_mnt_count: 0xFFFF,
            magic: 0xEF53,
            state: 1, errors: 1,
            minor_rev_level: 0,
            lastcheck: 0, checkinterval: 0,
            creator_os: 0, rev_level: 0,
            def_resuid: 0, def_resgid: 0,
        };
        
        // Serialize Superblock to buffer
        // Align to 1024 bytes
        let mut sb_buf = alloc::vec![0u8; 1024];
        unsafe {
             let ptr = sb_buf.as_ptr() as *mut Superblock;
             *ptr = sb;
        }
        
        // Write SB to LBA 2 (Sector 2 and 3)
        // Convert u8 buf to u16 for ATA
        let mut ata_buf = alloc::vec![0u16; 512]; // 1024 bytes
        unsafe {
            let src = sb_buf.as_ptr() as *const u16;
            let dst = ata_buf.as_mut_ptr();
            core::ptr::copy_nonoverlapping(src, dst, 512);
        }
        
        if !ata::write_sectors(&ata_buf, 2, 2) { 
            crate::drivers::video::put_str("Failed to write SB!\n");
            return false; 
        }
        
        // 2. Block Group Descriptor Table (BGDT)
        // Block 2 (LBA 4, 5) if block_size=1024 (SB is Block 1)
        
        // Fixed BGDT for 1 Group
        let bgd = BlockGroupDescriptor {
            block_bitmap_lo: 3,
            inode_bitmap_lo: 4,
            inode_table_lo: 5,
            free_blocks_count_lo: 8000, // Dummy
            free_inodes_count_lo: 1000, // Dummy
            used_dirs_count_lo: 1,      // Root
            flags: 0,
            exclude_bitmap_lo: 0,
            block_bitmap_hi: 0,
            inode_bitmap_hi: 0,
            inode_table_hi: 0,
            free_blocks_count_hi: 0,
            free_inodes_count_hi: 0,
            used_dirs_count_hi: 0,
            pad: 0,
            reserved: [0; 3],  
        };
        
        let mut bgd_buf = alloc::vec![0u8; 1024];
        unsafe {
             let ptr = bgd_buf.as_ptr() as *mut BlockGroupDescriptor;
             *ptr = bgd;
        }
        
        // Write BGDT to LBA 4 (Block 2)
        unsafe {
            let src = bgd_buf.as_ptr() as *const u16;
            let dst = ata_buf.as_mut_ptr();
            core::ptr::copy_nonoverlapping(src, dst, 512); 
        }
        if !ata::write_sectors(&ata_buf, 4, 2) { 
             crate::drivers::video::put_str("Failed to write BGDT!\n");
             return false; 
        }
        
        // 3. Bitmaps (LBA 6, 7 for Block Bitmap | LBA 8, 9 for Inode Bitmap)
        // Clear them (all free except reserved)
        let mut zero_buf = alloc::vec![0u16; 512];
        ata::write_sectors(&zero_buf, 6, 2); // Block Bitmap
        ata::write_sectors(&zero_buf, 8, 2); // Inode Bitmap
        
        // 4. Inode Table (Starts LBA 10. We need to write Inode 2)
        // Inode 2 is Root.
        // Inode 2 is at offset 256 bytes (Inode size 256 assumed in SB?).
        // Actually SB inodes_count etc don't specify size, usually in SB (inode_size).
        // Let's assume 256 for now.
        
        // Construct Inode Table Block (containing inodes 1..N)
        let mut itable_buf = alloc::vec![0u8; 1024]; // 4 inodes (256 bytes per inode)
        // Inode 2 is at offset 256.
        
        let root_inode = DiskInode {
            mode: 0x41ED, // Dir
            uid: 0,
            size_lo: 1024,
            atime: 0, ctime: 0, mtime: 0, dtime: 0,
            gid: 0,
            links_count: 2,
            blocks_lo: 2, // 2 sectors? or 1 block? 1 block = 2 sectors.
            flags: 0,
            osd1: 0,
            block: {
                let mut b = [0; 15];
                b[0] = 20; // Data block for Root Dir (arbitrary far enough)
                b
            },
            generation: 0,
            file_acl_lo: 0,
            size_hi: 0,
            obso_faddr: 0,
        };
        
        unsafe {
             // Offset 256 (Inode 2)
             let ptr = itable_buf.as_ptr().add(256) as *mut DiskInode;
             *ptr = root_inode;
        }
        
        // Write Inode Table Block (LBA 10)
        unsafe {
            let src = itable_buf.as_ptr() as *const u16;
            let dst = ata_buf.as_mut_ptr();
            core::ptr::copy_nonoverlapping(src, dst, 512);
        }
        ata::write_sectors(&ata_buf, 10, 2);
        
        // 5. Root Directory Data Block (Block 20 -> LBA 20*2 = 40)
        // Write . and ..
        let mut dir_buf = alloc::vec![0u8; 1024];
        
        // Entry 1: .
        let entry_dot = DirEntry2 {
            inode: 2,
            rec_len: 12, // 8 + 1 + 3 align
            name_len: 1,
            file_type: 2, // Dir
        };
        
        // Entry 2: ..
        let entry_dotdot = DirEntry2 {
            inode: 2,
            rec_len: 1012, // Rest of block (1024 - 12)
            name_len: 2,
            file_type: 2,
        };
        
        unsafe {
             let ptr1 = dir_buf.as_ptr() as *mut DirEntry2;
             *ptr1 = entry_dot;
             core::ptr::copy_nonoverlapping(".".as_ptr(), dir_buf.as_mut_ptr().add(8), 1);
             
             let ptr2 = dir_buf.as_ptr().add(12) as *mut DirEntry2;
             *ptr2 = entry_dotdot;
             core::ptr::copy_nonoverlapping("..".as_ptr(), dir_buf.as_mut_ptr().add(12+8), 2);
        }
        
        // Write Dir Data (LBA 40)
        unsafe {
            let src = dir_buf.as_ptr() as *const u16;
            let dst = ata_buf.as_mut_ptr();
            core::ptr::copy_nonoverlapping(src, dst, 512);
        }
        ata::write_sectors(&ata_buf, 40, 2);
        
        crate::drivers::video::put_str("Done.\n");
        true
    }
}
