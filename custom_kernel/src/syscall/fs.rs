use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use spin::Mutex;
use crate::fs::vfs::FileHandle;
use lazy_static::lazy_static;
use core::slice;

// For now, since we only have one main process context, a global FD table will suffice.
// In a real OS, this would be per-process in the Process Control Block (PCB).
lazy_static! {
    pub static ref FD_TABLE: Mutex<BTreeMap<usize, Arc<dyn FileHandle>>> = Mutex::new(BTreeMap::new());
}

lazy_static! {
    static ref NEXT_FD: Mutex<usize> = Mutex::new(3); // 0=stdin, 1=stdout, 2=stderr
}

pub fn sys_open(path_ptr: *const u8, flags: i32, mode: i32) -> isize {
    // We need to read a null-terminated string from userspace.
    // WARNING: This assumes path_ptr is valid. In a real OS, we must validate userspace pointers!
    let mut len = 0;
    unsafe {
        while *path_ptr.add(len) != 0 {
            len += 1;
            if len > 4096 {
                return -36; // ENAMETOOLONG
            }
        }
    }
    let path_slice = unsafe { slice::from_raw_parts(path_ptr, len) };
    let path_str = match core::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return -22, // EINVAL
    };
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_open: path={}, flags={}, mode={}\n", path_str, flags, mode);

    // Call the VFS open here.
    if let Ok(inode) = crate::fs::vfs::resolve_path(path_str) {
        if let Ok(stat) = inode.stat() {
            let is_read = (flags & 3) == 0 || (flags & 3) == 2;
            let is_write = (flags & 3) == 1 || (flags & 3) == 2;
            
            let mut req_mode = 0;
            if is_read { req_mode |= 4; }
            if is_write { req_mode |= 2; }
            
            if crate::fs::vfs::check_permission(&stat, req_mode).is_err() {
                return -13; // EACCES
            }
            
            if let Ok(handle) = inode.open(flags as u32) {
                let mut table = FD_TABLE.lock();
                let mut next_fd = NEXT_FD.lock();
                let fd = *next_fd;
                *next_fd += 1;
                table.insert(fd, handle);
                return fd as isize;
            }
        }
    } else if (flags & 64) != 0 {
        // O_CREAT is set, try to create the file
        let (parent_path, name) = match path_str.rsplit_once('/') {
            Some((p, n)) => (if p.is_empty() { "/" } else { p }, n),
            None => (".", path_str),
        };
        if let Ok(parent) = crate::fs::vfs::resolve_path(parent_path) {
            if let Ok(stat) = parent.stat() {
                if crate::fs::vfs::check_permission(&stat, 2).is_ok() {
                    if let Ok(new_inode) = parent.create(name, crate::fs::vfs::FileType::File) {
                        if let Ok(handle) = new_inode.open(flags as u32) {
                            let mut table = FD_TABLE.lock();
                            let mut next_fd = NEXT_FD.lock();
                            let fd = *next_fd;
                            *next_fd += 1;
                            table.insert(fd, handle);
                            return fd as isize;
                        }
                    }
                }
            }
        }
    }
    
    // Fallback if VFS fails (temporarily keep dummy FDs for unresolved standard streams/tests)
    let mut next_fd = NEXT_FD.lock();
    let fd = *next_fd;
    *next_fd += 1;
    fd as isize
}

pub fn sys_read(fd: usize, buf: *mut u8, count: usize) -> isize {
    let table = FD_TABLE.lock();
    if let Some(handle) = table.get(&fd) {
        let slice = unsafe { slice::from_raw_parts_mut(buf, count) };
        match handle.read(slice, 0) { // Offset tracking should be per-FD, omitted for simplicity
            Ok(bytes) => bytes as isize,
            Err(_) => -5, // EIO
        }
    } else {
        -9 // EBADF
    }
}

pub fn sys_write(fd: usize, buf: *const u8, count: usize) -> isize {
    if fd == 1 || fd == 2 {
        // stdout / stderr -> print to kernel console
        let slice = unsafe { slice::from_raw_parts(buf, count) };
        if let Ok(s) = core::str::from_utf8(slice) {
            let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
            use core::fmt::Write;
            let _ = write!(serial, "{}", s);
        }
        return count as isize;
    }

    let table = FD_TABLE.lock();
    if let Some(handle) = table.get(&fd) {
        let slice = unsafe { slice::from_raw_parts(buf, count) };
        match handle.write(slice, 0) {
            Ok(bytes) => bytes as isize,
            Err(_) => -5, // EIO
        }
    } else {
        -9 // EBADF
    }
}

pub fn sys_close(fd: usize) -> isize {
    let mut table = FD_TABLE.lock();
    if let Some(handle) = table.remove(&fd) {
        let _ = handle.close();
        0
    } else {
        -9 // EBADF
    }
}

pub const AT_FDCWD: i32 = -100;

#[repr(C)]
pub struct IoVec {
    pub iov_base: usize,
    pub iov_len: usize,
}

pub fn sys_openat(dirfd: i32, path_ptr: *const u8, flags: i32, mode: i32) -> isize {
    // For now we assume all paths are absolute or relative to root
    sys_open(path_ptr, flags, mode)
}

pub fn sys_readv(fd: usize, iov_ptr: *const u8, iovcnt: usize) -> isize {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_readv: fd={}, iovcnt={}\n", fd, iovcnt);
    -38 // ENOSYS
}

pub fn sys_writev(fd: usize, iov_ptr: *const u8, iovcnt: usize) -> isize {
    // Writev to stdout/stderr
    if fd == 1 || fd == 2 {
        let mut bytes_written = 0;
        let iovs = unsafe { slice::from_raw_parts(iov_ptr as *const IoVec, iovcnt) };
        for iov in iovs {
            let base = iov.iov_base as *const u8;
            let len = iov.iov_len;
            let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
            unsafe {
                for chunk in core::slice::from_raw_parts(base, len) {
                    serial.write_byte(*chunk);
                }
            }
            bytes_written += len;
        }
        return bytes_written as isize;
    }
    
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_writev: fd={}, iovcnt={}\n", fd, iovcnt);
    -38 // ENOSYS
}

pub fn sys_ioctl(fd: usize, request: usize, argp: usize) -> isize {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_ioctl: fd={}, req={:#x}, arg={:#x}\n", fd, request, argp);
    
    // Stub for TCGETS (0x5401) on stdout/stdin to tell musl it's a TTY (or not)
    // -25 is ENOTTY
    -25
}

pub fn sys_disk_read(lba: u32, sectors: u8, buf: *mut u8) -> isize {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_disk_read: lba={}, sectors={}\n", lba, sectors);
    
    let total_bytes = (sectors as usize) * 512;
    let mut tmp = alloc::vec![0u16; (sectors as usize) * 256];
    
    if crate::drivers::ata::read_sectors(&mut tmp, lba, sectors) {
        let tmp_bytes = unsafe { core::slice::from_raw_parts(tmp.as_ptr() as *const u8, total_bytes) };
        let user_slice = unsafe { core::slice::from_raw_parts_mut(buf, total_bytes) };
        user_slice.copy_from_slice(tmp_bytes);
        total_bytes as isize
    } else {
        -1
    }
}

pub fn sys_disk_write(lba: u32, sectors: u8, buf: *const u8) -> isize {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_disk_write: lba={}, sectors={}\n", lba, sectors);
    
    let total_bytes = (sectors as usize) * 512;
    let user_slice = unsafe { core::slice::from_raw_parts(buf, total_bytes) };
    let mut tmp = alloc::vec![0u16; (sectors as usize) * 256];
    
    let tmp_bytes = unsafe { core::slice::from_raw_parts_mut(tmp.as_mut_ptr() as *mut u8, total_bytes) };
    tmp_bytes.copy_from_slice(user_slice);
    
    if crate::drivers::ata::write_sectors(&tmp, lba, sectors) {
        total_bytes as isize
    } else {
        -1
    }
}

pub fn sys_chmod(path_ptr: *const u8, mode: u16) -> isize {
    let mut len = 0;
    unsafe {
        while *path_ptr.add(len) != 0 {
            len += 1;
            if len > 4096 { return -36; } // ENAMETOOLONG
        }
    }
    let path_slice = unsafe { slice::from_raw_parts(path_ptr, len) };
    let path_str = match core::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return -22, // EINVAL
    };
    
    if let Ok(inode) = crate::fs::vfs::resolve_path(path_str) {
        if let Ok(stat) = inode.stat() {
            // Must be owner or root
            let pid = crate::cpu::smp::get_current_pid();
            let tasks = crate::process::scheduler::TASKS.lock();
            let mut is_owner_or_root = false;
            if let Some(task) = &tasks[pid] {
                if task.euid == 0 || task.euid as u16 == stat.uid {
                    is_owner_or_root = true;
                }
            }
            drop(tasks);
            
            if is_owner_or_root {
                if inode.chmod(mode).is_ok() {
                    return 0;
                }
            } else {
                return -1; // EPERM
            }
        }
    }
    -2 // ENOENT
}

pub fn sys_chown(path_ptr: *const u8, uid: u16, gid: u16) -> isize {
    let mut len = 0;
    unsafe {
        while *path_ptr.add(len) != 0 {
            len += 1;
            if len > 4096 { return -36; }
        }
    }
    let path_slice = unsafe { slice::from_raw_parts(path_ptr, len) };
    let path_str = match core::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return -22,
    };
    
    if let Ok(inode) = crate::fs::vfs::resolve_path(path_str) {
        if let Ok(stat) = inode.stat() {
            let pid = crate::cpu::smp::get_current_pid();
            let tasks = crate::process::scheduler::TASKS.lock();
            let mut can_chown = false;
            if let Some(task) = &tasks[pid] {
                // Only root can change owner. 
                // Owner can change group if they belong to that group (simplified: only root for now).
                if task.euid == 0 {
                    can_chown = true;
                }
            }
            drop(tasks);
            
            if can_chown {
                if inode.chown(uid, gid).is_ok() {
                    return 0;
                }
            } else {
                return -1; // EPERM
            }
        }
    }
    -2 // ENOENT
}


pub fn sys_disk_identify(buf: *mut u8) -> isize {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_disk_identify\n");
    
    let total_bytes = 512;
    let mut tmp = alloc::vec![0u16; 256];
    
    if crate::drivers::ata::identify_buffer(&mut tmp) {
        let tmp_bytes = unsafe { core::slice::from_raw_parts(tmp.as_ptr() as *const u8, total_bytes) };
        let user_slice = unsafe { core::slice::from_raw_parts_mut(buf, total_bytes) };
        user_slice.copy_from_slice(tmp_bytes);
        total_bytes as isize
    } else {
        -1
    }
}

// Helper to extract string from userspace pointer
fn get_user_string(ptr: *const u8) -> Result<&'static str, isize> {
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
            if len > 4096 { return Err(-36); } // ENAMETOOLONG
        }
    }
    let slice = unsafe { slice::from_raw_parts(ptr, len) };
    core::str::from_utf8(slice).map_err(|_| -22) // EINVAL
}

pub fn sys_mkdir(path_ptr: *const u8, _mode: u16) -> isize {
    let path = match get_user_string(path_ptr) {
        Ok(s) => s,
        Err(e) => return e,
    };
    
    // Find parent directory
    let (parent_path, name) = match path.rsplit_once('/') {
        Some((p, n)) => (if p.is_empty() { "/" } else { p }, n),
        None => (".", path),
    };
    
    if let Ok(parent) = crate::fs::vfs::resolve_path(parent_path) {
        if let Ok(stat) = parent.stat() {
            if crate::fs::vfs::check_permission(&stat, 2).is_err() { // need write to parent
                return -13; // EACCES
            }
            if parent.mkdir(name).is_ok() {
                return 0;
            }
        }
    }
    -2 // ENOENT
}

pub fn sys_rmdir(path_ptr: *const u8) -> isize {
    let path = match get_user_string(path_ptr) {
        Ok(s) => s,
        Err(e) => return e,
    };
    
    let (parent_path, name) = match path.rsplit_once('/') {
        Some((p, n)) => (if p.is_empty() { "/" } else { p }, n),
        None => (".", path),
    };
    
    if let Ok(parent) = crate::fs::vfs::resolve_path(parent_path) {
        if let Ok(stat) = parent.stat() {
            if crate::fs::vfs::check_permission(&stat, 2).is_err() {
                return -13;
            }
            if parent.remove_dir(name).is_ok() {
                return 0;
            }
        }
    }
    -2
}

pub fn sys_unlink(path_ptr: *const u8) -> isize {
    let path = match get_user_string(path_ptr) {
        Ok(s) => s,
        Err(e) => return e,
    };
    
    let (parent_path, name) = match path.rsplit_once('/') {
        Some((p, n)) => (if p.is_empty() { "/" } else { p }, n),
        None => (".", path),
    };
    
    if let Ok(parent) = crate::fs::vfs::resolve_path(parent_path) {
        if let Ok(stat) = parent.stat() {
            if crate::fs::vfs::check_permission(&stat, 2).is_err() {
                return -13;
            }
            if parent.unlink(name).is_ok() {
                return 0;
            }
        }
    }
    -2
}

pub fn sys_rename(old_ptr: *const u8, new_ptr: *const u8) -> isize {
    let old_path = match get_user_string(old_ptr) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let new_path = match get_user_string(new_ptr) {
        Ok(s) => s,
        Err(e) => return e,
    };
    
    let (old_parent_path, old_name) = match old_path.rsplit_once('/') {
        Some((p, n)) => (if p.is_empty() { "/" } else { p }, n),
        None => (".", old_path),
    };
    let (new_parent_path, new_name) = match new_path.rsplit_once('/') {
        Some((p, n)) => (if p.is_empty() { "/" } else { p }, n),
        None => (".", new_path),
    };
    
    if let Ok(old_parent) = crate::fs::vfs::resolve_path(old_parent_path) {
        if let Ok(new_parent) = crate::fs::vfs::resolve_path(new_parent_path) {
            if old_parent.rename(old_name, new_parent, new_name).is_ok() {
                return 0;
            }
        }
    }
    -2
}

pub fn sys_link(old_ptr: *const u8, new_ptr: *const u8) -> isize {
    let old_path = match get_user_string(old_ptr) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let new_path = match get_user_string(new_ptr) {
        Ok(s) => s,
        Err(e) => return e,
    };
    
    let (new_parent_path, new_name) = match new_path.rsplit_once('/') {
        Some((p, n)) => (if p.is_empty() { "/" } else { p }, n),
        None => (".", new_path),
    };
    
    if let Ok(inode) = crate::fs::vfs::resolve_path(old_path) {
        if let Ok(new_parent) = crate::fs::vfs::resolve_path(new_parent_path) {
            if new_parent.link(new_name, inode).is_ok() {
                return 0;
            }
        }
    }
    -2
}
