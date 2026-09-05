// src/apps/telnetd.rs
// Remote Shell Daemon for Ainux

use alloc::string::String;
use alloc::format;
use crate::fs::vfs::FileHandle;
use crate::shell::{CURRENT_IN, CURRENT_OUT, execute_command};

pub extern "C" fn main() {
    crate::drivers::video::put_str("telnetd: Starting telnet server on port 23...\n");
    let fd = crate::net::sys_socket(2, 1, 0); // 2=AF_INET, 1=SOCK_STREAM
    if fd < 0 { 
        crate::drivers::video::put_str("telnetd: Error creating socket.\n");
        return; 
    }
    
    // Bind to port 23 (sys_bind takes port as addr_len for now)
    crate::net::sys_bind(fd as usize, core::ptr::null(), 23);
    crate::net::sys_listen(fd as usize, 10);
    crate::drivers::video::put_str("telnetd: Listening on port 23...\n");
    
    loop {
        let client_fd = crate::net::sys_accept(fd as usize);
        if client_fd >= 0 {
            crate::drivers::video::put_str("telnetd: Incoming connection accepted!\n");
            let pid = crate::process::scheduler::get_current_pid();
            let handle_opt = crate::cpu::without_interrupts(|| {
                let tasks = crate::process::scheduler::TASKS.lock();
                if let Some(task) = &tasks[pid] {
                    task.fds.get_handle(client_fd as usize).ok()
                } else { None }
            });
            
            if let Some(handle) = handle_opt {
                let welcome = "Ainux Telnet Server\r\n> ";
                let _ = handle.write(welcome.as_bytes(), 0);
                
                let mut buffer = String::new();
                loop {
                    let mut buf = [0u8; 1];
                    let read_res = handle.read(&mut buf, 0);
                    
                    match read_res {
                        Ok(0) => {
                            // Connection closed
                            crate::drivers::video::put_str("telnetd: Client disconnected.\n");
                            break;
                        }
                        Ok(1) => {
                            let c = buf[0] as char;
                            if c == '\n' || c == '\r' {
                                let _ = handle.write(b"\r\n", 0);
                                
                                // Redirect output to this socket
                                *CURRENT_IN.lock() = Some(handle.clone());
                                *CURRENT_OUT.lock() = Some(handle.clone());
                                
                                let trimmed = buffer.trim();
                                if trimmed == "exit" {
                                    *CURRENT_IN.lock() = None;
                                    *CURRENT_OUT.lock() = None;
                                    break;
                                }
                                
                                if !trimmed.is_empty() {
                                    execute_command(trimmed);
                                }
                                
                                // Restore original output
                                *CURRENT_IN.lock() = None;
                                *CURRENT_OUT.lock() = None;
                                
                                buffer.clear();
                                let _ = handle.write(b"> ", 0);
                            } else if c == '\x08' || c == '\x7f' { // Backspace
                                if !buffer.is_empty() {
                                    buffer.pop();
                                    let _ = handle.write(b"\x08 \x08", 0);
                                }
                            } else {
                                buffer.push(c);
                                let _ = handle.write(&[buf[0]], 0);
                            }
                        }
                        _ => {
                            // Yield CPU on EAGAIN or error
                            crate::process::scheduler::yield_now();
                        }
                    }
                }
            }
            // Close socket.
            crate::cpu::without_interrupts(|| {
                let mut tasks = crate::process::scheduler::TASKS.lock();
                if let Some(task) = &mut tasks[pid] {
                    task.fds.free_fd(client_fd as usize);
                }
            });
        }
        crate::process::scheduler::yield_now();
    }
}
