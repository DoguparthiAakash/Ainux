#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    println!("Netcat: Connecting to 10.0.2.2:8080...");
    let fd = syscalls::sys_socket(2, 1, 6); // AF_INET, SOCK_STREAM, IPPROTO_TCP
    if fd < 0 {
        println!("nc: Failed to open socket.");
        return 1;
    }
    
    // Connect to 10.0.2.2 (host loopback in QEMU user networking) on port 8080
    let ip = [10, 0, 2, 2];
    let res = syscalls::sys_connect(fd as usize, &ip, 8080);
    if res < 0 {
        println!("nc: Connection failed.");
        syscalls::sys_close(fd as usize);
        return 1;
    }
    
    println!("Connected. Type your message:");
    let msg = "Hello from Mithl OS Netcat!\n";
    syscalls::sys_write(fd as usize, msg.as_bytes());
    
    // Attempt to read response
    let mut buf = [0u8; 1024];
    for _ in 0..100 {
        let n = syscalls::sys_read(fd as usize, &mut buf);
        if n > 0 {
            if let Ok(s) = core::str::from_utf8(&buf[0..(n as usize)]) {
                print!("{}", s);
            }
            break;
        }
        for _ in 0..1_000_000 {
            syscalls::sys_yield();
        }
    }
    
    println!("nc: Closing connection.");
    syscalls::sys_close(fd as usize);
    0
}
