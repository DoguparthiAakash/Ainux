#![no_std]
#![no_main]

use libainux::{print, println, syscalls};

fn parse_usize(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut val = 0;
    for c in s.chars() {
        if c >= '0' && c <= '9' {
            val = val * 10 + (c as usize - '0' as usize);
        } else {
            return None;
        }
    }
    Some(val)
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    println!("kill: Enter PID to kill:");
    
    let mut buf = [0u8; 32];
    let n = syscalls::sys_read(0, &mut buf);
    if n > 0 {
        if let Ok(s) = core::str::from_utf8(&buf[..n as usize]) {
            if let Some(pid) = parse_usize(s) {
                let res = syscalls::sys_kill(pid, 9); // SIGKILL
                if res < 0 {
                    println!("kill: Failed to kill PID {}", pid);
                } else {
                    println!("kill: Sent SIGKILL to PID {}", pid);
                }
            } else {
                println!("kill: Invalid PID");
            }
        }
    }
    0
}
