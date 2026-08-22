use core::arch::asm;

pub unsafe fn syscall(id: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            in("rax") id,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            in("r8")  a5,
            in("r9")  a6,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    unsafe { syscall(1, fd as u64, buf.as_ptr() as u64, buf.len() as u64, 0, 0, 0) as isize }
}

pub fn sys_read(fd: usize, buf: &mut [u8]) -> isize {
    unsafe { syscall(0, fd as u64, buf.as_mut_ptr() as u64, buf.len() as u64, 0, 0, 0) as isize }
}

pub fn sys_open(path: &str, flags: u32) -> isize {
    unsafe { syscall(2, path.as_ptr() as u64, flags as u64, 0, 0, 0, 0) as isize }
}

pub fn sys_close(fd: usize) -> isize {
    unsafe { syscall(3, fd as u64, 0, 0, 0, 0, 0) as isize }
}

pub fn sys_kill(pid: usize, sig: u32) -> isize {
    unsafe { syscall(62, pid as u64, sig as u64, 0, 0, 0, 0) as isize }
}

pub fn sys_get_tasks(buf: &mut [u8]) -> isize {
    unsafe { syscall(500, buf.as_mut_ptr() as u64, buf.len() as u64, 0, 0, 0, 0) as isize }
}

pub fn sys_exit(code: isize) -> ! {
    unsafe {
        syscall(60, code as u64, 0, 0, 0, 0, 0);
        core::hint::unreachable_unchecked()
    }
}

pub fn sys_yield() {
    unsafe { syscall(24, 0, 0, 0, 0, 0, 0); }
}

pub fn sys_socket(domain: i32, type_: i32, protocol: i32) -> isize {
    unsafe { syscall(41, domain as u64, type_ as u64, protocol as u64, 0, 0, 0) as isize }
}

pub fn sys_connect(fd: usize, ip: &[u8], port: u16) -> isize {
    unsafe { syscall(42, fd as u64, ip.as_ptr() as u64, port as u64, 0, 0, 0) as isize }
}

pub fn sys_execve(path: &str) -> isize {
    unsafe { syscall(59, path.as_ptr() as u64, 0, 0, 0, 0, 0) as isize }
}

pub fn sys_fork() -> isize {
    unsafe { syscall(57, 0, 0, 0, 0, 0, 0) as isize }
}

pub fn sys_wait4(pid: isize, status: *mut i32, options: i32, rusage: u64) -> isize {
    unsafe { syscall(61, pid as u64, status as u64, options as u64, rusage, 0, 0) as isize }
}

pub fn sys_mmap(addr: u64, length: u64, prot: u64, flags: u64, fd: usize, offset: u64) -> u64 {
    unsafe { syscall(9, addr, length, prot, flags, fd as u64, offset) }
}

pub fn sys_readdir(path: &str, buf: &mut [u8]) -> isize {
    unsafe { syscall(78, path.as_ptr() as u64, buf.as_mut_ptr() as u64, buf.len() as u64, 0, 0, 0) as isize }
}
