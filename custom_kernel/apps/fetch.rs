// src/apps/fetch.rs
// Native HTTP Client for Ainux

use alloc::string::String;
use alloc::vec::Vec;
use crate::cpu::syscall::syscall;

pub fn main(args: &[&str]) {
    if args.len() < 2 {
        crate::drivers::video::put_str("Usage: fetch <ip_address>\n");
        crate::drivers::video::put_str("Example: fetch 93.184.216.34 (example.com)\n");
        return;
    }

    let ip_str = args[1];
    let mut ip_bytes = [0u8; 4];
    
    // Check if it's an IP or a Hostname
    if ip_str.contains('.') && ip_str.chars().any(|c| c.is_alphabetic()) {
        crate::drivers::video::put_str(&alloc::format!("fetch: Resolving {}...\n", ip_str));
        if unsafe { syscall(45, ip_str.as_ptr() as u64, ip_bytes.as_mut_ptr() as u64, ip_str.len() as u64, 0, 0) } != 0 {
            crate::drivers::video::put_str("fetch: Failed to resolve hostname.\n");
            return;
        }
    } else {
        // Simple IP parser
        let parts: Vec<&str> = ip_str.split('.').collect();
        if parts.len() != 4 {
            crate::drivers::video::put_str("fetch: Invalid IP address.\n");
            return;
        }
        for i in 0..4 {
            ip_bytes[i] = parts[i].parse().unwrap_or(0);
        }
    }

    crate::drivers::video::put_str(&alloc::format!("fetch: Connecting to {} on port 80...\n", ip_str));

    // 1. Create Socket
    let fd = unsafe { syscall(40, 1, 0, 0, 0, 0) }; // sys_socket(1=TCP)
    if fd == u64::MAX {
        crate::drivers::video::put_str("fetch: Failed to create socket.\n");
        return;
    }

    // 2. Connect
    if unsafe { syscall(44, fd, ip_bytes.as_ptr() as u64, 80, 0, 0) } != 0 {
        crate::drivers::video::put_str("fetch: Connection failed.\n");
        return;
    }

    crate::drivers::video::put_str("fetch: Connected. Sending HTTP Request...\n");

    // 3. Send Request
    let request = alloc::format!("GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", ip_str);
    unsafe { syscall(1, fd, request.as_ptr() as u64, request.len() as u64, 0, 0); }

    // 4. Read Response
    crate::drivers::video::put_str("fetch: Waiting for data...\n");
    let mut response_received = false;
    
    // We'll poll for 5 seconds
    let start = unsafe { syscall(10, 0, 0, 0, 0, 0) }; 
    while unsafe { syscall(10, 0, 0, 0, 0, 0) } < start + 5000 {
        let mut buf = [0u8; 1024];
        let n = unsafe { syscall(7, fd, buf.as_mut_ptr() as u64, 1024, 0, 0) };
        if n > 0 && n != u64::MAX {
            if let Ok(s) = core::str::from_utf8(&buf[..n as usize]) {
                crate::drivers::video::put_str(s);
                response_received = true;
            }
        } else if n == 0 && response_received {
            break; // Finished
        }
        
        // Give CPU time to other tasks and network stack
        unsafe { syscall(24, 0, 0, 0, 0, 0) }; 
    }

    if !response_received {
        crate::drivers::video::put_str("\nfetch: No response received (Timeout).\n");
    }

    // 5. Close
    unsafe { syscall(8, fd, 0, 0, 0, 0) };
    crate::drivers::video::put_str("\nfetch: Done.\n");
}
