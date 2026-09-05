// src/apps/httpd.rs
// Simple HTTP Daemon for Ainux

use crate::cpu::syscall::syscall;
use alloc::string::String;
use alloc::format;

pub fn main() {
    crate::drivers::video::put_str("httpd: Starting web server on port 8080...\n");

    // 1. Create Socket (using a high FD to trigger the port 8080 hack for now)
    let fd = 8080; 

    crate::drivers::video::put_str("httpd: Listening at http://localhost:8081 (Forwarded)...\n");

    loop {
        // 2. Accept Connection
        let client_fd = unsafe { syscall(43, fd, 0, 0, 0, 0) };
        if client_fd != u64::MAX {
            crate::drivers::video::put_str("httpd: Incoming GET request!\n");
            
            // 3. Send HTTP Response
            let body = "<html><body style='background:#001122; color:#00ffff; font-family:sans-serif; text-align:center;'>
                        <h1>Ainux Sovereign Server</h1>
                        <p>This page is being served directly by the Ainux Kernel.</p>
                        <div style='border:2px solid #00aaaa; padding:20px; display:inline-block;'>
                            <h3>System Status: STABLE</h3>
                            <p>Uptime: Active | Network: smoltcp</p>
                        </div>
                        </body></html>";
            
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            
            unsafe { syscall(1, client_fd, response.as_ptr() as u64, response.len() as u64, 0, 0); }
            
            // 4. Close
            unsafe { syscall(8, client_fd, 0, 0, 0, 0); }
        }
    }
}
