#![no_std]
#![no_main]

use libainux::{print, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    let mut buf = [0u8; 4096];
    let n = syscalls::sys_get_tasks(&mut buf);
    if n > 0 {
        if let Ok(s) = core::str::from_utf8(&buf[..n as usize]) {
            print!("{}", s);
        }
    } else {
        print!("ps: Error fetching tasks\n");
    }
    
    0
}
