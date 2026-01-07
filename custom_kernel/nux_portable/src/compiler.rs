use std::vec::Vec;
use std::string::String;
use std::collections::BTreeMap;
use std::format;

// Import OpCodes from vm.rs logic (hardcoded here for now or shared)
// We should ideally share them, but for modularity I'll define map here.

pub fn compile(source: &str) -> Result<Vec<u8>, String> {
    let mut ops = Vec::new();
    let mut labels = BTreeMap::new(); // Label Name -> ByteOffset
    let mut label_refs = Vec::new(); // (ByteOffsetToPatch, LabelName)

    // Pass 1: Parse and Generate Code (with placeholder for labels)
    // 64 Byte Header
    ops.extend_from_slice(b"ANUX");
    ops.extend_from_slice(&[0u8; 60]); // Padding
    
    // We parse line by line
    for line in source.lines() {
        let line = line.trim();
        // Remove comments
        let line = if let Some(idx) = line.find(';') {
            &line[..idx]
        } else {
            line
        }.trim();

        if line.is_empty() { continue; }

        // Check Label
        if line.ends_with(':') {
            let label_name = &line[..line.len()-1];
            labels.insert(String::from(label_name), ops.len());
            continue;
        }

        // Parse Instruction
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mnemonic = parts[0].to_ascii_uppercase();

        match mnemonic.as_str() {
            "PUSH" => {
                ops.push(0x01); // OP_PUSH
                if parts.len() < 2 { return Err(format!("PUSH missing operand")); }
                let val = parts[1].parse::<i64>().map_err(|_| "Invalid number")?;
                ops.extend_from_slice(&val.to_le_bytes());
            },
            "POP" => ops.push(0x02),
            "ADD" => ops.push(0x10),
            "SUB" => ops.push(0x11),
            "MUL" => ops.push(0x12),
            "DIV" => ops.push(0x13),
            "MOD" => ops.push(0x14),
            "AND" => ops.push(0x18),
            "OR" => ops.push(0x19),
            "EQ" => ops.push(0x90),
            "NEQ" => ops.push(0x91),
            "LT" => ops.push(0x92),
            "GT" => ops.push(0x93),
            "LTE" => ops.push(0x94),
            "GTE" => ops.push(0x95),
            "DRAW_RECT" => ops.push(0x20),
            "DRAW_IMG" => ops.push(0x21),
            "SLEEP" => ops.push(0x30),
            "PRINT_CHAR" => ops.push(0x51),
            "INPUT" => ops.push(0x52),
            "PRINT_VAL" => ops.push(0x53),
            "PRINT_FLOAT" => ops.push(0x54),
            "FADD" => ops.push(0x1A),
            "FSUB" => ops.push(0x1B),
            "FMUL" => ops.push(0x1C),
            "FDIV" => ops.push(0x1D),
            "ITOF" => ops.push(0x1E),
            "FTOI" => ops.push(0x1F),
            "PEEK" => ops.push(0x40),
            "POKE" => ops.push(0x41),
            "PEEK8" => ops.push(0x42),
            "POKE8" => ops.push(0x43),
            
            // Vision
            "IMG_ALLOC" => ops.push(0xA0),
            "IMG_FREE" => ops.push(0xA1),
            "CAM_CAPTURE" => ops.push(0xA2),
            "IMG_GET" => ops.push(0xA3),
            "IMG_SET" => ops.push(0xA4),
            "IMG_FILTER" => ops.push(0xA5),
            
            "DEBUG" => ops.push(0x50), // DEBUG_PRINT
            "JMP" => {
                ops.push(0x60);
                if parts.len() < 2 { return Err(format!("JMP missing label")); }
                label_refs.push((ops.len(), String::from(parts[1])));
                ops.extend_from_slice(&[0u8; 8]); // Placeholder
            },
            "JE" => {
                ops.push(0x61);
                if parts.len() < 2 { return Err(format!("JE missing label")); }
                label_refs.push((ops.len(), String::from(parts[1])));
                ops.extend_from_slice(&[0u8; 8]);
            },
            "CALL" => {
                ops.push(0x70);
                if parts.len() < 2 { return Err(format!("CALL missing label")); }
                label_refs.push((ops.len(), String::from(parts[1])));
                ops.extend_from_slice(&[0u8; 8]);
            },
            "RET" => ops.push(0x71),
            "EXIT" => ops.push(0xFF),
            _ => return Err(format!("Unknown instruction: {}", mnemonic)),
        }
    }

    // Pass 2: Patch Labels
    for (offset, label_name) in label_refs {
        if let Some(&target_addr) = labels.get(&label_name) {
             let bytes = (target_addr as i64).to_le_bytes();
             for i in 0..8 {
                 ops[offset + i] = bytes[i];
             }
        } else {
            return Err(format!("Undefined label: {}", label_name));
        }
    }

    Ok(ops)
}
