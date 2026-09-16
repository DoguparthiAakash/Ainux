#![no_std]
#![no_main]

use libainux::{println, syscalls};

/// Read one line from `fd` into `buf`. Returns bytes read, 0 on EOF.
fn read_line(fd: usize, buf: &mut [u8]) -> usize {
    let mut total = 0usize;
    loop {
        if total >= buf.len() - 1 { break; }
        let mut b = [0u8; 1];
        let n = syscalls::sys_read(fd, &mut b);
        if n <= 0 { break; }
        buf[total] = b[0];
        total += 1;
        if b[0] == b'\n' { break; }
    }
    total
}

/// Simple substring search — no alloc, no regex.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() { return true; }
    if haystack.len() < needle.len() { return false; }
    let n = needle.len();
    for i in 0..=(haystack.len() - n) {
        if &haystack[i..i + n] == needle { return true; }
    }
    false
}

fn process_fd(fd: usize, pattern: &[u8]) {
    let mut buf = [0u8; 512];
    loop {
        let n = read_line(fd, &mut buf);
        if n == 0 { break; }
        if contains(&buf[..n], pattern) {
            syscalls::sys_write(1, &buf[..n]);
        }
    }
}

#[no_mangle]
pub extern "C" fn main() -> isize {
    // Collect args into a fixed-size array on the stack to avoid alloc.
    let mut arg_ptrs: [Option<&str>; 16] = [None; 16];
    let mut argc = 0usize;
    for a in libainux::env::args() {
        if argc < 16 {
            arg_ptrs[argc] = Some(a);
            argc += 1;
        }
    }

    if argc < 2 {
        println!("Usage: grep <pattern> [file...]");
        return 1;
    }

    let pattern = arg_ptrs[1].unwrap_or("").as_bytes();

    if argc <= 2 {
        // stdin mode
        process_fd(0, pattern);
    } else {
        for i in 2..argc {
            if let Some(filename) = arg_ptrs[i] {
                let fd = syscalls::sys_open(filename, 0);
                if fd >= 0 {
                    process_fd(fd as usize, pattern);
                    syscalls::sys_close(fd as usize);
                } else {
                    println!("grep: {}: No such file or directory", filename);
                }
            }
        }
    }
    0
}
