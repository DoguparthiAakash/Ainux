use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::format;
use spin::Mutex;
use crate::fs::vfs::{FileSystem, Inode, FileHandle, FileStat, FileType, VfsResult, VfsError};
use crate::alloc::string::ToString;

#[derive(Debug)]
pub struct NetFileSystem;

impl FileSystem for NetFileSystem {
    fn root_inode(&self) -> Arc<dyn Inode> {
        Arc::new(NetDirInode::new_root())
    }
}

// ---------------------------------------------------------------------------
// Inode Types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct NetDirInode {
    name: String,
    entries: Mutex<Vec<(String, Arc<dyn Inode>)>>,
}

impl crate::object::KernelObject for NetDirInode {
    fn name(&self) -> String { self.name.clone() }
    fn id(&self) -> usize { 0x4000 }
    fn object_type(&self) -> &'static str { "NetDirectory" }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: Vec::new(), related_handles: Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> {
        Ok(())
    }
}

impl NetDirInode {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            entries: Mutex::new(Vec::new()),
        }
    }

    pub fn new_root() -> Self {
        let root = Self::new("net");
        
        // /net/status
        root.add_entry("status", Arc::new(NetStatusInode::new()));

        // /net/ipifc
        let ipifc = Arc::new(NetDirInode::new("ipifc"));
        let ifc0 = Arc::new(NetDirInode::new("0"));
        ifc0.add_entry("status", Arc::new(NetStatusInode::new()));
        ifc0.add_entry("ctl", Arc::new(NetIpifcCtlInode::new()));
        ipifc.add_entry("0", ifc0);
        ipifc.add_entry("clone", Arc::new(NetStaticFileInode::new("clone", "0\n")));
        root.add_entry("ipifc", ipifc);

        // /net/tcp
        let tcp = Arc::new(NetDirInode::new("tcp"));
        tcp.add_entry("clone", Arc::new(NetTcpCloneInode::new()));
        tcp.add_entry("stats", Arc::new(NetStaticFileInode::new("stats", "tcp stats: enabled\n")));
        root.add_entry("tcp", tcp);

        // /net/udp
        let udp = Arc::new(NetDirInode::new("udp"));
        udp.add_entry("clone", Arc::new(NetUdpCloneInode::new()));
        udp.add_entry("stats", Arc::new(NetStaticFileInode::new("stats", "udp stats: enabled\n")));
        root.add_entry("udp", udp);

        root
    }

    pub fn add_entry(&self, name: &str, inode: Arc<dyn Inode>) {
        let mut entries = self.entries.lock();
        entries.push((String::from(name), inode));
    }
}

impl Inode for NetDirInode {
    fn inode_num(&self) -> u32 { 0x4000 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat {
            size: 0,
            file_type: FileType::Directory,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        let entries = self.entries.lock();
        for (n, inode) in entries.iter() {
            if n == name {
                return Ok(inode.clone());
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
        let entries = self.entries.lock();
        Ok(entries.iter().map(|(n, _)| n.clone()).collect())
    }
    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn rename(&self, _old: &str, _p: Arc<dyn Inode>, _n: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _n: &str, _i: Arc<dyn Inode>) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _m: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _u: u16, _g: u16) -> VfsResult<()> { Ok(()) }
}

// ---------------------------------------------------------------------------
// /net/status & /net/ipifc/0/status Inode
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct NetStatusInode;

impl NetStatusInode {
    pub fn new() -> Self { Self }
}

impl crate::object::KernelObject for NetStatusInode {
    fn name(&self) -> String { String::from("status") }
    fn id(&self) -> usize { 0x4001 }
    fn object_type(&self) -> &'static str { "NetStatusFile" }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: Vec::new(), related_handles: Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> { Ok(()) }
}

impl Inode for NetStatusInode {
    fn inode_num(&self) -> u32 { 0x4001 }
    fn stat(&self) -> VfsResult<FileStat> {
        let content = crate::net::get_net_status();
        Ok(FileStat {
            size: content.len() as u64,
            file_type: FileType::File,
            mode: 0o444,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        let content = crate::net::get_net_status();
        Ok(Arc::new(NetMemoryHandle {
            content: content.into_bytes(),
            writable: false,
        }))
    }
    fn create(&self, _name: &str, _file_type: FileType) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::NotADirectory) }
    fn rename(&self, _old: &str, _p: Arc<dyn Inode>, _n: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _n: &str, _i: Arc<dyn Inode>) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _m: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _u: u16, _g: u16) -> VfsResult<()> { Ok(()) }
}

// ---------------------------------------------------------------------------
// /net/ipifc/0/ctl Inode (Plan 9 / Inferno interface control)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct NetIpifcCtlInode;

impl NetIpifcCtlInode {
    pub fn new() -> Self { Self }
}

impl crate::object::KernelObject for NetIpifcCtlInode {
    fn name(&self) -> String { String::from("ctl") }
    fn id(&self) -> usize { 0x4002 }
    fn object_type(&self) -> &'static str { "NetIpifcCtlFile" }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: Vec::new(), related_handles: Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> { Ok(()) }
}

impl Inode for NetIpifcCtlInode {
    fn inode_num(&self) -> u32 { 0x4002 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat {
            size: 0,
            file_type: FileType::File,
            mode: 0o660,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        Ok(Arc::new(NetIpifcCtlHandle))
    }
    fn create(&self, _name: &str, _file_type: FileType) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::NotADirectory) }
    fn rename(&self, _old: &str, _p: Arc<dyn Inode>, _n: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _n: &str, _i: Arc<dyn Inode>) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _m: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _u: u16, _g: u16) -> VfsResult<()> { Ok(()) }
}

#[derive(Debug)]
pub struct NetIpifcCtlHandle;

impl FileHandle for NetIpifcCtlHandle {
    fn read(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        let status = crate::net::get_net_status();
        let bytes = status.as_bytes();
        if offset as usize >= bytes.len() {
            return Ok(0);
        }
        let available = &bytes[offset as usize..];
        let n = core::cmp::min(buf.len(), available.len());
        buf[..n].copy_from_slice(&available[..n]);
        Ok(n)
    }

    fn write(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        let cmd = match core::str::from_utf8(buf) {
            Ok(s) => s.trim(),
            Err(_) => return Err(VfsError::IOError),
        };

        // Parse command: e.g. "add 192.168.1.50 24 [192.168.1.1]" or "ip 192.168.1.50/24 [gw]"
        let mut parts = cmd.split_whitespace();
        let verb = parts.next().unwrap_or("");
        if verb == "add" || verb == "ip" || verb == "set" {
            if let Some(ip_spec) = parts.next() {
                let (ip_str, prefix) = if let Some(slash_idx) = ip_spec.find('/') {
                    let ip = &ip_spec[..slash_idx];
                    let prefix_val = ip_spec[slash_idx + 1..].parse::<u8>().unwrap_or(24);
                    (ip, prefix_val)
                } else {
                    let prefix_val = parts.next().and_then(|p| p.parse::<u8>().ok()).unwrap_or(24);
                    (ip_spec, prefix_val)
                };

                // Parse IP bytes
                let mut ip_bytes = [0u8; 4];
                let mut octet_idx = 0;
                for octet in ip_str.split('.') {
                    if octet_idx < 4 {
                        if let Ok(b) = octet.parse::<u8>() {
                            ip_bytes[octet_idx] = b;
                            octet_idx += 1;
                        }
                    }
                }

                // Optional gateway
                let gw_bytes = parts.next().and_then(|gw_str| {
                    let mut b = [0u8; 4];
                    let mut idx = 0;
                    for oct in gw_str.split('.') {
                        if idx < 4 {
                            if let Ok(v) = oct.parse::<u8>() {
                                b[idx] = v;
                                idx += 1;
                            }
                        }
                    }
                    if idx == 4 { Some(b) } else { None }
                });

                if octet_idx == 4 {
                    crate::net::set_static_ip(ip_bytes, prefix, gw_bytes);
                    return Ok(buf.len());
                }
            }
        }
        Ok(buf.len())
    }

    fn truncate(&self) -> VfsResult<()> { Ok(()) }
    fn close(&self) -> VfsResult<()> { Ok(()) }
}

// ---------------------------------------------------------------------------
// TCP / UDP Clone Inodes (Inferno OS Connection Multiplexing)
// ---------------------------------------------------------------------------

static NEXT_TCP_CONV: Mutex<usize> = Mutex::new(0);

#[derive(Debug)]
pub struct NetTcpCloneInode;

impl NetTcpCloneInode {
    pub fn new() -> Self { Self }
}

impl crate::object::KernelObject for NetTcpCloneInode {
    fn name(&self) -> String { String::from("clone") }
    fn id(&self) -> usize { 0x4003 }
    fn object_type(&self) -> &'static str { "NetTcpClone" }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: Vec::new(), related_handles: Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> { Ok(()) }
}

impl Inode for NetTcpCloneInode {
    fn inode_num(&self) -> u32 { 0x4003 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat {
            size: 0,
            file_type: FileType::File,
            mode: 0o660,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        let mut conv = NEXT_TCP_CONV.lock();
        let id = *conv;
        *conv += 1;
        let id_str = format!("{}\n", id);
        Ok(Arc::new(NetMemoryHandle {
            content: id_str.into_bytes(),
            writable: false,
        }))
    }
    fn create(&self, _name: &str, _file_type: FileType) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::NotADirectory) }
    fn rename(&self, _old: &str, _p: Arc<dyn Inode>, _n: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _n: &str, _i: Arc<dyn Inode>) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _m: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _u: u16, _g: u16) -> VfsResult<()> { Ok(()) }
}

static NEXT_UDP_CONV: Mutex<usize> = Mutex::new(0);

#[derive(Debug)]
pub struct NetUdpCloneInode;

impl NetUdpCloneInode {
    pub fn new() -> Self { Self }
}

impl crate::object::KernelObject for NetUdpCloneInode {
    fn name(&self) -> String { String::from("clone") }
    fn id(&self) -> usize { 0x4004 }
    fn object_type(&self) -> &'static str { "NetUdpClone" }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: Vec::new(), related_handles: Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> { Ok(()) }
}

impl Inode for NetUdpCloneInode {
    fn inode_num(&self) -> u32 { 0x4004 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat {
            size: 0,
            file_type: FileType::File,
            mode: 0o660,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        let mut conv = NEXT_UDP_CONV.lock();
        let id = *conv;
        *conv += 1;
        let id_str = format!("{}\n", id);
        Ok(Arc::new(NetMemoryHandle {
            content: id_str.into_bytes(),
            writable: false,
        }))
    }
    fn create(&self, _name: &str, _file_type: FileType) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::NotADirectory) }
    fn rename(&self, _old: &str, _p: Arc<dyn Inode>, _n: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _n: &str, _i: Arc<dyn Inode>) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _m: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _u: u16, _g: u16) -> VfsResult<()> { Ok(()) }
}

// ---------------------------------------------------------------------------
// Static Inode & Memory Handle Helpers
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct NetStaticFileInode {
    name: String,
    data: String,
}

impl NetStaticFileInode {
    pub fn new(name: &str, data: &str) -> Self {
        Self {
            name: String::from(name),
            data: String::from(data),
        }
    }
}

impl crate::object::KernelObject for NetStaticFileInode {
    fn name(&self) -> String { self.name.clone() }
    fn id(&self) -> usize { 0x4005 }
    fn object_type(&self) -> &'static str { "NetStaticFile" }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: Vec::new(), related_handles: Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> { Ok(()) }
}

impl Inode for NetStaticFileInode {
    fn inode_num(&self) -> u32 { 0x4005 }
    fn stat(&self) -> VfsResult<FileStat> {
        Ok(FileStat {
            size: self.data.len() as u64,
            file_type: FileType::File,
            mode: 0o444,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn open(&self, _mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        Ok(Arc::new(NetMemoryHandle {
            content: self.data.as_bytes().to_vec(),
            writable: false,
        }))
    }
    fn create(&self, _name: &str, _file_type: FileType) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::PermissionDenied) }
    fn read_dir(&self) -> VfsResult<Vec<String>> { Err(VfsError::NotADirectory) }
    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> { Err(VfsError::NotADirectory) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn remove_dir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::NotADirectory) }
    fn rename(&self, _old: &str, _p: Arc<dyn Inode>, _n: &str) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn link(&self, _n: &str, _i: Arc<dyn Inode>) -> VfsResult<()> { Err(VfsError::PermissionDenied) }
    fn chmod(&self, _m: u16) -> VfsResult<()> { Ok(()) }
    fn chown(&self, _u: u16, _g: u16) -> VfsResult<()> { Ok(()) }
}

#[derive(Debug)]
pub struct NetMemoryHandle {
    pub content: Vec<u8>,
    pub writable: bool,
}

impl FileHandle for NetMemoryHandle {
    fn read(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        if offset as usize >= self.content.len() {
            return Ok(0);
        }
        let available = &self.content[offset as usize..];
        let n = core::cmp::min(buf.len(), available.len());
        buf[..n].copy_from_slice(&available[..n]);
        Ok(n)
    }

    fn write(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        if self.writable {
            Ok(buf.len())
        } else {
            Err(VfsError::PermissionDenied)
        }
    }

    fn truncate(&self) -> VfsResult<()> { Ok(()) }
    fn close(&self) -> VfsResult<()> { Ok(()) }
}
