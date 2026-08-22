#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    // For simplicity, we just read "/"
    // We should parse argv, but we don't have that yet in `main()`.
    // Wait, sys_execve doesn't pass argv to _start right now in the kernel.
    // The kernel's `exec_elf` zeroes out everything and sets up a clean stack.
    let path = "/\0";
    
    let mut buf = [0u8; 4096];
    let res = syscalls::sys_readdir(path, &mut buf);
    
    if res > 0 {
        let read_len = res as usize;
        let mut start = 0;
        for i in 0..read_len {
            if buf[i] == b'\n' {
                if let Ok(s) = core::str::from_utf8(&buf[start..i]) {
                    println!("{}", s);
                }
                start = i + 1;
            }
        }
    } else {
        println!("ls: Error reading directory.");
    }
    
    0
}
