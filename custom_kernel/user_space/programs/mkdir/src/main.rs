#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    // For simplicity until argument parsing is added, 
    // we'll assume a hardcoded or passed argument structure later.
    // In a real OS we'd read argc and argv.
    // For now, let's create a dummy directory just to test the syscall.
    let path = "test_dir\0";
    
    let res = syscalls::sys_mkdir(path);
    if res < 0 {
        println!("mkdir: Cannot create directory '{}': Error {}", "test_dir", res);
        return 1;
    }
    
    0
}
