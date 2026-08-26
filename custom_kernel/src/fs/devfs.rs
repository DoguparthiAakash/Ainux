use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use crate::fs::vfs::{FileSystem, Inode, FileHandle, FileStat, FileType, VfsResult, VfsError, ArcInode};
use crate::object::KernelObject;

#[derive(Debug)]
pub struct DevFs;

impl FileSystem for DevFs {
    fn root_inode(&self) -> ArcInode {
        Arc::new(DevFsRoot)
    }
}

#[derive(Debug)]
pub struct DevFsRoot;

impl KernelObject for DevFsRoot {
    fn id(&self) -> usize { 0 }
    fn object_type(&self) -> &'static str { "Directory" }
    fn name(&self) -> String { String::from("DevFsRoot") }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &str> { Err("Not implemented") }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &str> { Err("Not implemented") }
}

impl Inode for DevFsRoot {
    fn inode_num(&self) -> u32 { 1 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::Directory, mode: 0o755, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, name: &str) -> VfsResult<ArcInode> {
        match name {
            "null" => Ok(Arc::new(DevNull)),
            "zero" => Ok(Arc::new(DevZero)),
            "urandom" => Ok(Arc::new(DevURandom)),
            "dri" => Ok(Arc::new(DriDir)),
            "fb0" => Ok(Arc::new(DevFb0)),
            _ => Err(VfsError::NotFound),
        }
    }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> { Err(VfsError::IsADirectory) }
    fn create(&self, _name: &str, _type: FileType) -> VfsResult<ArcInode> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> {
        Ok(alloc::vec![String::from("null"), String::from("zero"), String::from("urandom"), String::from("dri"), String::from("fb0")])
    }
    fn mkdir(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::PermissionDenied) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _o: &str, _np: ArcInode, _nn: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _name: &str, _inode: ArcInode) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _mode: u16) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chown(&self, _uid: u16, _gid: u16) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
}

// /dev/null
#[derive(Debug)]
pub struct DevNull;

impl KernelObject for DevNull {
    fn id(&self) -> usize { 0 }
    fn object_type(&self) -> &'static str { "Device" }
    fn name(&self) -> String { String::from("DevNull") }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &str> { Err("Not implemented") }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &str> { Err("Not implemented") }
}

impl Inode for DevNull {
    fn inode_num(&self) -> u32 { 2 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::Device, mode: 0o666, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> { Ok(Arc::new(DevNullHandle)) }
    fn create(&self, _name: &str, _type: FileType) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _o: &str, _np: ArcInode, _nn: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _name: &str, _inode: ArcInode) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _mode: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _uid: u16, _gid: u16) -> VfsResult<()> { Ok(()) }
}

#[derive(Debug)]
pub struct DevNullHandle;
impl FileHandle for DevNullHandle {
    fn read(&self, _buf: &mut [u8], _offset: u64) -> VfsResult<usize> { Ok(0) } // EOF immediately
    fn write(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> { Ok(buf.len()) } // Discard
    fn truncate(&self) -> VfsResult<()> { Ok(()) }
    fn close(&self) -> VfsResult<()> { Ok(()) }
}

// /dev/zero
#[derive(Debug)]
pub struct DevZero;

impl KernelObject for DevZero {
    fn id(&self) -> usize { 0 }
    fn object_type(&self) -> &'static str { "Device" }
    fn name(&self) -> String { String::from("DevZero") }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &str> { Err("Not implemented") }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &str> { Err("Not implemented") }
}

impl Inode for DevZero {
    fn inode_num(&self) -> u32 { 3 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::Device, mode: 0o666, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> { Ok(Arc::new(DevZeroHandle)) }
    fn create(&self, _name: &str, _type: FileType) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _o: &str, _np: ArcInode, _nn: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _name: &str, _inode: ArcInode) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _mode: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _uid: u16, _gid: u16) -> VfsResult<()> { Ok(()) }
}

#[derive(Debug)]
pub struct DevZeroHandle;
impl FileHandle for DevZeroHandle {
    fn read(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        for b in buf.iter_mut() { *b = 0; }
        Ok(buf.len())
    }
    fn write(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> { Ok(buf.len()) } // Discard
    fn truncate(&self) -> VfsResult<()> { Ok(()) }
    fn close(&self) -> VfsResult<()> { Ok(()) }
}

// /dev/urandom
#[derive(Debug)]
pub struct DevURandom;

impl KernelObject for DevURandom {
    fn id(&self) -> usize { 0 }
    fn object_type(&self) -> &'static str { "Device" }
    fn name(&self) -> String { String::from("DevURandom") }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &str> { Err("Not implemented") }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &str> { Err("Not implemented") }
}

impl Inode for DevURandom {
    fn inode_num(&self) -> u32 { 6 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::Device, mode: 0o666, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> { Ok(Arc::new(DevURandomHandle)) }
    fn create(&self, _name: &str, _type: FileType) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _o: &str, _np: ArcInode, _nn: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _name: &str, _inode: ArcInode) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _mode: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _uid: u16, _gid: u16) -> VfsResult<()> { Ok(()) }
}

#[derive(Debug)]
pub struct DevURandomHandle;
impl FileHandle for DevURandomHandle {
    fn read(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        crate::lib::prng::get_random_bytes(buf);
        Ok(buf.len())
    }
    fn write(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> { Ok(buf.len()) }
    fn truncate(&self) -> VfsResult<()> { Ok(()) }
    fn close(&self) -> VfsResult<()> { Ok(()) }
}

// /dev/dri
#[derive(Debug)]
pub struct DriDir;

impl KernelObject for DriDir {
    fn id(&self) -> usize { 0 }
    fn object_type(&self) -> &'static str { "Directory" }
    fn name(&self) -> String { String::from("DriDir") }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &str> { Err("Not implemented") }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &str> { Err("Not implemented") }
}

impl Inode for DriDir {
    fn inode_num(&self) -> u32 { 4 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::Directory, mode: 0o755, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, name: &str) -> VfsResult<ArcInode> {
        if name == "card0" { Ok(Arc::new(Card0Device)) } else { Err(VfsError::NotFound) }
    }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> { Err(VfsError::IsADirectory) }
    fn create(&self, _name: &str, _type: FileType) -> VfsResult<ArcInode> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Ok(alloc::vec![String::from("card0")]) }
    fn mkdir(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::PermissionDenied) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _o: &str, _np: ArcInode, _nn: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _name: &str, _inode: ArcInode) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _mode: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _uid: u16, _gid: u16) -> VfsResult<()> { Ok(()) }
}

// /dev/dri/card0
#[derive(Debug)]
pub struct Card0Device;

impl KernelObject for Card0Device {
    fn id(&self) -> usize { 0 }
    fn object_type(&self) -> &'static str { "Device" }
    fn name(&self) -> String { String::from("Card0Device") }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &str> { Err("Not implemented") }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &str> { Err("Not implemented") }
}

impl Inode for Card0Device {
    fn inode_num(&self) -> u32 { 5 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::Device, mode: 0o666, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> { Ok(Arc::new(Card0Handle)) }
    fn create(&self, _name: &str, _type: FileType) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _o: &str, _np: ArcInode, _nn: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _name: &str, _inode: ArcInode) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _mode: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _uid: u16, _gid: u16) -> VfsResult<()> { Ok(()) }
}

#[derive(Debug)]
pub struct Card0Handle;

impl FileHandle for Card0Handle {
    fn read(&self, _buf: &mut [u8], _offset: u64) -> VfsResult<usize> { Ok(0) }
    fn write(&self, _buf: &[u8], _offset: u64) -> VfsResult<usize> { Ok(0) }
    fn truncate(&self) -> VfsResult<()> { Ok(()) }
    fn close(&self) -> VfsResult<()> { Ok(()) }

    fn ioctl(&self, request: u64, arg: u64) -> VfsResult<u64> {
        crate::drivers::drm::handle_ioctl(request, arg)
    }

    fn mmap(&self, offset: u64, size: usize) -> VfsResult<Option<u64>> {
        crate::drivers::drm::handle_mmap(offset, size)
    }
}

// /dev/fb0
#[derive(Debug)]
pub struct DevFb0;

impl KernelObject for DevFb0 {
    fn id(&self) -> usize { 0 }
    fn object_type(&self) -> &'static str { "Device" }
    fn name(&self) -> String { String::from("DevFb0") }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &str> { Err("Not implemented") }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &str> { Err("Not implemented") }
}

impl Inode for DevFb0 {
    fn inode_num(&self) -> u32 { 7 }
    fn stat(&self) -> VfsResult<FileStat> {
        let (w, h) = crate::drivers::video::get_resolution();
        let pitch = *crate::drivers::video::FRAMEBUFFER_PITCH.lock() as usize;
        let size = (pitch * h) as u64;
        Ok(FileStat { size, file_type: FileType::Device, mode: 0o666, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> { Ok(Arc::new(DevFb0Handle)) }
    fn create(&self, _name: &str, _type: FileType) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<ArcInode> { Err(VfsError::NotADirectory) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _o: &str, _np: ArcInode, _nn: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _name: &str, _inode: ArcInode) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _mode: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _uid: u16, _gid: u16) -> VfsResult<()> { Ok(()) }
}

#[derive(Debug)]
pub struct DevFb0Handle;

impl FileHandle for DevFb0Handle {
    fn read(&self, _buf: &mut [u8], _offset: u64) -> VfsResult<usize> { Ok(0) }
    fn write(&self, _buf: &[u8], _offset: u64) -> VfsResult<usize> { Ok(0) }
    fn truncate(&self) -> VfsResult<()> { Ok(()) }
    fn close(&self) -> VfsResult<()> { Ok(()) }

    fn ioctl(&self, request: u64, arg: u64) -> VfsResult<u64> {
        // FBIOGET_VSCREENINFO = 0x4600
        if request == 0x4600 {
            if crate::mm::user::validate_user_range(arg, 160) {
                let (w, h) = crate::drivers::video::get_resolution();
                let bpp = *crate::drivers::video::FRAMEBUFFER_BPP.lock() as u32;
                unsafe {
                    // xres, yres, xres_virtual, yres_virtual
                    *(arg as *mut u32) = w as u32;
                    *((arg + 4) as *mut u32) = h as u32;
                    *((arg + 8) as *mut u32) = w as u32;
                    *((arg + 12) as *mut u32) = h as u32;
                    // xoffset, yoffset, bits_per_pixel
                    *((arg + 16) as *mut u32) = 0;
                    *((arg + 20) as *mut u32) = 0;
                    *((arg + 24) as *mut u32) = bpp;
                }
                return Ok(0);
            }
        }
        // FBIOGET_FSCREENINFO = 0x4602
        if request == 0x4602 {
            if crate::mm::user::validate_user_range(arg, 80) {
                let pitch = *crate::drivers::video::FRAMEBUFFER_PITCH.lock() as u32;
                let smem_len = pitch * (*crate::drivers::video::FRAMEBUFFER_HEIGHT.lock() as u32);
                unsafe {
                    // smem_start (not mapped to userspace directly by fbdev), smem_len, type, type_aux, visual, xpanstep, ypanstep, ywrapstep, line_length
                    *((arg + 16) as *mut u32) = smem_len; // smem_len
                    *((arg + 48) as *mut u32) = pitch; // line_length
                }
                return Ok(0);
            }
        }
        Err(VfsError::PermissionDenied)
    }

    fn mmap(&self, _offset: u64, _size: usize) -> VfsResult<Option<u64>> {
        // Return physical/virtual framebuffer address to sys_mmap
        let addr = *crate::drivers::video::FRAMEBUFFER_ADDR.lock();
        if addr != 0 {
            Ok(Some(addr))
        } else {
            Err(VfsError::NotFound)
        }
    }
}
