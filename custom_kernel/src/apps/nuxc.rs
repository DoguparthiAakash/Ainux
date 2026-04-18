use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::video;
use crate::fs::vfs::{ROOT, FileType, FileHandle};

#[repr(C, packed)]
struct AloHeader {
    magic: [u8; 4],
    entry: u64,
    segment_count: u32,
    header_size: u32,
}

#[repr(C, packed)]
struct AloSegment {
    typ: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    size: u64,
}

pub fn cmd_nuxc(args: &[&str]) {
    if args.len() < 4 || args[2] != "-o" {
        video::put_str("Usage: nuxc <source.c> -o <target.alo>\n");
        return;
    }
    
    let source_file = args[1];
    let target_file = args[3];
    
    // Load Source Code
    let mut source_code = String::new();
    let root = ROOT.lock();
    if let Some(r) = root.as_ref() {
        if let Ok(inode) = r.lookup(source_file) {
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
    }
    core::mem::drop(root);

    if source_code.is_empty() {
        video::put_str("nuxc: Source file is empty.\n");
        return;
    }

    video::put_str(&format!("Compiling {} to ALO v2 (Segmented)...\n", source_file));

    let mut machine_code: Vec<u8> = Vec::new();
    let mut data_section: Vec<u8> = Vec::new();
    let mut string_lengths: Vec<usize> = Vec::new();
    
    for line in source_code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#include") || trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if trimmed.contains("printf(") {
            if let Some(start_quote) = trimmed.find('"') {
                if let Some(end_quote) = trimmed.rfind('"') {
                    if end_quote > start_quote {
                        let text = &trimmed[start_quote + 1..end_quote];
                        let processed_text = text.replace("\\n", "\n");
                        
                        string_lengths.push(processed_text.len());
                        data_section.extend_from_slice(processed_text.as_bytes());
                        
                        // MOV RDI, 1
                        machine_code.extend_from_slice(&[0x48, 0xc7, 0xc7, 0x01, 0x00, 0x00, 0x00]);
                        
                        // MOV RSI, string_ptr (Placeholder: 0x48, 0xBE, 8 bytes)
                        machine_code.push(0x48);
                        machine_code.push(0xBE);
                        machine_code.extend_from_slice(&[0u8; 8]);
                        
                        // MOV RDX, string_len
                        machine_code.extend_from_slice(&[0x48, 0xc7, 0xc2]);
                        machine_code.extend_from_slice(&(processed_text.len() as u32).to_le_bytes());
                        
                        // MOV RAX, 1 (write)
                        machine_code.extend_from_slice(&[0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00]);
                        
                        // SYSCALL
                        machine_code.extend_from_slice(&[0x0f, 0x05]);
                    }
                }
            }
        }

        if trimmed.contains("return") {
            let val_str: String = trimmed.chars().skip_while(|c| !c.is_numeric()).take_while(|c| c.is_numeric()).collect();
            let val = val_str.parse::<u64>().unwrap_or(0);
            
            // MOV RDI, val
            machine_code.extend_from_slice(&[0x48, 0xc7, 0xc7]);
            machine_code.extend_from_slice(&(val as u32).to_le_bytes());
            
            // MOV RAX, 60 (exit)
            machine_code.extend_from_slice(&[0x48, 0xc7, 0xc0, 0x3c, 0x00, 0x00, 0x00]);
            machine_code.extend_from_slice(&[0x0f, 0x05]);
        }
    }

    if !source_code.contains("return") {
        machine_code.extend_from_slice(&[0x48, 0xc7, 0xc7, 0x00, 0x00, 0x00, 0x00]);
        machine_code.extend_from_slice(&[0x48, 0xc7, 0xc0, 0x3c, 0x00, 0x00, 0x00, 0x0f, 0x05]);
    }

    // --- ALO v2 Assembly ---
    // Code Segment Base: 0x400000 (X)
    // Data Segment Base: 0x500000 (W)
    let code_vaddr: u64 = 0x400000;
    let data_vaddr: u64 = 0x500000;
    
    // Patch RSI Pointers
    let mut string_idx = 0;
    let mut total_string_offset = 0;
    let mut i = 0;
    while i < machine_code.len() - 9 {
        if machine_code[i] == 0x48 && machine_code[i+1] == 0xBE {
            let addr = data_vaddr + (total_string_offset as u64);
            let addr_bytes = addr.to_le_bytes();
            for b_idx in 0..8 {
                machine_code[i + 2 + b_idx] = addr_bytes[b_idx];
            }
            if string_idx < string_lengths.len() {
                total_string_offset += string_lengths[string_idx];
                string_idx += 1;
            }
            i += 9;
        }
        i += 1;
    }

    let header_size = core::mem::size_of::<AloHeader>() as u32;
    let seg_size = core::mem::size_of::<AloSegment>() as u32;
    let code_offset = (header_size + (2 * seg_size)) as u64;
    let data_offset = code_offset + (machine_code.len() as u64);

    let header = AloHeader {
        magic: *b"ALO\x02",
        entry: code_vaddr,
        segment_count: 2,
        header_size,
    };

    let code_seg = AloSegment {
        typ: 1, // CODE
        flags: 5, // R | X
        offset: code_offset,
        vaddr: code_vaddr,
        size: machine_code.len() as u64,
    };

    let data_seg = AloSegment {
        typ: 2, // DATA
        flags: 3, // R | W
        offset: data_offset,
        vaddr: data_vaddr,
        size: data_section.len() as u64,
    };

    let mut alo_binary: Vec<u8> = Vec::new();
    
    // Push Header
    unsafe {
        let ptr = &header as *const _ as *const u8;
        alo_binary.extend_from_slice(core::slice::from_raw_parts(ptr, header_size as usize));
        
        let ptr = &code_seg as *const _ as *const u8;
        alo_binary.extend_from_slice(core::slice::from_raw_parts(ptr, seg_size as usize));
        
        let ptr = &data_seg as *const _ as *const u8;
        alo_binary.extend_from_slice(core::slice::from_raw_parts(ptr, seg_size as usize));
    }

    alo_binary.extend(machine_code);
    alo_binary.extend(data_section);

    let root = ROOT.lock();
    if let Some(r) = root.as_ref() {
        let _ = r.create(target_file, FileType::File);
        if let Ok(inode) = r.lookup(target_file) {
            if let Ok(handle) = inode.open(0) {
                 let _ = handle.truncate();
                 let _ = handle.write(&alo_binary, 0);
                 video::put_str(&format!("Success! Produced ALO v2 binary: {} bytes\n", alo_binary.len()));
            }
        }
    }
}
