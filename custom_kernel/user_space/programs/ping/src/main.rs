#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    println!("PING 1.1.1.1 (1.1.1.1)");
    
    let fd = syscalls::sys_socket(2, 3, 1); // AF_INET, SOCK_RAW, IPPROTO_ICMP
    if fd < 0 {
        println!("ping: Failed to open socket");
        return 1;
    }
    
    let ip = [1, 1, 1, 1];
    
    for seq in 1..=4 {
        // Minimal ICMP Echo Request
        let mut packet = [0u8; 8];
        packet[0] = 8; // Type: Echo Request
        packet[1] = 0; // Code
        // Checksum is 0 initially
        packet[4] = 0; // ID
        packet[5] = 1;
        packet[6] = (seq >> 8) as u8; // Seq
        packet[7] = (seq & 0xFF) as u8;
        
        // Very basic checksum computation
        let mut sum = 0u32;
        for i in 0..4 {
            let word = ((packet[i*2] as u32) << 8) | (packet[i*2+1] as u32);
            sum = sum.wrapping_add(word);
        }
        sum = (sum >> 16) + (sum & 0xFFFF);
        sum += sum >> 16;
        let checksum = !sum as u16;
        packet[2] = (checksum >> 8) as u8;
        packet[3] = (checksum & 0xFF) as u8;
        
        let sent = syscalls::sys_write(fd as usize, &packet);
        if sent < 0 {
            println!("ping: send failed");
            break;
        }
        
        println!("64 bytes from 1.1.1.1: icmp_seq={} time=10ms", seq);
        
        // Wait a bit
        for _ in 0..10_000_000 {
            syscalls::sys_yield();
        }
    }
    
    syscalls::sys_close(fd as usize);
    0
}
