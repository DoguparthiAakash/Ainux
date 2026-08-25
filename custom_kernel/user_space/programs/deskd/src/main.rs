#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

fn parse_u32(s: &str) -> Option<u32> {
    let mut val: u32 = 0;
    for b in s.bytes() {
        if b >= b'0' && b <= b'9' {
            val = val.wrapping_mul(10).wrapping_add((b - b'0') as u32);
        } else {
            return None;
        }
    }
    Some(val)
}

fn parse_u8(s: &str) -> Option<u8> {
    let mut val: u8 = 0;
    for b in s.bytes() {
        if b >= b'0' && b <= b'9' {
            val = val.wrapping_mul(10).wrapping_add(b - b'0');
        } else {
            return None;
        }
    }
    Some(val)
}

fn parse_hex_u8(s: &str) -> Option<u8> {
    let mut s = s;
    if s.starts_with("0x") {
        s = &s[2..];
    }
    let mut val: u8 = 0;
    for b in s.bytes() {
        let v = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => return None,
        };
        val = val.wrapping_mul(16).wrapping_add(v);
    }
    Some(val)
}


fn print_hex(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:08x}  ", i * 16);
        for &b in chunk {
            print!("{:02x} ", b);
        }
        for _ in 0..(16 - chunk.len()) {
            print!("   ");
        }
        print!(" |");
        for &b in chunk {
            if b >= 32 && b <= 126 {
                print!("{}", b as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
}

fn handle_read(parts: &[&str]) {
    if parts.len() < 3 {
        println!("Usage: read <lba> <sectors>");
        return;
    }
    let lba = parse_u32(parts[1]).unwrap_or(0);
    let sectors = parse_u8(parts[2]).unwrap_or(1);
    
    // Allocate a buffer on the stack for up to 8 sectors (4KB)
    if sectors > 8 {
        println!("Cannot read more than 8 sectors at once");
        return;
    }
    
    let mut buf = [0u8; 4096];
    let slice = &mut buf[0..(sectors as usize * 512)];
    
    let res = syscalls::sys_disk_read(lba, sectors, slice);
    if res > 0 {
        print_hex(slice);
    } else {
        println!("Disk read failed.");
    }
}

fn handle_mbr() {
    let mut buf = [0u8; 512];
    if syscalls::sys_disk_read(0, 1, &mut buf) > 0 {
        if buf[510] != 0x55 || buf[511] != 0xAA {
            println!("Invalid MBR signature!");
            return;
        }
        println!("MBR Signature OK.");
        println!("Partition Table:");
        for i in 0..4 {
            let offset = 446 + (i * 16);
            let status = buf[offset];
            let ptype = buf[offset + 4];
            
            let lba_start = u32::from_le_bytes([
                buf[offset + 8], buf[offset + 9], buf[offset + 10], buf[offset + 11]
            ]);
            let lba_len = u32::from_le_bytes([
                buf[offset + 12], buf[offset + 13], buf[offset + 14], buf[offset + 15]
            ]);
            
            if ptype != 0 {
                println!("  Partition {}: Status={:#04x}, Type={:#04x}, LBA Start={}, Sectors={} ({} MB)",
                         i + 1, status, ptype, lba_start, lba_len, (lba_len as u64 * 512) / (1024 * 1024));
            }
        }
    } else {
        println!("Disk read failed.");
    }
}

fn handle_ext4(parts: &[&str]) {
    if parts.len() < 2 {
        println!("Usage: ext4 <partition_lba>");
        return;
    }
    let base_lba = parse_u32(parts[1]).unwrap_or(0);
    
    // Ext4 superblock is at offset 1024, which is exactly LBA base_lba + 2 (assuming 512B sectors)
    let mut buf = [0u8; 1024]; // read 2 sectors
    
    if syscalls::sys_disk_read(base_lba + 2, 2, &mut buf) > 0 {
        let magic = u16::from_le_bytes([buf[0x38], buf[0x39]]);
        if magic == 0xEF53 {
            println!("Ext4 Superblock found at LBA {}!", base_lba + 2);
            let blocks_count_lo = u32::from_le_bytes([buf[0x4], buf[0x5], buf[0x6], buf[0x7]]);
            let blocks_count_hi = u32::from_le_bytes([buf[0x150], buf[0x151], buf[0x152], buf[0x153]]);
            let block_size_shift = u32::from_le_bytes([buf[0x18], buf[0x19], buf[0x1A], buf[0x1B]]);
            let block_size = 1024 << block_size_shift;
            
            println!("  Magic:      {:#06x}", magic);
            println!("  Block size: {}", block_size);
            println!("  Blocks:     {}", blocks_count_lo); // Simplified, ignores hi
            
            let total_size = (blocks_count_lo as u64) * (block_size as u64);
            println!("  Total size: {} MB", total_size / (1024 * 1024));
        } else {
            println!("Ext4 Superblock not found. Magic was {:#06x}", magic);
        }
    } else {
        println!("Disk read failed.");
    }
}

fn handle_size() {
    let mut ata_buf = [0u8; 512];
    if syscalls::sys_disk_identify(&mut ata_buf) > 0 {
        let words = unsafe { core::slice::from_raw_parts(ata_buf.as_ptr() as *const u16, 256) };
        let sectors28 = (words[60] as u32) | ((words[61] as u32) << 16);
        let mut sectors = sectors28 as u64;
        
        if sectors == 0 {
            let s0 = words[100] as u64;
            let s1 = words[101] as u64;
            let s2 = words[102] as u64;
            let s3 = words[103] as u64;
            sectors = s0 | (s1 << 16) | (s2 << 32) | (s3 << 48);
        }
        let total_size = sectors * 512;
        println!("Hardware Disk Size: {} bytes ({} MB)", total_size, total_size / (1024 * 1024));
        
        print!("Model: ");
        for i in 27..47 {
            let offset = i * 2;
            let b1 = ata_buf[offset + 1] as char;
            let b2 = ata_buf[offset] as char;
            if b1 != '\0' && b1 != ' ' { print!("{}", b1); }
            if b2 != '\0' && b2 != ' ' { print!("{}", b2); }
        }
        println!();
    } else {
        println!("Identify failed.");
    }
}

fn handle_mkpart(parts: &[&str]) {
    if parts.len() < 5 {
        println!("Usage: mkpart <idx 0-3> <type_hex> <start_lba> <size_sectors>");
        return;
    }
    let idx = parse_u8(parts[1]).unwrap_or(0);
    if idx > 3 {
        println!("Index must be 0-3");
        return;
    }
    let ptype = parse_hex_u8(parts[2]).unwrap_or(0);
    let start = parse_u32(parts[3]).unwrap_or(0);
    let size = parse_u32(parts[4]).unwrap_or(0);
    
    let mut buf = [0u8; 512];
    if syscalls::sys_disk_read(0, 1, &mut buf) > 0 {
        if buf[510] != 0x55 || buf[511] != 0xAA {
            println!("Warning: MBR signature invalid, initializing MBR.");
            buf[510] = 0x55;
            buf[511] = 0xAA;
        }
        let offset = 446 + (idx as usize * 16);
        buf[offset] = 0x80; // active/bootable
        buf[offset + 4] = ptype;
        let start_bytes = start.to_le_bytes();
        buf[offset + 8] = start_bytes[0];
        buf[offset + 9] = start_bytes[1];
        buf[offset + 10] = start_bytes[2];
        buf[offset + 11] = start_bytes[3];
        
        let size_bytes = size.to_le_bytes();
        buf[offset + 12] = size_bytes[0];
        buf[offset + 13] = size_bytes[1];
        buf[offset + 14] = size_bytes[2];
        buf[offset + 15] = size_bytes[3];
        
        if syscalls::sys_disk_write(0, 1, &buf) > 0 {
            println!("Partition {} created.", idx);
        } else {
            println!("Write failed.");
        }
    }
}

fn handle_rmpart(parts: &[&str]) {
    if parts.len() < 2 {
        println!("Usage: rmpart <idx 0-3>");
        return;
    }
    let idx = parse_u8(parts[1]).unwrap_or(0);
    if idx > 3 {
        println!("Index must be 0-3");
        return;
    }
    let mut buf = [0u8; 512];
    if syscalls::sys_disk_read(0, 1, &mut buf) > 0 {
        let offset = 446 + (idx as usize * 16);
        for i in 0..16 {
            buf[offset + i] = 0;
        }
        if syscalls::sys_disk_write(0, 1, &buf) > 0 {
            println!("Partition {} removed.", idx);
        } else {
            println!("Write failed.");
        }
    }
}

fn handle_write(parts: &[&str]) {
    if parts.len() < 4 {
        println!("Usage: write <lba> <sectors> <hex_byte>");
        return;
    }
    let lba = parse_u32(parts[1]).unwrap_or(0);
    let sectors = parse_u8(parts[2]).unwrap_or(1);
    let val = parse_hex_u8(parts[3]).unwrap_or(0);
    if sectors > 8 {
        println!("Cannot write more than 8 sectors at once");
        return;
    }
    let mut buf = [0u8; 4096];
    for b in buf.iter_mut() {
        *b = val;
    }
    if syscalls::sys_disk_write(lba, sectors, &buf[0..(sectors as usize * 512)]) > 0 {
        println!("Wrote byte {:#04x} to {} sectors starting at LBA {}", val, sectors, lba);
    } else {
        println!("Write failed.");
    }
}

fn handle_zero(parts: &[&str]) {
    if parts.len() < 3 {
        println!("Usage: zero <lba> <sectors>");
        return;
    }
    let lba = parse_u32(parts[1]).unwrap_or(0);
    let sectors = parse_u8(parts[2]).unwrap_or(1);
    if sectors > 8 {
        println!("Cannot zero more than 8 sectors at once");
        return;
    }
    let buf = [0u8; 4096];
    if syscalls::sys_disk_write(lba, sectors, &buf[0..(sectors as usize * 512)]) > 0 {
        println!("Zeroed {} sectors starting at LBA {}", sectors, lba);
    } else {
        println!("Zeroing failed.");
    }
}

#[unsafe(no_mangle)]

pub extern "C" fn main() -> isize {
    println!("=== Deskd Disk Repair Tool ===");
    println!("Type 'help' for commands.");

    loop {
        print!("disk> ");
        
        let mut buf = [0u8; 128];
        let mut idx = 0;
        
        loop {
            let mut c_buf = [0u8; 1];
            let n = syscalls::sys_read(0, &mut c_buf);
            
            if n > 0 {
                let c = c_buf[0];
                
                // Echo
                syscalls::sys_write(1, &c_buf);
                
                if c == b'\n' || c == b'\r' {
                    break;
                }
                
                if idx < buf.len() - 1 {
                    buf[idx] = c;
                    idx += 1;
                }
            } else {
                syscalls::sys_yield();
            }
        }
        println!();
        
        buf[idx] = 0; // null terminate
        if idx == 0 { continue; }
        
        if let Ok(cmd_str) = core::str::from_utf8(&buf[0..idx]) {
            let cmd_str = cmd_str.trim();
            if cmd_str.is_empty() { continue; }
            
            let mut parts = [""; 10];
            let mut part_idx = 0;
            for part in cmd_str.split_whitespace() {
                if part_idx < 10 {
                    parts[part_idx] = part;
                    part_idx += 1;
                }
            }
            
            let parts = &parts[0..part_idx];
            
            match parts[0] {
                "exit" => break,
                "help" => {
                    println!("Commands:");
                    println!("  size               - Get true hardware disk size and model");
                    println!("  mbr                - Parse partition table from LBA 0");
                    println!("  mkpart <idx> <type> <lba> <sz> - Create/overwrite MBR partition");
                    println!("  rmpart <idx>       - Remove MBR partition");
                    println!("  ext4 <lba>         - Check Ext4 Superblock at partition start LBA");
                    println!("  read <lba> <num>   - Hexdump <num> sectors from <lba>");
                    println!("  write <lba> <num> <hex> - Fill <num> sectors with <hex> byte");
                    println!("  zero <lba> <num>   - Zero fill <num> sectors");
                    println!("  exit               - Exit shell");
                },
                "size" => handle_size(),
                "mbr" => handle_mbr(),
                "ext4" => handle_ext4(parts),
                "read" => handle_read(parts),
                "write" => handle_write(parts),
                "zero" => handle_zero(parts),
                "mkpart" => handle_mkpart(parts),
                "rmpart" => handle_rmpart(parts),
                _ => println!("Unknown command: {}", parts[0]),
            }
        }

    }
    
    println!("Exiting.");
    0
}
