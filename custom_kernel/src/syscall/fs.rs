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
    // For now, since VFS is partially stubbed, let's just return a dummy FD or check specific paths.
    
    // Fallback:
    let mut next_fd = NEXT_FD.lock();
    let fd = *next_fd;
    *next_fd += 1;
    
    // Note: We should actually insert a real FileHandle into FD_TABLE here.
    // For now, we return a success code.
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
