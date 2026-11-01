#![no_std]
#![no_main]

use libainux::syscalls;
use libainux::print;
use libainux::println;

static mut FOREGROUND_CHILD: Option<usize> = None;

extern "C" fn sigint_handler(_sig: usize) {
    unsafe {
        let child = core::ptr::read_volatile(&raw const FOREGROUND_CHILD);
        if let Some(pid) = child {
            syscalls::sys_kill(pid, 2); // Send SIGINT to foreground child
        }
    }
}

fn execute_command_string(cmd_str: &str) {
    let cmd = cmd_str.trim();
    if cmd.is_empty() {
        return;
    }

    // Check for pipe '|'
    if let Some(pipe_idx) = cmd.find('|') {
        let left = cmd[..pipe_idx].trim();
        let right = cmd[pipe_idx + 1..].trim();

        if left.is_empty() || right.is_empty() {
            println!("ush: syntax error near unexpected token '|'");
            return;
        }

        let mut p = [0i32; 2];
        if syscalls::sys_pipe(&mut p) < 0 {
            println!("ush: failed to create pipe");
            return;
        }

        let pid1 = syscalls::sys_fork();
        if pid1 == 0 {
            // Left child: stdout -> p[1]
            syscalls::sys_close(p[0] as usize);
            syscalls::sys_dup2(p[1] as usize, 1);
            syscalls::sys_close(p[1] as usize);
            run_single_command(left);
            syscalls::sys_exit(0);
        }

        let pid2 = syscalls::sys_fork();
        if pid2 == 0 {
            // Right child: stdin -> p[0]
            syscalls::sys_close(p[1] as usize);
            syscalls::sys_dup2(p[0] as usize, 0);
            syscalls::sys_close(p[0] as usize);
            run_single_command(right);
            syscalls::sys_exit(0);
        }

        // Parent closes both pipe ends and waits
        syscalls::sys_close(p[0] as usize);
        syscalls::sys_close(p[1] as usize);

        let mut status = 0;
        syscalls::sys_wait4(pid1, &mut status, 0, 0);
        syscalls::sys_wait4(pid2, &mut status, 0, 0);
        return;
    }

    // Single command execution (may include redirection)
    run_single_command(cmd);
}

fn run_single_command(cmd: &str) {
    // Check redirection '>'
    let (clean_cmd, out_file) = if let Some(redir_idx) = cmd.find('>') {
        let command_part = cmd[..redir_idx].trim();
        let file_part = cmd[redir_idx + 1..].trim();
        (command_part, Some(file_part))
    } else {
        (cmd, None)
    };

    let mut parts = clean_cmd.split_whitespace();
    let program = match parts.next() {
        Some(p) => p,
        None => return,
    };

    // Shell Builtins (run in current process)
    match program {
        "cd" => {
            let target = parts.next().unwrap_or("/");
            if syscalls::sys_chdir(target) < 0 {
                println!("ush: cd: {}: No such directory", target);
            }
            return;
        }
        "pwd" => {
            let mut buf = [0u8; 256];
            let n = syscalls::sys_getcwd(&mut buf);
            if n > 0 {
                let s = core::str::from_utf8(&buf[..n as usize]).unwrap_or("");
                println!("{}", s.trim_matches('\0'));
            } else {
                println!("/");
            }
            return;
        }
        "echo" => {
            let mut first = true;
            for arg in parts {
                if !first {
                    print!(" ");
                }
                print!("{}", arg);
                first = false;
            }
            println!();
            return;
        }
        "help" => {
            println!("Mithl OS Unix V7 User Shell (ush)");
            println!("Builtins: cd, pwd, echo, help, exit");
            println!("Utilities: ls, cat, cp, mv, rm, mkdir, rmdir, grep, ping, httpd, nc");
            println!("Features: pipes ('|'), output redirection ('>')");
            return;
        }
        _ => {}
    }

    // External binary execution via fork + execve
    let pid = syscalls::sys_fork();
    if pid < 0 {
        println!("ush: fork failed");
        return;
    }

    if pid == 0 {
        // Child process
        if let Some(filename) = out_file {
            let fd = syscalls::sys_open(filename, 0x02 | 0x40 | 0x200); // O_RDWR | O_CREAT | O_TRUNC
            if fd >= 0 {
                syscalls::sys_dup2(fd as usize, 1);
                syscalls::sys_close(fd as usize);
            } else {
                println!("ush: cannot open {} for redirection", filename);
                syscalls::sys_exit(1);
            }
        }

        // Search PATH: /bin/<prog>, /usr/bin/<prog>, <prog>
        let mut path_buf = [0u8; 256];
        let bin_prefix = b"/bin/";
        let mut path_len = 0;
        for &b in bin_prefix { path_buf[path_len] = b; path_len += 1; }
        for &b in program.as_bytes() {
            if path_len < path_buf.len() { path_buf[path_len] = b; path_len += 1; }
        }
        let bin_path = core::str::from_utf8(&path_buf[..path_len]).unwrap_or("");

        let res = syscalls::sys_execve(bin_path);
        if res < 0 {
            let res2 = syscalls::sys_execve(program);
            if res2 < 0 {
                println!("ush: {}: command not found", program);
                syscalls::sys_exit(127);
            }
        }
        syscalls::sys_exit(0);
    } else {
        // Parent process: wait for foreground child
        unsafe { core::ptr::write_volatile(&raw mut FOREGROUND_CHILD, Some(pid as usize)); }
        let mut status = 0;
        syscalls::sys_wait4(pid, &mut status, 0, 0);
        unsafe { core::ptr::write_volatile(&raw mut FOREGROUND_CHILD, None); }
    }
}

#[no_mangle]
pub extern "C" fn main() -> isize {
    // Install SIGINT handler (V7 Unix style)
    let act = syscalls::SigAction {
        handler: sigint_handler as *const () as usize,
        flags: 0,
        restorer: 0,
        mask: 0,
    };
    syscalls::sys_sigaction(2, &act, core::ptr::null_mut());

    println!("Mithl OS Unix V7 Shell (ush)");

    let mut input = [0u8; 1024];
    let mut input_len: usize;

    loop {
        // Display prompt
        let mut cwd_buf = [0u8; 128];
        let n = syscalls::sys_getcwd(&mut cwd_buf);
        if n > 0 {
            let cwd = core::str::from_utf8(&cwd_buf[..n as usize]).unwrap_or("");
            print!("ush:{}$ ", cwd.trim_matches('\0'));
        } else {
            print!("ush$ ");
        }

        input_len = 0;

        // Line reading loop with backspace support
        loop {
            let mut buf = [0u8; 1];
            let n = syscalls::sys_read(0, &mut buf);
            if n > 0 {
                let c = buf[0] as char;
                if c == '\n' || c == '\r' {
                    println!();
                    break;
                } else if c == '\x08' || c == '\x7f' { // Backspace
                    if input_len > 0 {
                        input_len -= 1;
                        print!("\x08 \x08");
                    }
                } else if c >= ' ' && c <= '~' {
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

        execute_command_string(cmd);
    }

    0
}
