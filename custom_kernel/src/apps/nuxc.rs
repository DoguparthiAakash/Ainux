use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::video;
use crate::fs::vfs::{FileType, FileHandle};
use crate::cpu::syscall::syscall;

pub fn cmd_nuxc(args: &[&str]) {
    if args.len() < 4 || args[2] != "-o" {
        video::put_str("Usage: nuxc <source.c> -o <target.alo>\n");
        return;
    }
    
    let source_file = args[1];
    let target_file = args[3];
    
    // Load Source Code
    let mut source_code = String::new();
    if let Ok(inode) = crate::shell::find_inode(source_file) {
        if let Ok(handle) = inode.open(0) {
             let mut data = alloc::vec![0u8; 16384];
             if let Ok(bytes) = handle.read(&mut data, 0) {
                 if bytes > 0 {
                     source_code = String::from(core::str::from_utf8(&data[..bytes]).unwrap_or(""));
                 }
             }
        }
    } else {
        video::put_str("nuxc: Source file not found.\n");
        return;
    }

    if source_code.is_empty() {
        video::put_str("nuxc: Source file is empty.\n");
        return;
    }

    video::put_str(&alloc::format!("Forwarding {} to Host LLVM Backend...\n", source_file));
    compile_via_host(&source_code, target_file);
}

pub fn compile_via_host(source_code: &str, target_file: &str) {
    let ip_bytes = [10u8, 0, 2, 2]; // QEMU Gateway IP
    
    // 1. Create Socket
    let fd = unsafe { syscall(40, 1, 0, 0, 0, 0) }; 
    if fd == u64::MAX {
        video::put_str("nuxc: Failed to create socket.\n");
        return;
    }

    // 2. Connect
    if unsafe { syscall(44, fd, ip_bytes.as_ptr() as u64, 8000, 0, 0) } != 0 {
        video::put_str("nuxc: Failed to connect to Host LLVM Backend (10.0.2.2:8000).\n");
        video::put_str("Make sure host_compiler.py is running on your host machine!\n");
        return;
    }

    video::put_str("Connected to LLVM Backend. Sending source code...\n");

    // 3. Send HTTP Request
    let request_body = source_code;
    let request_header = alloc::format!("POST /compile HTTP/1.1\r\nHost: 10.0.2.2:8000\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", request_body.len());
    
    unsafe { syscall(1, fd, request_header.as_ptr() as u64, request_header.len() as u64, 0, 0); }
    unsafe { syscall(1, fd, request_body.as_ptr() as u64, request_body.len() as u64, 0, 0); }

    // 4. Read Response (Binary ELF)
    video::put_str("Waiting for compiled binary...\n");
    let mut response: Vec<u8> = Vec::new();
    let mut response_received = false;
    
    let start = unsafe { syscall(10, 0, 0, 0, 0, 0) }; 
    while unsafe { syscall(10, 0, 0, 0, 0, 0) } < start + 10000 {
        let mut buf = [0u8; 4096];
        let n = unsafe { syscall(7, fd, buf.as_mut_ptr() as u64, 4096, 0, 0) };
        if n > 0 && n != u64::MAX {
            response.extend_from_slice(&buf[..n as usize]);
            response_received = true;
        } else if n == 0 && response_received {
            break; 
        }
        unsafe { syscall(24, 0, 0, 0, 0, 0) }; 
    }

    unsafe { syscall(8, fd, 0, 0, 0, 0) };

    if !response_received || response.is_empty() {
        video::put_str("nuxc: No response or timeout.\n");
        return;
    }

    let mut body_start = 0;
    for i in 0..response.len().saturating_sub(4) {
        if response[i] == b'\r' && response[i+1] == b'\n' && response[i+2] == b'\r' && response[i+3] == b'\n' {
            body_start = i + 4;
            break;
        }
    }

    let body = &response[body_start..];
    
    if let Ok(resp_str) = core::str::from_utf8(&response[..body_start]) {
        if resp_str.contains(" 500 ") {
            video::put_str("nuxc: Compilation Failed on Host:\n");
            if let Ok(err_msg) = core::str::from_utf8(body) {
                video::put_str(err_msg);
            }
            video::put_str("\n");
            return;
        }
    }

    if body.is_empty() {
        video::put_str("nuxc: Received empty binary.\n");
        return;
    }

    video::put_str(&alloc::format!("Compilation successful. Received ELF binary: {} bytes.\n", body.len()));

    if let Ok((parent, name)) = crate::shell::find_parent_and_name(target_file) {
        let _ = parent.create(&name, FileType::File);
        if let Ok(inode) = parent.lookup(&name) {
            if let Ok(handle) = inode.open(0) {
                 let _ = handle.truncate();
                 let _ = handle.write(body, 0);
                 video::put_str("Binary saved to disk.\n");
            }
        }
    } else {
        video::put_str("nuxc: Target directory not found.\n");
    }
}
