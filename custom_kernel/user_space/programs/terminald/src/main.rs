#![no_std]
#![no_main]

use libainux::syscalls;
use libainux::println;

#[no_mangle]
pub extern "C" fn main() -> isize {
    println!("Terminal Daemon (terminald) Started");
    
    let mut p_in = [0i32; 2];
    let mut p_out = [0i32; 2];
    
    if syscalls::sys_pipe(&mut p_in) < 0 {
        println!("terminald: failed to create p_in");
        return 1;
    }
    if syscalls::sys_pipe(&mut p_out) < 0 {
        println!("terminald: failed to create p_out");
        return 1;
    }
    
    let pid = syscalls::sys_fork();
    if pid < 0 {
        println!("terminald: fork failed");
        return 1;
    } else if pid == 0 {
        // Child: Userspace Shell
        syscalls::sys_dup2(p_in[0] as usize, 0); // stdin
        syscalls::sys_dup2(p_out[1] as usize, 1); // stdout
        syscalls::sys_dup2(p_out[1] as usize, 2); // stderr
        
        syscalls::sys_close(p_in[0] as usize);
        syscalls::sys_close(p_in[1] as usize);
        syscalls::sys_close(p_out[0] as usize);
        syscalls::sys_close(p_out[1] as usize);
        
        let res = syscalls::sys_execve("/bin/sh");
        if res < 0 {
            // Because terminald uses kernel print directly through sys_write(1), 
            // wait, stdout was just redirected! We can't print normally unless we write to video directly.
            // But we can just exit.
            syscalls::sys_exit(1);
        }
    } else {
        // Parent: Terminal Daemon
        syscalls::sys_close(p_in[0] as usize);
        syscalls::sys_close(p_out[1] as usize);
        
        let in_write = p_in[1] as usize;
        let out_read = p_out[0] as usize;
        
        loop {
            // Read from keyboard (fd 0)
            let mut key_buf = [0u8; 1];
            let n1 = syscalls::sys_read(0, &mut key_buf);
            if n1 > 0 {
                if key_buf[0] == 3 { // Ctrl+C
                    syscalls::sys_kill(pid as usize, 2); // Send SIGINT to shell
                } else {
                    // Forward to shell
                    syscalls::sys_write(in_write, &key_buf);
                }
            }
            
            // Read from shell (pipe out_read)
            let mut out_buf = [0u8; 64];
            let n2 = syscalls::sys_read(out_read, &mut out_buf);
            if n2 > 0 {
                // Write to video (fd 1)
                syscalls::sys_write(1, &out_buf[..n2 as usize]);
            }
            
            if n1 <= 0 && n2 <= 0 {
                syscalls::sys_yield();
            }
        }
    }
    
    0
}
