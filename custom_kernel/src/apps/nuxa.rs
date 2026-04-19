use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use crate::drivers::video;
use crate::fs::vfs::{ROOT, FileType};

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

pub fn cmd_nuxa(args: &[&str]) {
    if args.len() < 4 || args[2] != "-o" {
        video::put_str("Usage: nuxa <source.s> -o <target.alo>\n");
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
            video::put_str("nuxa: Source not found.\n");
            return;
        }
    }
    core::mem::drop(root);

    video::put_str(&format!("Assembling {} (AT&T Syntax)...\n", source_file));

    let mut machine_code: Vec<u8> = Vec::new();
    
    for line in source_code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#") || trimmed.starts_with(";") {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() { continue; }
        
        let mut op = parts[0].to_lowercase();
        // Remove trailing comma from parts[1] if present
        let mut arg1 = if parts.len() > 1 { parts[1].trim_matches(',') } else { "" };
        let mut arg2 = if parts.len() > 2 { parts[2] } else { "" };

        match op.as_str() {
            "movq" => {
                // AT&T: movq $imm, %reg  OR  movq %reg, %reg
                if arg1.starts_with('$') && arg2.starts_with('%') {
                    // movq $imm, %reg
                    let imm_str = &arg1[1..];
                    let imm = if imm_str.starts_with("0x") {
                        u64::from_str_radix(&imm_str[2..], 16).unwrap_or(0)
                    } else {
                        imm_str.parse::<u64>().unwrap_or(0)
                    };
                    
                    let reg_idx = get_reg_idx(arg2);
                    // REX.W + 0xB8 + rd
                    machine_code.push(0x48 | (if reg_idx > 7 { 1 } else { 0 }));
                    machine_code.push(0xB8 + (reg_idx % 8) as u8);
                    machine_code.extend_from_slice(&imm.to_le_bytes());
                } else if arg1.starts_with('%') && arg2.starts_with('%') {
                    // movq %reg_src, %reg_dest
                    let src = get_reg_idx(arg1);
                    let dest = get_reg_idx(arg2);
                    // REX.W + 0x89 + ModRM
                    machine_code.push(0x48);
                    machine_code.push(0x89);
                    machine_code.push(0xC0 | ((src % 8) << 3) as u8 | (dest % 8) as u8);
                }
            },
            "syscall" => {
                machine_code.extend_from_slice(&[0x0F, 0x05]);
            },
            "ret" => {
                machine_code.push(0xC3);
            },
            "pushq" => {
                if arg1.starts_with('%') {
                    let reg = get_reg_idx(arg1);
                    machine_code.push(0x50 + (reg % 8) as u8);
                }
            },
            "popq" => {
                if arg1.starts_with('%') {
                    let reg = get_reg_idx(arg1);
                    machine_code.push(0x58 + (reg % 8) as u8);
                }
            },
            "addq" => {
                 if arg1.starts_with('$') && arg2.starts_with('%') {
                    let imm_str = &arg1[1..];
                    let imm = imm_str.parse::<u32>().unwrap_or(0);
                    let reg = get_reg_idx(arg2);
                    // REX.W + 0x81 + /0 + imm32
                    machine_code.push(0x48);
                    machine_code.push(0x81);
                    machine_code.push(0xC0 | (reg % 8) as u8);
                    machine_code.extend_from_slice(&imm.to_le_bytes());
                 }
            },
            _ => {
                video::put_str(&format!("nuxa: Unknown instruction: {}\n", op));
            }
        }
    }

    // Binary Generation (ALO v2)
    let binary = build_alo_v2(machine_code);
    
    let root = ROOT.lock();
    if let Some(r) = root.as_ref() {
        let _ = r.create(target_file, FileType::File);
        if let Ok(inode) = r.lookup(target_file) {
            if let Ok(handle) = inode.open(0) {
                 let _ = handle.truncate();
                 let _ = handle.write(&binary, 0);
                 video::put_str(&format!("Success! Produced ALO v2: {} bytes\n", binary.len()));
            }
        }
    }
}

fn get_reg_idx(reg: &str) -> usize {
    match reg.to_lowercase().trim_matches('%') {
        "rax" => 0, "rcx" => 1, "rdx" => 2, "rbx" => 3,
        "rsp" => 4, "rbp" => 5, "rsi" => 6, "rdi" => 7,
        _ => 0,
    }
}

fn build_alo_v2(code: Vec<u8>) -> Vec<u8> {
    let header_size = core::mem::size_of::<AloHeader>() as u32;
    let seg_size = core::mem::size_of::<AloSegment>() as u32;
    let code_vaddr = 0x400000;
    
    let header = AloHeader {
        magic: *b"ALO\x02",
        entry: code_vaddr,
        segment_count: 1,
        header_size,
    };
    
    let seg = AloSegment {
        typ: 1, // CODE
        flags: 5, // R | X
        offset: (header_size + seg_size) as u64,
        vaddr: code_vaddr,
        size: code.len() as u64,
    };
    
    let mut bin = Vec::new();
    unsafe {
        let h_ptr = &header as *const _ as *const u8;
        bin.extend_from_slice(core::slice::from_raw_parts(h_ptr, header_size as usize));
        let s_ptr = &seg as *const _ as *const u8;
        bin.extend_from_slice(core::slice::from_raw_parts(s_ptr, seg_size as usize));
    }
    bin.extend(code);
    bin
}
