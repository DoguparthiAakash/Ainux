#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    // Hardcoded for now.
    let src = "test_file_copy.txt\0";
    let dest = "test_file_renamed.txt\0";
    
    let res = syscalls::sys_rename(src, dest);
    if res < 0 {
        println!("mv: Cannot rename '{}' to '{}': Error {}", "test_file_copy.txt", "test_file_renamed.txt", res);
        return 1;
    }
    
    0
}
