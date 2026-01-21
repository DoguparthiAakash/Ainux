use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
    Device,
}

#[derive(Debug, Clone, Copy)]
pub struct FileStat {
    pub size: u64,
    pub file_type: FileType,
    pub mode: u16,
    pub uid: u16,
    pub gid: u16,
    pub mtime: u32,
}

// Result type for VFS operations
pub type VfsResult<T> = Result<T, VfsError>;

#[derive(Debug)]
pub enum VfsError {
    NotFound,
    PermissionDenied,
    IsADirectory,
    NotADirectory,
    IOError,
    NoSpace,
    InvalidHandle,
    NotImplemented,
    NotEmpty,
    AlreadyExists,
}

pub trait FileSystem: Send + Sync {
    fn root_inode(&self) -> Arc<dyn Inode>;
}

pub trait Inode: Send + Sync {
    fn stat(&self) -> VfsResult<FileStat>;
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>>;
    fn open(&self, mode: u32) -> VfsResult<Arc<dyn FileHandle>>;
    fn create(&self, name: &str, file_type: FileType) -> VfsResult<Arc<dyn Inode>>;
    // For directories
    fn read_dir(&self) -> VfsResult<Vec<String>>;
    
    // Extended Ops
    fn mkdir(&self, name: &str) -> VfsResult<Arc<dyn Inode>>;
    fn unlink(&self, name: &str) -> VfsResult<()>;
    fn remove_dir(&self, name: &str) -> VfsResult<()>;
    fn rename(&self, old_name: &str, new_name: &str) -> VfsResult<()>;
    fn chmod(&self, mode: u16) -> VfsResult<()>;
    fn chown(&self, uid: u16, gid: u16) -> VfsResult<()>;
}

pub trait FileHandle: Send + Sync + core::fmt::Debug {
    fn read(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize>;
    fn write(&self, buf: &[u8], offset: u64) -> VfsResult<usize>;
    fn close(&self) -> VfsResult<()>;
}

pub static ROOT: Mutex<Option<Arc<dyn Inode>>> = Mutex::new(None);

pub fn init(fs: Arc<dyn FileSystem>) {
    *ROOT.lock() = Some(fs.root_inode());
}

pub fn root() -> Arc<dyn Inode> {
    ROOT.lock().as_ref().expect("VFS root not initialized").clone()
}
