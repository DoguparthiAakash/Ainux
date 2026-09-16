// Mithl OS System Call Interface
// Provides the POSIX-like interface needed for Musl libc and LLVM to run.

use core::arch::asm;
use core::arch::global_asm;

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_MMAP: usize = 9;
pub const SYS_MUNMAP: usize = 11;
pub const SYS_EXIT: usize = 60;

/// Syscall handler entry point
/// Expected to be called by the `syscall` instruction handler in assembly.
#[no_mangle]
pub extern "C" fn syscall_handler(
    sys_num: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    let sys_num_i = sys_num as isize;
    
    if sys_num_i < 0 {
        crate::microkernel::ipc::handle_mach_trap(sys_num_i, arg1, arg2, arg3, arg4, arg5, arg6) as usize
    } else {
        handle_bsd_syscall(sys_num, arg1, arg2, arg3, arg4, arg5, arg6)
    }
}

fn handle_bsd_syscall(
    sys_num: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    match sys_num {
        SYS_READ => {
            let fd = arg1;
            let buf = arg2 as *mut u8;
            let len = arg3;
            // Validate pointer before passing to process_read
            if crate::mm::user::validate_user_range(buf as u64, len) {
                let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };
                crate::process::scheduler::process_read(fd, slice) as usize
            } else {
                usize::MAX // -EFAULT
            }
        }
        SYS_WRITE => {
            let fd = arg1;
            let buf = arg2 as *const u8;
            let len = arg3;
            
            if !crate::mm::user::validate_user_range(buf as u64, len) {
                return usize::MAX; // -EFAULT
            }
            
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            
            // For now, if writing to stdout (fd=1) or stderr (fd=2), output to serial/VGA
            if fd == 1 || fd == 2 {
                if let Ok(s) = core::str::from_utf8(slice) {
                    crate::print!("{}", s);
                }
                len
            } else {
                crate::process::scheduler::process_write(fd, slice) as usize
            }
        }
        SYS_OPEN => {
            let path_ptr = arg1 as *const u8;
            let flags = arg2 as u32;
            
            // Basic string read from user space
            let mut len = 0;
            while crate::mm::user::validate_user_range(path_ptr as u64 + len, 1) {
                if unsafe { *path_ptr.add(len as usize) } == 0 {
                    break;
                }
                len += 1;
                if len > 4096 { break; }
            }
            
            let slice = unsafe { core::slice::from_raw_parts(path_ptr, len as usize) };
            if let Ok(path_str) = core::str::from_utf8(slice) {
                crate::process::scheduler::process_open(path_str, flags) as usize
            } else {
                usize::MAX // -EFAULT
            }
        }
        SYS_CLOSE => {
            let fd = arg1;
            crate::process::scheduler::process_close(fd) as usize
        }
        SYS_MMAP => {
            let addr = arg1 as u64;
            let length = arg2 as u64;
            let prot = arg3 as u32;
            let flags = arg4 as u32;
            let fd = arg5 as i32;
            let offset = arg6 as u64;
            crate::process::scheduler::process_mmap(addr, length, prot, flags, fd, offset) as usize
        }
        SYS_EXIT => {
            let exit_code = arg1 as isize;
            crate::process::scheduler::exit_current_task(exit_code);
            0
        }
        41 => { // SYS_SOCKET
            crate::net::sys_socket(arg1 as i32, arg2 as i32, arg3 as i32) as usize
        }
        42 => { // SYS_CONNECT
            crate::net::sys_connect(arg1, arg2 as *const u8, arg3) as usize
        }
        43 => { // SYS_ACCEPT
            crate::net::sys_accept(arg1) as usize
        }
        49 => { // SYS_BIND
            crate::net::sys_bind(arg1, arg2 as *const u8, arg3) as usize
        }
        50 => { // SYS_LISTEN
            crate::net::sys_listen(arg1, arg2 as i32) as usize
        }
        _ => {
            crate::println!("Unknown syscall: {}", sys_num);
            usize::MAX
        }
    }
}

// ----------------------------------------------------------------------------
// Low-Level Syscall Assembly Entry Point
// ----------------------------------------------------------------------------

// The low-level syscall assembly entry point `syscall_entry` is defined in
// `arch/x86_64/asm/syscall.asm` and linked during the build process.
