#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    // For simplicity, we just read "hello.txt"
    // In a real implementation we would parse arguments
    let path = "hello.txt\0";
    
    let fd = syscalls::sys_open(path, 0); // 0 = O_RDONLY
    if fd < 0 {
        println!("cat: Error opening file.");
        return 1;
    }
    
    let mut buf = [0u8; 4096];
    loop {
        let n = syscalls::sys_read(fd as usize, &mut buf);
        if n <= 0 { break; }
        
        let read_len = n as usize;
        if let Ok(s) = core::str::from_utf8(&buf[0..read_len]) {
            print!("{}", s);
        } else {
            println!("<Binary Content>");
            break;
        }
    }
    println!();
    
    syscalls::sys_close(fd as usize);
    0
}
