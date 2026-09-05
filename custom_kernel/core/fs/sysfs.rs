use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::format;
use spin::Mutex;
use crate::fs::vfs::{FileSystem, Inode, FileHandle, FileStat, FileType, VfsResult, VfsError};
use crate::alloc::string::ToString;

#[derive(Debug)]
pub struct SysFileSystem;

impl FileSystem for SysFileSystem {
    fn root_inode(&self) -> Arc<dyn Inode> {
        Arc::new(SysDirInode::new())
    }
}

#[derive(Debug)]
pub struct SysDirInode {
    entries: Vec<(String, Arc<dyn Inode>)>,
}

impl crate::object::KernelObject for SysDirInode {
    fn name(&self) -> alloc::string::String { alloc::string::String::from("sysdir") }
    fn id(&self) -> usize { 0x5000 }
    fn object_type(&self) -> &'static str { "Directory" }

    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: alloc::vec::Vec::new(), related_handles: alloc::vec::Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> { Ok(()) }
}

impl SysDirInode {
    pub fn new() -> Self {
        let mut entries = Vec::new();
        // A minimal /sys/kernel directory stub
        entries.push((String::from("kernel"), Arc::new(SysKernelDirInode::new()) as Arc<dyn Inode>));
        Self { entries }
    }
}

impl Inode for SysDirInode {
    fn inode_num(&self) -> u32 { 0x5000 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::Directory, mode: 0o555, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        for (n, inode) in &self.entries {
            if n == name { return Ok(inode.clone()); }
        }
        Err(VfsError::NotFound)
    }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> { Err(VfsError::IsADirectory) }
    fn create(&self, _name: &str, _file_type: FileType) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> {
        let mut names = Vec::new();
        for (n, _) in &self.entries { names.push(n.clone()); }
        Ok(names)
    }
    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _old: &str, _p: Arc<dyn Inode>, _new: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _n: &str, _i: Arc<dyn Inode>) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _m: u16) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chown(&self, _u: u16, _g: u16) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
}

#[derive(Debug)]
pub struct SysKernelDirInode {
    entries: Vec<(String, Arc<dyn Inode>)>,
}

impl crate::object::KernelObject for SysKernelDirInode {
    fn name(&self) -> alloc::string::String { alloc::string::String::from("syskerneldir") }
    fn id(&self) -> usize { 0x5001 }
    fn object_type(&self) -> &'static str { "Directory" }

    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: alloc::vec::Vec::new(), related_handles: alloc::vec::Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> { Ok(()) }
}

impl SysKernelDirInode {
    pub fn new() -> Self {
        let mut entries = Vec::new();
        entries.push((String::from("hostname"), Arc::new(SysKernelFileInode::new(SysKernelFileType::Hostname)) as Arc<dyn Inode>));
        entries.push((String::from("capabilities"), Arc::new(SysKernelFileInode::new(SysKernelFileType::Capabilities)) as Arc<dyn Inode>));
        Self { entries }
    }
}

impl Inode for SysKernelDirInode {
    fn inode_num(&self) -> u32 { 0x5001 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::Directory, mode: 0o555, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        for (n, inode) in &self.entries {
            if n == name { return Ok(inode.clone()); }
        }
        Err(VfsError::NotFound)
    }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> { Err(VfsError::IsADirectory) }
    fn create(&self, _name: &str, _file_type: FileType) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> {
        let mut names = Vec::new();
        for (n, _) in &self.entries { names.push(n.clone()); }
        Ok(names)
    }
    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _old: &str, _p: Arc<dyn Inode>, _new: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _n: &str, _i: Arc<dyn Inode>) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _m: u16) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chown(&self, _u: u16, _g: u16) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
}

#[derive(Debug, Clone, Copy)]
pub enum SysKernelFileType {
    Hostname,
    Capabilities,
}

#[derive(Debug)]
pub struct SysKernelFileInode {
    file_type: SysKernelFileType,
}

impl crate::object::KernelObject for SysKernelFileInode {
    fn name(&self) -> alloc::string::String { alloc::format!("syskernelfile-{:?}", self.file_type) }
    fn id(&self) -> usize { 0x5002 }
    fn object_type(&self) -> &'static str { "File" }

    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: alloc::vec::Vec::new(), related_handles: alloc::vec::Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> { Ok(()) }
}

impl SysKernelFileInode {
    pub fn new(t: SysKernelFileType) -> Self { Self { file_type: t } }
}

impl Inode for SysKernelFileInode {
    fn inode_num(&self) -> u32 { 0x5002 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::File, mode: 0o444, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        let content = match self.file_type {
            SysKernelFileType::Hostname => format!("ainux-sovereign\n"),
            SysKernelFileType::Capabilities => format!("NamespaceMediation=1\nExecutionCells=1\n"),
        };
        Ok(Arc::new(SysFileHandle { content: content.into_bytes(), offset: Mutex::new(0) }))
    }
    fn create(&self, _n: &str, _t: FileType) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _old: &str, _p: Arc<dyn Inode>, _new: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _n: &str, _i: Arc<dyn Inode>) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _m: u16) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chown(&self, _u: u16, _g: u16) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
}

#[derive(Debug)]
pub struct SysFileHandle {
    content: Vec<u8>,
    offset: Mutex<usize>,
}

impl FileHandle for SysFileHandle {
    fn read(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        let mut off = self.offset.lock();
        if offset as usize >= self.content.len() { return Ok(0); }
        let start = offset as usize;
        let end = (start + buf.len()).min(self.content.len());
        let len = end - start;
        buf[..len].copy_from_slice(&self.content[start..end]);
        *off = end;
        Ok(len)
    }
    fn write(&self, _buf: &[u8], _offset: u64) -> VfsResult<usize> { Err(VfsError::PermissionDenied) }
    fn truncate(&self) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn close(&self) -> VfsResult<()> { Ok(()) }
}
