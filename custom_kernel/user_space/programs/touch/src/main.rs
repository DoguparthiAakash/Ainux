#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    // For simplicity until argument parsing is added, 
    // we'll assume a hardcoded or passed argument structure later.
    let path = "test_file.txt\0";
    
    // O_CREAT is 64
    let fd = syscalls::sys_open(path, 64);
    if fd < 0 {
        println!("touch: Cannot touch '{}': Error {}", "test_file.txt", fd);
        return 1;
    }
    
    syscalls::sys_close(fd as usize);
    0
}
