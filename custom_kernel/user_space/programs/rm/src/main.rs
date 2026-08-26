#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    // For simplicity until argument parsing is added, 
    // we'll assume a hardcoded or passed argument structure later.
    let path = "test_file.txt\0";
    
    let res = syscalls::sys_unlink(path);
    if res < 0 {
        println!("rm: Cannot remove '{}': Error {}", "test_file.txt", res);
        return 1;
    }
    
    0
}
