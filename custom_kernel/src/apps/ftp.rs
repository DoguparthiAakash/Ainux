// src/apps/ftp.rs
// Simple FTP client for Ainux

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::fs::vfs::{FileHandle, FileType};

pub fn cmd_ftp(args: &[&str]) {
    if args.len() < 4 {
        crate::drivers::video::put_str("Usage: ftp <IP> <GET|PUT> <filename>\n");
        return;
    }
    
    let ip = args[1];
    let cmd = args[2].to_uppercase();
    let filename = args[3];
    
    // Parse IP
    let mut ip_bytes = [0u8; 4];
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        crate::drivers::video::put_str("ftp: Invalid IP address format.\n");
        return;
    }
    for i in 0..4 {
        if let Ok(b) = parts[i].parse::<u8>() {
            ip_bytes[i] = b;
        } else {
            crate::drivers::video::put_str("ftp: Invalid IP address.\n");
            return;
        }
    }
    let ip_addr = u32::from_be_bytes(ip_bytes);
    
    let fd = crate::net::sys_socket(2, 1, 0); // 2=AF_INET, 1=SOCK_STREAM
    if fd < 0 {
        crate::drivers::video::put_str("ftp: Failed to create socket.\n");
        return;
    }
    
    crate::drivers::video::put_str("ftp: Connecting...\n");
    let res = crate::net::sys_connect(fd as usize, ip_bytes.as_ptr(), 21);
    if res < 0 {
        crate::drivers::video::put_str("ftp: Connection failed.\n");
        // close socket (TODO)
        return;
    }
    
    let pid = crate::process::scheduler::get_current_pid();
    let handle_opt = crate::cpu::without_interrupts(|| {
        let tasks = crate::process::scheduler::TASKS.lock();
        if let Some(task) = &tasks[pid] {
            task.fds.get_handle(fd as usize).ok()
        } else { None }
    });
    
    let handle = if let Some(h) = handle_opt { h } else {
        crate::drivers::video::put_str("ftp: Invalid handle.\n");
        return;
    };
    
    // Wait for welcome message
    let mut buf = [0u8; 128];
    // Simple non-blocking wait
    for _ in 0..100 {
        if handle.read(&mut buf, 0).unwrap_or(0) > 0 {
            break;
        }
        crate::process::scheduler::yield_now();
    }
    
    if cmd == "GET" {
        let req = alloc::format!("GET {}\n", filename);
        let _ = handle.write(req.as_bytes(), 0);
        
        // Read response
        let mut buffer = String::new();
        let mut file_size = 0usize;
        let mut is_receiving_data = false;
        let mut file_data = Vec::new();
        
        loop {
            let mut read_buf = [0u8; 256];
            let n = handle.read(&mut read_buf, 0).unwrap_or(0);
            if n == 0 {
                break;
            }
            
            if is_receiving_data {
                file_data.extend_from_slice(&read_buf[..n]);
                if file_data.len() >= file_size {
                    break;
                }
            } else {
                for i in 0..n {
                    let c = read_buf[i] as char;
                    if c == '\n' {
                        let trimmed = buffer.trim();
                        if trimmed.starts_with("OK ") {
                            let size_str = trimmed.trim_start_matches("OK ");
                            if let Ok(size) = size_str.parse::<usize>() {
                                file_size = size;
                                is_receiving_data = true;
                                file_data.extend_from_slice(&read_buf[i+1..n]);
                                break;
                            }
                        } else {
                            crate::drivers::video::put_str("ftp: Server error: ");
                            crate::drivers::video::put_str(trimmed);
                            crate::drivers::video::put_str("\n");
                            return;
                        }
                    } else {
                        buffer.push(c);
                    }
                }
            }
        }
        
        if is_receiving_data {
            let parent_inode = crate::shell::find_inode("/").unwrap_or_else(|_| crate::fs::vfs::root());
            if let Ok(inode) = parent_inode.create(filename, FileType::File) {
                if let Ok(file_handle) = inode.open(1) { // 1=write
                    let _ = file_handle.write(&file_data, 0);
                    crate::drivers::video::put_str("ftp: Download complete.\n");
                }
            } else {
                crate::drivers::video::put_str("ftp: Failed to save file.\n");
            }
        }
        
    } else if cmd == "PUT" {
        if let Ok(inode) = crate::shell::find_inode(filename) {
            let size = inode.stat().map(|s| s.size).unwrap_or(0);
            let mut data = alloc::vec![0u8; size as usize];
            if let Ok(file_handle) = inode.open(0) { // 0=read
                if file_handle.read(&mut data, 0).is_ok() {
                    let req = alloc::format!("PUT {} {}\n", filename, size);
                    let _ = handle.write(req.as_bytes(), 0);
                    let _ = handle.write(&data, 0);
                    
                    // Wait for OK
                    loop {
                        let mut read_buf = [0u8; 128];
                        let n = handle.read(&mut read_buf, 0).unwrap_or(0);
                        if n > 0 {
                            let s = core::str::from_utf8(&read_buf[..n]).unwrap_or("");
                            if s.contains("OK") {
                                crate::drivers::video::put_str("ftp: Upload complete.\n");
                            } else {
                                crate::drivers::video::put_str("ftp: Server replied: ");
                                crate::drivers::video::put_str(s);
                            }
                            break;
                        }
                    }
                } else {
                    crate::drivers::video::put_str("ftp: Failed to read file.\n");
                }
            } else {
                crate::drivers::video::put_str("ftp: Failed to open file.\n");
            }
        } else {
            crate::drivers::video::put_str("ftp: File not found locally.\n");
        }
    } else {
        crate::drivers::video::put_str("ftp: Unknown command. Use GET or PUT.\n");
    }
    
    // Close socket (TODO)
    crate::cpu::without_interrupts(|| {
        let mut tasks = crate::process::scheduler::TASKS.lock();
        if let Some(task) = &mut tasks[pid] {
            task.fds.free_fd(fd as usize);
        }
    });
}
