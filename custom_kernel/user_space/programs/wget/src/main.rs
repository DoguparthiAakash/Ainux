#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    println!("Connecting to 1.1.1.1:80...");
    
    // AF_INET = 2, SOCK_STREAM = 1, IPPROTO_TCP = 0
    let fd = syscalls::sys_socket(2, 1, 0); 
    if fd < 0 {
        println!("wget: Failed to open socket");
        return 1;
    }
    
    let ip = [1, 1, 1, 1];
    
    // sys_connect returns 0 if initiated
    let res = syscalls::sys_connect(fd as usize, &ip, 80);
    if res < 0 {
        println!("wget: Connect failed");
        syscalls::sys_close(fd as usize);
        return 1;
    }
    
    // HTTP GET Request
    let request = b"GET / HTTP/1.1\r\nHost: 1.1.1.1\r\nConnection: close\r\n\r\n";
    
    // Wait for connection to establish and send request
    let mut sent = -1;
    let mut timeout = 0;
    while sent < 0 {
        sent = syscalls::sys_write(fd as usize, request);
        if sent < 0 {
            syscalls::sys_yield();
            timeout += 1;
            if timeout > 100_000_000 {
                println!("wget: Connection timeout");
                syscalls::sys_close(fd as usize);
                return 1;
            }
        }
    }
    
    println!("Request sent ({} bytes). Waiting for response...", sent);
    
    // Wait for response
    let mut buf = [0u8; 1024];
    let mut received = -1;
    timeout = 0;
    while received <= 0 {
        received = syscalls::sys_read(fd as usize, &mut buf);
        if received <= 0 {
            syscalls::sys_yield();
            timeout += 1;
            if timeout > 100_000_000 {
                println!("wget: Read timeout");
                syscalls::sys_close(fd as usize);
                return 1;
            }
        }
    }
    
    println!("Response ({} bytes):", received);
    let slice = &buf[..received as usize];
    if let Ok(s) = core::str::from_utf8(slice) {
        println!("{}", s);
    } else {
        println!("<binary data>");
    }
    
    syscalls::sys_close(fd as usize);
    0
}
