#![no_std]
#![no_main]


use libainux::syscalls;
use libainux::print;
use libainux::println;

#[no_mangle]
pub extern "C" fn main() -> isize {
    // Install SIGINT handler
    let act = syscalls::SigAction {
        handler: sigint_handler as usize,
        flags: 0,
        restorer: 0,
        mask: 0,
    };
    syscalls::sys_sigaction(2, &act, core::ptr::null_mut());

    println!("Welcome to Ainux User Shell (ush)!");
    
    let mut input = [0u8; 1024];
    let mut input_len = 0;
    
    loop {
        print!("ush> ");
        input_len = 0;
        
        loop {
            let mut buf = [0u8; 1];
            let n = syscalls::sys_read(0, &mut buf);
            if n > 0 {
                let c = buf[0] as char;
                if c == '\n' || c == '\r' {
                    println!();
                    break;
                } else if c == '\x08' || c == '\x7f' { // backspace
                    if input_len > 0 {
                        input_len -= 1;
                        // print backspace, space, backspace
                        print!("\x08 \x08");
                    }
                } else {
                    if input_len < input.len() {
                        input[input_len] = buf[0];
                        input_len += 1;
                        print!("{}", c);
                    }
                }
            } else {
                syscalls::sys_yield();
            }
        }
        
        let cmd = core::str::from_utf8(&input[..input_len]).unwrap_or("").trim();
        if cmd.is_empty() {
            continue;
        }
        
        if cmd == "exit" {
            break;
        }
        
        // Very basic parsing
        let mut parts = cmd.split_whitespace();
        let program = parts.next().unwrap_or("");
        
        // Spawn
        let pid = syscalls::sys_fork();
        if pid < 0 {
            println!("ush: fork failed");
        } else if pid == 0 {
            // Child
            // Try to execve
            let mut path_buf = [0u8; 256];
            let bin_prefix = b"/bin/";
            let mut path_len = 0;
            
            for &b in bin_prefix { path_buf[path_len] = b; path_len += 1; }
            for &b in program.as_bytes() { 
                if path_len < path_buf.len() { path_buf[path_len] = b; path_len += 1; }
            }
            
            let path = core::str::from_utf8(&path_buf[..path_len]).unwrap_or("");
            let res = syscalls::sys_execve(path);
            if res < 0 {
                let res2 = syscalls::sys_execve(program);
                if res2 < 0 {
                    println!("ush: command not found: {}", program);
                    syscalls::sys_exit(1);
                }
            }
        } else {
            // Parent
            unsafe { core::ptr::write_volatile(&mut FOREGROUND_CHILD, Some(pid as usize)); }
            let mut status = 0;
            syscalls::sys_wait4(pid, &mut status, 0, 0);
            unsafe { core::ptr::write_volatile(&mut FOREGROUND_CHILD, None); }
        }
    }
    
    0
}

static mut FOREGROUND_CHILD: Option<usize> = None;

extern "C" fn sigint_handler(_sig: usize) {
    unsafe {
        let child = core::ptr::read_volatile(&FOREGROUND_CHILD);
        if let Some(pid) = child {
            syscalls::sys_kill(pid, 2); // Send SIGINT to foreground child
        }
    }
}
