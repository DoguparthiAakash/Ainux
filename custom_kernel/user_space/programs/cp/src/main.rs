#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    // Hardcoded for now.
    let src = "test_file.txt\0";
    let dest = "test_file_copy.txt\0";
    
    let fd_src = syscalls::sys_open(src, 0); // O_RDONLY
    if fd_src < 0 {
        println!("cp: Cannot open source '{}'", "test_file.txt");
        return 1;
    }
    
    let fd_dest = syscalls::sys_open(dest, 65); // O_CREAT | O_WRONLY
    if fd_dest < 0 {
        println!("cp: Cannot open/create dest '{}'", "test_file_copy.txt");
        syscalls::sys_close(fd_src as usize);
        return 1;
    }
    
    let mut buf = [0u8; 4096];
    loop {
        let n = syscalls::sys_read(fd_src as usize, &mut buf);
        if n <= 0 { break; }
        let written = syscalls::sys_write(fd_dest as usize, &buf[0..(n as usize)]);
        if written < 0 {
            println!("cp: Error writing to dest.");
            break;
        }
    }
    
    syscalls::sys_close(fd_src as usize);
    syscalls::sys_close(fd_dest as usize);
    0
}
