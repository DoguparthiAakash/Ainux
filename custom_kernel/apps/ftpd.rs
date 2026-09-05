// src/apps/ftpd.rs
// Simple File Transfer Daemon for Ainux (Listens on Port 21)

use alloc::string::String;
use alloc::vec::Vec;
use crate::fs::vfs::FileHandle;

pub extern "C" fn main() {
    crate::drivers::video::put_str("ftpd: Starting file transfer server on port 21...\n");
    let fd = crate::net::sys_socket(2, 1, 0); // 2=AF_INET, 1=SOCK_STREAM
    if fd < 0 { 
        crate::drivers::video::put_str("ftpd: Error creating socket.\n");
        return; 
    }
    
    // Bind to port 21
    crate::net::sys_bind(fd as usize, core::ptr::null(), 21);
    crate::net::sys_listen(fd as usize, 10);
    crate::drivers::video::put_str("ftpd: Listening on port 21...\n");
    
    loop {
        let client_fd = crate::net::sys_accept(fd as usize);
        if client_fd >= 0 {
            crate::drivers::video::put_str("ftpd: Incoming connection accepted!\n");
            let pid = crate::process::scheduler::get_current_pid();
            let handle_opt = crate::cpu::without_interrupts(|| {
                let tasks = crate::process::scheduler::TASKS.lock();
                if let Some(task) = &tasks[pid] {
                    task.fds.get_handle(client_fd as usize).ok()
                } else { None }
            });
            
            if let Some(handle) = handle_opt {
                let welcome = "Ainux Simple FTP (GET <file> | PUT <file> <size>)\\n";
                let _ = handle.write(welcome.as_bytes(), 0);
                
                let mut buffer = String::new();
                let mut is_putting = false;
                let mut put_filename = String::new();
                let mut put_size = 0usize;
                let mut put_received = 0usize;
                let mut put_data = Vec::new();
                
                loop {
                    let mut buf = [0u8; 128];
                    let read_res = handle.read(&mut buf, 0);
                    
                    match read_res {
                        Ok(0) => {
                            // Connection closed
                            crate::drivers::video::put_str("ftpd: Client disconnected.\n");
                            break;
                        }
                        Ok(len) => {
                            if is_putting {
                                put_data.extend_from_slice(&buf[..len]);
                                put_received += len;
                                
                                if put_received >= put_size {
                                    // Save file
                                    let parent_inode = crate::shell::find_inode("/").unwrap_or_else(|_| crate::fs::vfs::root());
                                    if let Ok(file_inode) = parent_inode.create(&put_filename, crate::fs::vfs::FileType::File) {
                                        if let Ok(file_handle) = file_inode.open(1) { // 1=write
                                            let _ = file_handle.write(&put_data, 0);
                                            let _ = handle.write(b"OK\\n", 0);
                                        } else {
                                            let _ = handle.write(b"ERR failed to open file\\n", 0);
                                        }
                                    } else {
                                        let _ = handle.write(b"ERR failed to create file\\n", 0);
                                    }
                                    is_putting = false;
                                    buffer.clear();
                                }
                            } else {
                                for i in 0..len {
                                    let c = buf[i] as char;
                                    if c == '\n' || c == '\r' {
                                        let trimmed = buffer.trim();
                                        let mut parts = trimmed.split_whitespace();
                                        let cmd = parts.next().unwrap_or("");
                                        
                                        if cmd == "GET" {
                                            if let Some(filename) = parts.next() {
                                                if let Ok(inode) = crate::shell::find_inode(filename) {
                                                    let size = inode.stat().map(|s| s.size).unwrap_or(0);
                                                    let mut data = alloc::vec![0u8; size as usize];
                                                    if let Ok(file_handle) = inode.open(0) { // 0=read
                                                        if file_handle.read(&mut data, 0).is_ok() {
                                                            let reply = alloc::format!("OK {}\\n", size);
                                                            let _ = handle.write(reply.as_bytes(), 0);
                                                            let _ = handle.write(&data, 0);
                                                        } else {
                                                            let _ = handle.write(b"ERR read failed\\n", 0);
                                                        }
                                                    } else {
                                                        let _ = handle.write(b"ERR open failed\\n", 0);
                                                    }
                                                } else {
                                                    let _ = handle.write(b"ERR not found\\n", 0);
                                                }
                                            }
                                            buffer.clear();
                                        } else if cmd == "PUT" {
                                            if let (Some(filename), Some(size_str)) = (parts.next(), parts.next()) {
                                                if let Ok(size) = size_str.parse::<usize>() {
                                                    put_filename = filename.into();
                                                    put_size = size;
                                                    put_received = 0;
                                                    put_data.clear();
                                                    is_putting = true;
                                                } else {
                                                    let _ = handle.write(b"ERR invalid size\\n", 0);
                                                }
                                            }
                                            buffer.clear();
                                        } else if cmd == "EXIT" || cmd == "QUIT" {
                                            buffer.clear();
                                            break;
                                        } else if !cmd.is_empty() {
                                            let _ = handle.write(b"ERR unknown command\\n", 0);
                                            buffer.clear();
                                        }
                                    } else {
                                        buffer.push(c);
                                    }
                                }
                            }
                        }
                        _ => {
                            crate::process::scheduler::yield_now();
                        }
                    }
                }
            }
            
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
