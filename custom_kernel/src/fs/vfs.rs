use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;

#[derive(Default, Debug, Clone, Copy)]
pub struct UsageStats {
    pub total_size: u64,
    pub image_size: u64,
    pub system_size: u64,
    pub alo_size: u64,
}

pub static USAGE_CACHE: Mutex<UsageStats> = Mutex::new(UsageStats {
    total_size: 0,
    image_size: 0,
    system_size: 0,
    alo_size: 0,
});

pub fn update_vfs_usage(delta: i64, name: &str) {
    let mut cache = USAGE_CACHE.lock();
    cache.total_size = (cache.total_size as i64 + delta) as u64;
    
    if name.ends_with(".bmp") || name.ends_with(".png") {
        cache.image_size = (cache.image_size as i64 + delta) as u64;
    } else if name.ends_with(".nux") || name.ends_with(".sys") {
        cache.system_size = (cache.system_size as i64 + delta) as u64;
    } else if name.ends_with(".alo") || name.ends_with(".rs") || name.ends_with(".c") || name.ends_with(".zig") {
        cache.alo_size = (cache.alo_size as i64 + delta) as u64;
    }
}
pub type ArcInode = Arc<dyn Inode>;
pub type ArcHandle = Arc<dyn FileHandle>;

pub struct Mount {
    pub path: String,
    pub fs: Arc<dyn FileSystem>,
}

pub static MOUNTS: Mutex<Vec<Mount>> = Mutex::new(Vec::new());

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
    fn inode_num(&self) -> u32;
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
    fn rename(&self, old_name: &str, new_parent: Arc<dyn Inode>, new_name: &str) -> VfsResult<()>;
    fn link(&self, name: &str, inode: Arc<dyn Inode>) -> VfsResult<()>;
    fn chmod(&self, mode: u16) -> VfsResult<()>;
    fn chown(&self, uid: u16, gid: u16) -> VfsResult<()>;

    // Visualization Support
    fn get_occupancy(&self) -> Option<Vec<u8>> { None }
}

pub trait FileHandle: Send + Sync + core::fmt::Debug {
    fn read(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize>;
    fn write(&self, buf: &[u8], offset: u64) -> VfsResult<usize>;
    fn truncate(&self) -> VfsResult<()>;
    fn close(&self) -> VfsResult<()>;
}

pub static ROOT: Mutex<Option<Arc<dyn Inode>>> = Mutex::new(None);

pub fn init(fs: Arc<dyn FileSystem>) {
    *ROOT.lock() = Some(fs.root_inode());
}

pub fn root() -> Arc<dyn Inode> {
    ROOT.lock().as_ref().expect("VFS root not initialized").clone()
}

pub fn mount(path: &str, fs: Arc<dyn FileSystem>) {
    let mut mounts = MOUNTS.lock();
    mounts.push(Mount {
        path: String::from(path),
        fs,
    });
}

pub fn resolve_path(path: &str) -> VfsResult<Arc<dyn Inode>> {
    // Check mounts first (longest prefix match)
    let mounts = MOUNTS.lock();
    let mut best_match: Option<&Mount> = None;
    
    for m in mounts.iter() {
        if path.starts_with(&m.path) {
            if best_match.is_none() || m.path.len() > best_match.unwrap().path.len() {
                best_match = Some(m);
            }
        }
    }
    
    if let Some(m) = best_match {
        let rel_path = &path[m.path.len()..];
        let rel_path = rel_path.trim_start_matches('/');
        if rel_path.is_empty() {
             return Ok(m.fs.root_inode());
        }
        return recursive_lookup(m.fs.root_inode(), rel_path);
    }
    
    // Default to root
    recursive_lookup(root(), path)
}

fn recursive_lookup(start: ArcInode, path: &str) -> VfsResult<ArcInode> {
    let path = path.trim_start_matches('/');
    if path.is_empty() { return Ok(start); }
    
    let mut current = start;
    for bit in path.split('/') {
        if bit.is_empty() { continue; }
        current = current.lookup(bit)?;
    }
    Ok(current)
}
