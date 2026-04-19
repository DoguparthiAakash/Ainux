use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use crate::drivers::video;
use crate::fs::vfs::{ROOT, FileType};

pub fn cmd_nuxv(args: &[&str]) {
    if args.len() < 4 || args[2] != "-o" {
        video::put_str("Usage: nuxv <source.v> -o <target.vbin>\n");
        return;
    }
    
    let source_file = args[1];
    let target_file = args[3];
    
    // Load Source
    let mut source_code = String::new();
    let root = ROOT.lock();
    if let Some(r) = root.as_ref() {
        if let Ok(inode) = r.lookup(source_file) {
            if let Ok(handle) = inode.open(0) {
                 let mut data = alloc::vec![0u8; 16384];
                 if let Ok(bytes) = handle.read(&mut data, 0) {
                     source_code = String::from(core::str::from_utf8(&data[..bytes]).unwrap_or(""));
                 }
            }
        } else {
            video::put_str("nuxv: Source not found.\n");
            return;
        }
    }
    core::mem::drop(root);

    video::put_str(&format!("Assembling {} (Voyager-Quantum)...\n", source_file));

    let mut bytecode: Vec<u8> = Vec::new();
    
    // Magic for Hybrid Voyager/Quantum
    let is_quantum = source_code.contains("Q_");
    if is_quantum {
        bytecode.extend_from_slice(b"QNUX"); // Quantum ANUX
    } else {
        bytecode.extend_from_slice(b"ANUX"); // Classical ANUX
    }
    bytecode.extend_from_slice(&1u16.to_le_bytes()); // Version
    bytecode.extend_from_slice(&[0u8; 58]); // Padding to 64 bytes
    
    for line in source_code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(";") { continue; }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let op = parts[0].to_uppercase();
        
        match op.as_str() {
            "PUSH" => {
                bytecode.push(0x01);
                let val = parts[1].parse::<i64>().unwrap_or(0);
                bytecode.extend_from_slice(&val.to_le_bytes());
            },
            "POP" => bytecode.push(0x02),
            "ADD" => bytecode.push(0x10),
            "SUB" => bytecode.push(0x11),
            "PRINT" => bytecode.push(0x20),
            "RECT" => bytecode.push(0x21),
            "JMP" => {
                bytecode.push(0x30);
                let offset = parts[1].parse::<i32>().unwrap_or(0);
                bytecode.extend_from_slice(&offset.to_le_bytes());
            },
            "Q_SET" => bytecode.push(0x40),
            "Q_HAD" => {
                bytecode.push(0x41);
                let qid = parts[1].parse::<u8>().unwrap_or(0);
                bytecode.push(qid);
            },
            "Q_ENT" => {
                bytecode.push(0x42);
                let q1 = parts[1].parse::<u8>().unwrap_or(0);
                let q2 = parts[2].parse::<u8>().unwrap_or(1);
                bytecode.push(q1);
                bytecode.push(q2);
            },
            "Q_MEAS" => {
                bytecode.push(0x43);
                let qid = parts[1].parse::<u8>().unwrap_or(0);
                bytecode.push(qid);
            },
            "EXIT" => bytecode.push(0xFF),
            _ => video::put_str(&format!("nuxv: Unknown op: {}\n", op)),
        }
    }

    // Save
    let root = ROOT.lock();
    if let Some(r) = root.as_ref() {
        let _ = r.create(target_file, FileType::File);
        if let Ok(inode) = r.lookup(target_file) {
            if let Ok(handle) = inode.open(0) {
                 let _ = handle.truncate();
                 let _ = handle.write(&bytecode, 0);
                 video::put_str(&format!("Success! Produced Voyager bin: {} bytes\n", bytecode.len()));
            }
        }
    }
}
