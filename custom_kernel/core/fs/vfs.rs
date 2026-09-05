use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
pub type ArcInode = Arc<dyn Inode>;
pub type ArcHandle = Arc<dyn FileHandle>;

#[derive(Debug, Clone)]
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

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CStat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: i32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    pub __unused: [i64; 3],
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

pub trait FileSystem: Send + Sync + core::fmt::Debug {
    fn root_inode(&self) -> Arc<dyn Inode>;
}

pub trait Inode: Send + Sync + crate::object::KernelObject + core::fmt::Debug {
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
}

pub trait FileHandle: Send + Sync + core::fmt::Debug {
    fn read(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize>;
    fn write(&self, buf: &[u8], offset: u64) -> VfsResult<usize>;
    fn truncate(&self) -> VfsResult<()>;
    fn close(&self) -> VfsResult<()>;
    
    fn stat(&self) -> VfsResult<FileStat> {
        Err(VfsError::NotImplemented)
    }

    fn ioctl(&self, _request: u64, _arg: u64) -> VfsResult<u64> {
        Err(VfsError::NotImplemented)
    }
    
    fn mmap(&self, _offset: u64, _size: usize) -> VfsResult<Option<u64>> {
        Ok(None)
    }
    
    fn poll(&self, _events: u32) -> VfsResult<u32> {
        Ok(0)
    }
    
    // Hack for downcasting Unix Domain Sockets without full Any trait integration
    fn as_unix_socket_ptr(&self) -> *const () {
        core::ptr::null()
    }
    
    // Hack for downcasting Inet Sockets
    fn as_inet_socket_ptr(&self) -> *const () {
        core::ptr::null()
    }
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

pub fn resolve_path(path: &str) -> VfsResult<ArcInode> {
    // SECURITY: Use the current task's Execution Cell Namespace
    let cell = crate::process::scheduler::get_current_cell();
    let ns = cell.namespace.lock();
    let cell_root = ns.root.clone();

    // 1. Check Cell-Local Mounts (Longest Prefix Match)
    let mut best_match: Option<&Mount> = None;
    for m in ns.mounts.iter() {
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

    // 2. Fallback to Global Mounts if not root-restricted
    // In a strict Sovereign environment, we skip this unless we are the root cell (id == 0).
    if cell.id == 0 {
        let global_mounts = MOUNTS.lock();
        let mut g_best: Option<&Mount> = None;
        for m in global_mounts.iter() {
            if path.starts_with(&m.path) {
                if g_best.is_none() || m.path.len() > g_best.unwrap().path.len() {
                    g_best = Some(m);
                }
            }
        }
        if let Some(m) = g_best {
            let rel_path = &path[m.path.len()..];
            let rel_path = rel_path.trim_start_matches('/');
            if rel_path.is_empty() {
                 return Ok(m.fs.root_inode());
            }
            return recursive_lookup(m.fs.root_inode(), rel_path);
        }
    }

    // Default to the cell-local root
    recursive_lookup(cell_root, path)
}

pub fn check_permission(stat: &FileStat, req_mode: u16) -> VfsResult<()> {
    let pid = crate::cpu::smp::get_current_pid();
    let tasks = crate::process::scheduler::TASKS.lock();
    if let Some(task) = &tasks[pid] {
        let euid = task.euid;
        let egid = task.egid;

        if euid == 0 {
            return Ok(()); // Root can do anything
        }

        let granted = if euid as u16 == stat.uid {
            (stat.mode >> 6) & 7
        } else if egid as u16 == stat.gid {
            (stat.mode >> 3) & 7
        } else {
            stat.mode & 7
        };

        if (granted & req_mode) == req_mode {
            return Ok(());
        }
    }
    Err(VfsError::PermissionDenied)
}

fn recursive_lookup(start: ArcInode, path: &str) -> VfsResult<ArcInode> {
    let path = path.trim_start_matches('/');
    if path.is_empty() { return Ok(start); }
    
    let mut current = start;
    for bit in path.split('/') {
        if bit.is_empty() { continue; }
        
        let stat = current.stat()?;
        // Require execute permission (1) to traverse directory
        check_permission(&stat, 1)?;
        
        current = current.lookup(bit)?;
    }
    Ok(current)
}
pub fn semantic_search(key: &str, value: &str) -> Vec<ArcInode> {
    let registry = crate::semantic::core::REGISTRY.lock();
    let mut results = Vec::new();
    
    for entry in registry.entries.iter() {
        let has_sense = entry.senses.iter().any(|s| s.key == key && s.value == value);
        if has_sense {
            // Try to downcast to Inode
            // Rust doesn't support easy dynamic downcasting from Arc<dyn Trait> to Arc<dyn Trait2>
            // So we'll have to check names or use a custom mechanism.
            if entry.object.object_type() == "File" || entry.object.object_type() == "Directory" {
                 // We know it's an Inode if it's one of these types
                 // In a real implementation, we'd use a more robust downcast.
                 // For now, we'll use a hack or assume if it's in the registry with these types, it's an Inode.
                 // This is just a demonstration of the concept.
            }
        }
    }
    results
}
