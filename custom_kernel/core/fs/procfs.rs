use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::format;
use spin::Mutex;
use crate::fs::vfs::{FileSystem, Inode, FileHandle, FileStat, FileType, VfsResult, VfsError};
use crate::alloc::string::ToString;

#[derive(Debug)]
pub struct ProcFileSystem;

impl FileSystem for ProcFileSystem {
    fn root_inode(&self) -> Arc<dyn Inode> {
        Arc::new(ProcDirInode::new())
    }
}

#[derive(Debug)]
pub struct ProcDirInode {
    entries: Vec<(String, Arc<dyn Inode>)>,
}

impl crate::object::KernelObject for ProcDirInode {
    fn name(&self) -> alloc::string::String { alloc::string::String::from("procdir") }
    fn id(&self) -> usize { 0x1000 }
    fn object_type(&self) -> &'static str { "Directory" }

    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: alloc::vec::Vec::new(), related_handles: alloc::vec::Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> {
        Ok(())
    }
}

impl ProcDirInode {
    pub fn new() -> Self {
        let mut entries = Vec::new();
        entries.push((String::from("uptime"), Arc::new(ProcFileInode::new(ProcFileType::Uptime)) as Arc<dyn Inode>));
        entries.push((String::from("meminfo"), Arc::new(ProcFileInode::new(ProcFileType::MemInfo)) as Arc<dyn Inode>));
        entries.push((String::from("version"), Arc::new(ProcFileInode::new(ProcFileType::Version)) as Arc<dyn Inode>));
        entries.push((String::from("cpuinfo"), Arc::new(ProcFileInode::new(ProcFileType::CpuInfo)) as Arc<dyn Inode>));
        Self { entries }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProcFileType {
    Uptime,
    MemInfo,
    Version,
    CpuInfo,
}

#[derive(Debug)]
pub struct ProcFileInode {
    file_type: ProcFileType,
}

impl crate::object::KernelObject for ProcFileInode {
    fn name(&self) -> alloc::string::String { alloc::format!("procfile-{:?}", self.file_type) }
    fn id(&self) -> usize { 0x2000 }
    fn object_type(&self) -> &'static str { "File" }

    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: alloc::vec::Vec::new(), related_handles: alloc::vec::Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> {
        Ok(())
    }
}

impl ProcFileInode {
    pub fn new(t: ProcFileType) -> Self {
        Self { file_type: t }
    }
}

impl Inode for ProcDirInode {
    fn inode_num(&self) -> u32 { 0x1000 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat {
            size: 0,
            file_type: FileType::Directory,
            mode: 0o555,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        for (n, inode) in &self.entries {
            if n == name {
                return Ok(inode.clone());
            }
        }
        
        // Try parsing as PID
        if let Ok(pid) = name.parse::<usize>() {
            let exists = crate::cpu::without_interrupts(|| {
                let tasks = crate::process::scheduler::TASKS.lock();
                if pid < crate::process::scheduler::MAX_TASKS && tasks[pid].is_some() {
                    true
                } else {
                    false
                }
            });
            if exists {
                return Ok(Arc::new(ProcPidDirInode { pid }));
            }
        }
        
        Err(VfsError::NotFound)
    }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        Err(VfsError::IsADirectory)
    }
    fn create(&self, _name: &str, _file_type: FileType) -> VfsResult<Arc<dyn Inode>> {
        Err(VfsError::PermissionDenied)
    }
    fn read_dir(&self) -> VfsResult<Vec<String>> {
        let mut names = Vec::new();
        for (n, _) in &self.entries {
            names.push(n.clone());
        }
        
        crate::cpu::without_interrupts(|| {
            let tasks = crate::process::scheduler::TASKS.lock();
            for (pid, t) in tasks.iter().enumerate() {
                if t.is_some() {
                    names.push(format!("{}", pid));
                }
            }
        });
        
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

impl Inode for ProcFileInode {
    fn inode_num(&self) -> u32 { 0x2000 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat {
            size: 0, // Dynamic
            file_type: FileType::File,
            mode: 0o444,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        let content = match self.file_type {
            ProcFileType::Uptime => {
                let ticks = crate::process::scheduler::get_ticks();
                format!("{:.2}\n", ticks as f64 / 100.0)
            },
            ProcFileType::MemInfo => {
                let pmm_lock = crate::mm::pmm::PMM.lock();
                let (used_frames, total_frames) = if let Some(pmm) = pmm_lock.as_ref() {
                    pmm.get_stats_fast()
                } else {
                    (0, 0)
                };
                let used_kb = used_frames * 4096 / 1024;
                let total_kb = total_frames * 4096 / 1024;
                format!("MemTotal: {:8} kB\nMemUsed:  {:8} kB\nMemFree:  {:8} kB\n", total_kb, used_kb, total_kb - used_kb)
            },
            ProcFileType::Version => {
                format!("Ainux Kernel v0.2.0-Sovereign (x86_64) Rust 1.70+\n")
            },
            ProcFileType::CpuInfo => {
                let vendor = String::from(crate::cpu::cpuid::CPU_FEATURES.lock().vendor_str());
                format!("CPU Count: {}\nVendor:    {}\nFeatures:  FPU, SSE, AVX, SMP, VMX\n", 
                    crate::cpu::percpu::get_cpu_count(),
                    vendor)
            }
        };
        Ok(Arc::new(ProcFileHandle { content: content.into_bytes(), offset: Mutex::new(0) }))
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
pub struct ProcFileHandle {
    content: Vec<u8>,
    offset: Mutex<usize>,
}

impl FileHandle for ProcFileHandle {
    fn read(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        let mut off = self.offset.lock();
        if offset as usize >= self.content.len() {
            return Ok(0);
        }
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

#[derive(Debug)]
pub struct ProcPidDirInode {
    pid: usize,
}

impl crate::object::KernelObject for ProcPidDirInode {
    fn name(&self) -> String { format!("proc-{}", self.pid) }
    fn id(&self) -> usize { 0x3000 + self.pid }
    fn object_type(&self) -> &'static str { "Directory" }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> { Ok(crate::object::ObjectSnapshot { data: Vec::new(), related_handles: Vec::new() }) }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> { Ok(()) }
}

impl Inode for ProcPidDirInode {
    fn inode_num(&self) -> u32 { (0x3000 + self.pid) as u32 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::Directory, mode: 0o555, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        if name == "status" || name == "cmdline" || name == "cwd" {
            return Ok(Arc::new(ProcPidFileInode { pid: self.pid, file_type: String::from(name) }));
        }
        Err(VfsError::NotFound)
    }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> { Err(VfsError::IsADirectory) }
    fn create(&self, _n: &str, _t: FileType) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> {
        Ok(alloc::vec![String::from("status"), String::from("cmdline"), String::from("cwd")])
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
pub struct ProcPidFileInode {
    pid: usize,
    file_type: String,
}

impl crate::object::KernelObject for ProcPidFileInode {
    fn name(&self) -> String { format!("proc-{}-{}", self.pid, self.file_type) }
    fn id(&self) -> usize { 0x4000 + self.pid }
    fn object_type(&self) -> &'static str { "File" }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> { Ok(crate::object::ObjectSnapshot { data: Vec::new(), related_handles: Vec::new() }) }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> { Ok(()) }
}

impl Inode for ProcPidFileInode {
    fn inode_num(&self) -> u32 { (0x4000 + self.pid) as u32 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat { size: 0, file_type: FileType::File, mode: 0o444, uid: 0, gid: 0, mtime: 0 })
    }
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        let mut content = String::new();
        crate::cpu::without_interrupts(|| {
            let tasks = crate::process::scheduler::TASKS.lock();
            if let Some(task) = &tasks[self.pid] {
                if self.file_type == "status" {
                    content = format!("Name:\t{}\nState:\t{:?}\nPid:\t{}\nPPid:\t{}\n", task.name, task.state, task.id, task.parent_id.unwrap_or(0));
                } else if self.file_type == "cmdline" {
                    content = format!("{}\0", task.name);
                } else if self.file_type == "cwd" {
                    content = format!("{}\n", task.cwd);
                }
            }
        });
        if content.is_empty() { return Err(VfsError::NotFound); }
        Ok(Arc::new(ProcFileHandle { content: content.into_bytes(), offset: Mutex::new(0) }))
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
