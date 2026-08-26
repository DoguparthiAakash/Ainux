#![no_std]
#![no_main]

use libainux::syscalls::*;
use libainux::println;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    println!("Starting Ainux httpd on port 8080...");
    
    let sock = sys_socket(2, 1, 0); // AF_INET, SOCK_STREAM
    if sock < 0 {
        println!("httpd: sys_socket failed: {}", sock);
        return 1;
    }
    
    // Bind to 0.0.0.0:8080
    let ip = [0, 0, 0, 0];
    let bind_res = sys_bind(sock as usize, &ip, 8080);
    if bind_res < 0 {
        println!("httpd: sys_bind failed: {}", bind_res);
        return 1;
    }
    
    let listen_res = sys_listen(sock as usize, 10);
    if listen_res < 0 {
        println!("httpd: sys_listen failed: {}", listen_res);
        return 1;
    }
    
    println!("httpd: listening for incoming connections...");
    
    loop {
        let client_fd = sys_accept(sock as usize);
        if client_fd < 0 {
            // Probably EAGAIN or something, yield and continue
            sys_yield();
            continue;
        }
        
        println!("httpd: accepted connection on fd {}", client_fd);
        
        // Read HTTP request
        let mut buf = [0u8; 1024];
        let mut read_bytes = 0;
        
        // simple blocking read with timeout/yield loop
        for _ in 0..100 {
            let n = sys_read(client_fd as usize, &mut buf[read_bytes..]);
            if n > 0 {
                read_bytes += n as usize;
                // look for \r\n\r\n
                let s = core::str::from_utf8(&buf[..read_bytes]).unwrap_or("");
                if s.contains("\r\n\r\n") {
                    break;
                }
            }
            sys_yield();
        }
        
        if read_bytes == 0 {
            sys_close(client_fd as usize);
            continue;
        }
        
        let request = core::str::from_utf8(&buf[..read_bytes]).unwrap_or("");
        let mut lines = request.lines();
        if let Some(first_line) = lines.next() {
            let mut parts = first_line.split_whitespace();
            let method = parts.next().unwrap_or("");
            let path = parts.next().unwrap_or("");
            
            if method == "GET" {
                // If path is "/", maybe default to "/README.txt" or index.html
                let actual_path = if path == "/" {
                    "/README.txt"
                } else {
                    path
                };
                
                println!("httpd: GET {}", actual_path);
                
                let file_fd = sys_open(actual_path, 0);
                if file_fd >= 0 {
                    let header = "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n";
                    sys_write(client_fd as usize, header.as_bytes());
                    
                    let mut file_buf = [0u8; 512];
                    loop {
                        let n = sys_read(file_fd as usize, &mut file_buf);
                        if n <= 0 {
                            break;
                        }
                        sys_write(client_fd as usize, &file_buf[..n as usize]);
                    }
                    sys_close(file_fd as usize);
                } else {
                    let header = "HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\nFile Not Found";
                    sys_write(client_fd as usize, header.as_bytes());
                }
            } else {
                let header = "HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\nBad Request";
                sys_write(client_fd as usize, header.as_bytes());
            }
        }
        
        // Wait a bit to ensure it gets sent (hack for simple smoltcp implementation without proper flush)
        for _ in 0..10 { sys_yield(); }
        sys_close(client_fd as usize);
        println!("httpd: closed connection");
    }
}
