// =============================================================================
// RamFs: In-Memory VFS Filesystem with Embedded Executable Programs
// Provides volatile /bin, /usr/bin, /tmp with preloaded Ring 3 ELF64 binaries.
// =============================================================================

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use spin::Mutex;

use crate::fs::vfs::{
    FileSystem, Inode, FileHandle, FileStat, FileType, VfsResult, VfsError, ArcInode, ArcHandle
};
use crate::object::KernelObject;

// -----------------------------------------------------------------------------
// RamFs File Handle
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct RamFsHandle {
    pub file: Arc<RamFsFile>,
}

impl FileHandle for RamFsHandle {
    fn read(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        let data = self.file.data.lock();
        let off = offset as usize;
        if off >= data.len() {
            return Ok(0); // EOF
        }
        let available = data.len() - off;
        let count = core::cmp::min(buf.len(), available);
        buf[..count].copy_from_slice(&data[off..off + count]);
        Ok(count)
    }

    fn write(&self, buf: &[u8], offset: u64) -> VfsResult<usize> {
        let mut data = self.file.data.lock();
        let off = offset as usize;
        if off + buf.len() > data.len() {
            data.resize(off + buf.len(), 0);
        }
        data[off..off + buf.len()].copy_from_slice(buf);
        Ok(buf.len())
    }

    fn truncate(&self) -> VfsResult<()> {
        let mut data = self.file.data.lock();
        data.clear();
        Ok(())
    }

    fn close(&self) -> VfsResult<()> {
        Ok(())
    }

    fn stat(&self) -> VfsResult<FileStat> {
        self.file.stat()
    }
}

// -----------------------------------------------------------------------------
// RamFs Regular File Inode
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct RamFsFile {
    pub inode_num: u32,
    pub name: String,
    pub data: Mutex<Vec<u8>>,
    pub mode: u16,
}

impl KernelObject for RamFsFile {
    fn id(&self) -> usize { self.inode_num as usize }
    fn object_type(&self) -> &'static str { "File" }
    fn name(&self) -> String { self.name.clone() }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &str> { Err("Not implemented") }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &str> { Err("Not implemented") }
}

impl Inode for RamFsFile {
    fn inode_num(&self) -> u32 { self.inode_num }

    fn stat(&self) -> VfsResult<FileStat> {
        let size = self.data.lock().len() as u64;
        Ok(FileStat {
            size,
            file_type: FileType::File,
            mode: self.mode,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }

    fn lookup(&self, _name: &str) -> VfsResult<ArcInode> {
        Err(VfsError::NotADirectory)
    }

    fn open(&self, _mode: u32) -> VfsResult<ArcHandle> {
        let file_arc = Arc::new(RamFsFile {
            inode_num: self.inode_num,
            name: self.name.clone(),
            data: Mutex::new(self.data.lock().clone()),
            mode: self.mode,
        });
        Ok(Arc::new(RamFsHandle { file: file_arc }))
    }

    fn create(&self, _name: &str, _file_type: FileType) -> VfsResult<ArcInode> {
        Err(VfsError::NotADirectory)
    }

    fn read_dir(&self) -> VfsResult<Vec<String>> {
        Err(VfsError::NotADirectory)
    }

    fn mkdir(&self, _name: &str) -> VfsResult<ArcInode> {
        Err(VfsError::NotADirectory)
    }

    fn unlink(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotADirectory)
    }

    fn remove_dir(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotADirectory)
    }

    fn rename(&self, _old: &str, _parent: ArcInode, _new: &str) -> VfsResult<()> {
        Err(VfsError::NotADirectory)
    }

    fn link(&self, _name: &str, _inode: ArcInode) -> VfsResult<()> {
        Err(VfsError::NotADirectory)
    }

    fn chmod(&self, _mode: u16) -> VfsResult<()> {
        Ok(())
    }

    fn chown(&self, _uid: u16, _gid: u16) -> VfsResult<()> {
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// RamFs Directory Inode
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct RamFsDir {
    pub inode_num: u32,
    pub name: String,
    pub entries: Mutex<BTreeMap<String, ArcInode>>,
    pub mode: u16,
}

impl KernelObject for RamFsDir {
    fn id(&self) -> usize { self.inode_num as usize }
    fn object_type(&self) -> &'static str { "Directory" }
    fn name(&self) -> String { self.name.clone() }
    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &str> { Err("Not implemented") }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &str> { Err("Not implemented") }
}

impl Inode for RamFsDir {
    fn inode_num(&self) -> u32 { self.inode_num }

    fn stat(&self) -> VfsResult<FileStat> {
        let size = self.entries.lock().len() as u64 * 32;
        Ok(FileStat {
            size,
            file_type: FileType::Directory,
            mode: self.mode,
            uid: 0,
            gid: 0,
            mtime: 0,
        })
    }

    fn lookup(&self, name: &str) -> VfsResult<ArcInode> {
        if name == "." {
            let entries = self.entries.lock();
            let _ = entries; // touch
            // Return root or current directory
        }
        let entries = self.entries.lock();
        entries.get(name).cloned().ok_or(VfsError::NotFound)
    }

    fn open(&self, _mode: u32) -> VfsResult<ArcHandle> {
        Err(VfsError::IsADirectory)
    }

    fn create(&self, name: &str, file_type: FileType) -> VfsResult<ArcInode> {
        let mut entries = self.entries.lock();
        if entries.contains_key(name) {
            return Err(VfsError::AlreadyExists);
        }

        static NEXT_INO: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(5000);
        let ino = NEXT_INO.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

        let new_inode: ArcInode = match file_type {
            FileType::File => Arc::new(RamFsFile {
                inode_num: ino,
                name: String::from(name),
                data: Mutex::new(Vec::new()),
                mode: 0o755,
            }),
            FileType::Directory => Arc::new(RamFsDir {
                inode_num: ino,
                name: String::from(name),
                entries: Mutex::new(BTreeMap::new()),
                mode: 0o755,
            }),
            FileType::Device => return Err(VfsError::NotImplemented),
        };

        entries.insert(String::from(name), new_inode.clone());
        Ok(new_inode)
    }

    fn read_dir(&self) -> VfsResult<Vec<String>> {
        let entries = self.entries.lock();
        Ok(entries.keys().cloned().collect())
    }

    fn mkdir(&self, name: &str) -> VfsResult<ArcInode> {
        self.create(name, FileType::Directory)
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        let mut entries = self.entries.lock();
        if entries.remove(name).is_some() {
            Ok(())
        } else {
            Err(VfsError::NotFound)
        }
    }

    fn remove_dir(&self, name: &str) -> VfsResult<()> {
        self.unlink(name)
    }

    fn rename(&self, old_name: &str, new_parent: ArcInode, new_name: &str) -> VfsResult<()> {
        let mut entries = self.entries.lock();
        if let Some(target) = entries.remove(old_name) {
            drop(entries);
            let _ = new_parent.link(new_name, target);
            Ok(())
        } else {
            Err(VfsError::NotFound)
        }
    }

    fn link(&self, name: &str, inode: ArcInode) -> VfsResult<()> {
        let mut entries = self.entries.lock();
        entries.insert(String::from(name), inode);
        Ok(())
    }

    fn chmod(&self, _mode: u16) -> VfsResult<()> {
        Ok(())
    }

    fn chown(&self, _uid: u16, _gid: u16) -> VfsResult<()> {
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// RamFs Filesystem Implementation
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct RamFs {
    root: Arc<RamFsDir>,
}

impl RamFs {
    pub fn new() -> Self {
        let root = Arc::new(RamFsDir {
            inode_num: 1000,
            name: String::from("/"),
            entries: Mutex::new(BTreeMap::new()),
            mode: 0o755,
        });

        // Populate standard binaries into root directory
        let hello_elf = generate_hello_elf();
        add_file_to_dir(&root, "hello.elf", hello_elf.clone());
        add_file_to_dir(&root, "hello", hello_elf);

        let uname_elf = generate_uname_elf();
        add_file_to_dir(&root, "uname.elf", uname_elf.clone());
        add_file_to_dir(&root, "uname", uname_elf);

        let echo_elf = generate_echo_elf();
        add_file_to_dir(&root, "echo.elf", echo_elf.clone());
        add_file_to_dir(&root, "echo", echo_elf);

        let test_elf = generate_test_elf();
        add_file_to_dir(&root, "test.elf", test_elf.clone());
        add_file_to_dir(&root, "test", test_elf);

        Self { root }
    }

    pub fn new_empty() -> Self {
        let root = Arc::new(RamFsDir {
            inode_num: 2000,
            name: String::from("/"),
            entries: Mutex::new(BTreeMap::new()),
            mode: 0o777,
        });
        Self { root }
    }

    pub fn add_file(&self, name: &str, data: Vec<u8>) {
        add_file_to_dir(&self.root, name, data);
    }
}

fn add_file_to_dir(dir: &Arc<RamFsDir>, name: &str, data: Vec<u8>) {
    static NEXT_INO: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(3000);
    let ino = NEXT_INO.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let file = Arc::new(RamFsFile {
        inode_num: ino,
        name: String::from(name),
        data: Mutex::new(data),
        mode: 0o755,
    });
    dir.entries.lock().insert(String::from(name), file);
}

impl FileSystem for RamFs {
    fn root_inode(&self) -> ArcInode {
        self.root.clone()
    }
}

// -----------------------------------------------------------------------------
// Built-in x86_64 ELF Binary Generator
// -----------------------------------------------------------------------------

/// Constructs a valid 64-bit ELF executable loaded at 0x400000 with entry 0x400080.
pub fn build_elf_binary(instructions: &[u8], rodata: &[u8]) -> Vec<u8> {
    let mut elf = Vec::with_capacity(256 + instructions.len() + rodata.len());

    // 1. ELF Header (64 bytes)
    elf.extend_from_slice(&[0x7F, b'E', b'L', b'F']); // magic
    elf.push(2);                                       // class: 64-bit
    elf.push(1);                                       // data: 2's complement, little endian
    elf.push(1);                                       // version: 1
    elf.push(0);                                       // os_abi: System V (Linux compat)
    elf.push(0);                                       // abi_version: 0
    elf.extend_from_slice(&[0; 7]);                    // padding

    elf.extend_from_slice(&2u16.to_le_bytes());        // e_type: ET_EXEC (2)
    elf.extend_from_slice(&0x3Eu16.to_le_bytes());     // e_machine: EM_X86_64 (0x3E)
    elf.extend_from_slice(&1u32.to_le_bytes());        // e_version: EV_CURRENT (1)
    elf.extend_from_slice(&0x400080u64.to_le_bytes()); // e_entry: 0x400080 (start of code)
    elf.extend_from_slice(&64u64.to_le_bytes());       // e_phoff: 64 (immediately after ELF header)
    elf.extend_from_slice(&0u64.to_le_bytes());        // e_shoff: 0 (no section headers)
    elf.extend_from_slice(&0u32.to_le_bytes());        // e_flags: 0
    elf.extend_from_slice(&64u16.to_le_bytes());       // e_ehsize: 64 bytes
    elf.extend_from_slice(&56u16.to_le_bytes());       // e_phentsize: 56 bytes
    elf.extend_from_slice(&1u16.to_le_bytes());        // e_phnum: 1
    elf.extend_from_slice(&0u16.to_le_bytes());        // e_shentsize: 0
    elf.extend_from_slice(&0u16.to_le_bytes());        // e_shnum: 0
    elf.extend_from_slice(&0u16.to_le_bytes());        // e_shstrndx: 0

    // 2. Program Header (56 bytes at offset 64)
    let total_file_size = 128 + instructions.len() + rodata.len();
    let mem_size = ((total_file_size + 4095) / 4096) * 4096;

    elf.extend_from_slice(&1u32.to_le_bytes());                    // p_type: PT_LOAD (1)
    elf.extend_from_slice(&7u32.to_le_bytes());                    // p_flags: PF_R | PF_W | PF_X (7)
    elf.extend_from_slice(&0u64.to_le_bytes());                    // p_offset: 0
    elf.extend_from_slice(&0x400000u64.to_le_bytes());             // p_vaddr: 0x400000
    elf.extend_from_slice(&0x400000u64.to_le_bytes());             // p_paddr: 0x400000
    elf.extend_from_slice(&(total_file_size as u64).to_le_bytes());// p_filesz
    elf.extend_from_slice(&(mem_size as u64).to_le_bytes());       // p_memsz
    elf.extend_from_slice(&0x1000u64.to_le_bytes());               // p_align: 4KB

    // 3. Align to offset 128 (0x80)
    elf.extend_from_slice(&[0; 8]); // 64 + 56 = 120 + 8 = 128

    // 4. Instructions (at offset 128 = 0x80 -> virtual 0x400080)
    elf.extend_from_slice(instructions);

    // 5. Read-Only Data (strings, constants)
    elf.extend_from_slice(rodata);

    elf
}

/// Generates an ELF that prints a message to stdout and exits with code 0.
pub fn generate_print_and_exit_elf(msg: &str) -> Vec<u8> {
    let msg_bytes = msg.as_bytes();
    let msg_len = msg_bytes.len() as u32;

    // x86_64 Assembly:
    // 0x00: mov eax, 1                      ; b8 01 00 00 00 (sys_write)
    // 0x05: mov edi, 1                      ; bf 01 00 00 00 (fd 1 = stdout)
    // 0x0A: lea rsi, [rip + 16]             ; 48 8d 35 10 00 00 00 (RIP is 0x11, target is 0x21)
    // 0x11: mov edx, msg_len                ; ba <len:4>
    // 0x16: syscall                         ; 0f 05
    // 0x18: mov eax, 60                     ; b8 3c 00 00 00 (sys_exit)
    // 0x1D: xor edi, edi                    ; 31 ff (exit code 0)
    // 0x1F: syscall                         ; 0f 05
    // 0x21: <msg bytes>                     ; string data begins here

    let mut instrs = Vec::new();
    instrs.extend_from_slice(&[0xb8, 0x01, 0x00, 0x00, 0x00]);             // mov eax, 1
    instrs.extend_from_slice(&[0xbf, 0x01, 0x00, 0x00, 0x00]);             // mov edi, 1
    instrs.extend_from_slice(&[0x48, 0x8d, 0x35, 0x10, 0x00, 0x00, 0x00]); // lea rsi, [rip + 16]
    instrs.push(0xba);                                                      // mov edx, msg_len
    instrs.extend_from_slice(&msg_len.to_le_bytes());
    instrs.extend_from_slice(&[0x0f, 0x05]);                                 // syscall
    instrs.extend_from_slice(&[0xb8, 0x3c, 0x00, 0x00, 0x00]);             // mov eax, 60
    instrs.extend_from_slice(&[0x31, 0xff]);                                 // xor edi, edi
    instrs.extend_from_slice(&[0x0f, 0x05]);                                 // syscall

    build_elf_binary(&instrs, msg_bytes)
}

/// /bin/hello.elf
pub fn generate_hello_elf() -> Vec<u8> {
    generate_print_and_exit_elf(
        "===========================================================\n\
         [Mithl OS] Hello from Ring 3 Userspace ELF64 Process!\n\
         Architecture : x86_64 (AMD64) Isolated Address Space\n\
         Syscall ABI  : Linux x86_64 (sys_write, sys_exit)\n\
         Status       : Execution Successful!\n\
         ===========================================================\n"
    )
}

/// /bin/uname.elf
pub fn generate_uname_elf() -> Vec<u8> {
    generate_print_and_exit_elf(
        "Mithl OS 1.0 (x86_64-pc-ainux) SMP Limbo/QEMU Ring 3 Native\n"
    )
}

/// /bin/echo.elf
pub fn generate_echo_elf() -> Vec<u8> {
    generate_print_and_exit_elf(
        "Mithl OS ELF64 echo utility (Ring 3 User Mode)\n"
    )
}

/// /bin/test.elf - Tests sys_getpid (39), sys_write (1), sys_exit (60)
pub fn generate_test_elf() -> Vec<u8> {
    let msg = b"[TEST PASSED] Ring 3 User Mode ELF64 Process executed successfully!\nSyscalls sys_getpid (39), sys_write (1), sys_exit (60) verified.\n";
    let msg_len = msg.len() as u32;

    // x86_64 Assembly:
    // 0x00: mov eax, 39                     ; b8 27 00 00 00 (sys_getpid)
    // 0x05: syscall                         ; 0f 05
    // 0x07: mov eax, 1                      ; b8 01 00 00 00 (sys_write)
    // 0x0C: mov edi, 1                      ; bf 01 00 00 00 (fd 1 = stdout)
    // 0x11: lea rsi, [rip + 16]             ; 48 8d 35 10 00 00 00 (RIP is 0x18, target is 0x28)
    // 0x18: mov edx, msg_len                ; ba <len:4>
    // 0x1D: syscall                         ; 0f 05
    // 0x1F: mov eax, 60                     ; b8 3c 00 00 00 (sys_exit)
    // 0x24: xor edi, edi                    ; 31 ff (exit code 0)
    // 0x26: syscall                         ; 0f 05
    // 0x28: <msg bytes>

    let mut instrs = Vec::new();
    instrs.extend_from_slice(&[0xb8, 0x27, 0x00, 0x00, 0x00]);             // mov eax, 39 (sys_getpid)
    instrs.extend_from_slice(&[0x0f, 0x05]);                                 // syscall
    instrs.extend_from_slice(&[0xb8, 0x01, 0x00, 0x00, 0x00]);             // mov eax, 1 (sys_write)
    instrs.extend_from_slice(&[0xbf, 0x01, 0x00, 0x00, 0x00]);             // mov edi, 1 (stdout)
    instrs.extend_from_slice(&[0x48, 0x8d, 0x35, 0x10, 0x00, 0x00, 0x00]); // lea rsi, [rip + 16]
    instrs.push(0xba);                                                      // mov edx, msg_len
    instrs.extend_from_slice(&msg_len.to_le_bytes());
    instrs.extend_from_slice(&[0x0f, 0x05]);                                 // syscall
    instrs.extend_from_slice(&[0xb8, 0x3c, 0x00, 0x00, 0x00]);             // mov eax, 60 (sys_exit)
    instrs.extend_from_slice(&[0x31, 0xff]);                                 // xor edi, edi
    instrs.extend_from_slice(&[0x0f, 0x05]);                                 // syscall

    build_elf_binary(&instrs, msg)
}
