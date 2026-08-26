#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    let mut args = libainux::env::args();
    let _program_name = args.next(); // Skip argv[0]
    
    let target = args.next().unwrap_or("1.1.1.1:80");
    let path = args.next().unwrap_or("/");
    
    let mut ip_parts = [1u8, 1, 1, 1];
    let mut port: u16 = 80;
    
    // Parse target (e.g. 10.0.2.2:80)
    let mut colon_split = target.split(':');
    if let Some(ip_str) = colon_split.next() {
        let mut octets = ip_str.split('.');
        for i in 0..4 {
            if let Some(oct) = octets.next() {
                // simple primitive parse
                let mut val = 0u8;
                for b in oct.bytes() {
                    if b >= b'0' && b <= b'9' {
                        val = val.wrapping_mul(10).wrapping_add(b - b'0');
                    }
                }
                ip_parts[i] = val;
            }
        }
    }
    if let Some(port_str) = colon_split.next() {
        let mut val = 0u16;
        for b in port_str.bytes() {
            if b >= b'0' && b <= b'9' {
                val = val.wrapping_mul(10).wrapping_add((b - b'0') as u16);
            }
        }
        port = val;
    }
    
    println!("Connecting to {}.{}.{}.{}:{} for path {}...", ip_parts[0], ip_parts[1], ip_parts[2], ip_parts[3], port, path);
    
    // AF_INET = 2, SOCK_STREAM = 1, IPPROTO_TCP = 0
    let fd = syscalls::sys_socket(2, 1, 0); 
    if fd < 0 {
        println!("wget: Failed to open socket");
        return 1;
    }
    
    let res = syscalls::sys_connect(fd as usize, &ip_parts, port);
    if res < 0 {
        println!("wget: Connect failed");
        syscalls::sys_close(fd as usize);
        return 1;
    }
    
    // HTTP GET Request
    let request_start = "GET ";
    let request_mid = " HTTP/1.1\r\nHost: ";
    let request_end = "\r\nConnection: close\r\n\r\n";
    
    // Wait for connection to establish and send request
    let mut timeout = 0;
    loop {
        // We write in parts to avoid alloc
        let s1 = syscalls::sys_write(fd as usize, request_start.as_bytes());
        if s1 >= 0 {
            syscalls::sys_write(fd as usize, path.as_bytes());
            syscalls::sys_write(fd as usize, request_mid.as_bytes());
            syscalls::sys_write(fd as usize, target.as_bytes());
            syscalls::sys_write(fd as usize, request_end.as_bytes());
            break;
        }
        
        syscalls::sys_yield();
        timeout += 1;
        if timeout > 100_000_000 {
            println!("wget: Connection timeout");
            syscalls::sys_close(fd as usize);
            return 1;
        }
    }
    
    println!("Request sent. Waiting for response...");
    
    // Create output file (save to local disk)
    // Extract filename from path (last part after /)
    let mut filename = "index.html";
    if let Some(idx) = path.rfind('/') {
        if idx + 1 < path.len() {
            filename = &path[idx + 1..];
        }
    }
    
    // Attempt to open for writing. If we can't, we'll just print.
    // 0x40 is O_CREAT, 0x01 is O_WRONLY
    let out_fd = syscalls::sys_open(filename, 0x41);
    
    let mut buf = [0u8; 1024];
    let mut received = -1;
    let mut total_received = 0;
    timeout = 0;
    
    loop {
        received = syscalls::sys_read(fd as usize, &mut buf);
        if received > 0 {
            timeout = 0; // reset timeout
            total_received += received;
            if out_fd >= 0 {
                syscalls::sys_write(out_fd as usize, &buf[..received as usize]);
            } else {
                let slice = &buf[..received as usize];
                if let Ok(s) = core::str::from_utf8(slice) {
                    print!("{}", s);
                } else {
                    print!("<binary chunk>");
                }
            }
        } else if received <= 0 && total_received > 0 {
            // EOF
            break;
        } else {
            syscalls::sys_yield();
            timeout += 1;
            if timeout > 100_000_000 {
                println!("wget: Read timeout");
                break;
            }
        }
    }
    
    println!("\nResponse complete ({} bytes).", total_received);
    if out_fd >= 0 {
        println!("Saved to {}", filename);
        syscalls::sys_close(out_fd as usize);
    }
    
    syscalls::sys_close(fd as usize);
    0
}
