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
        _ => {
            crate::println!("Unknown syscall: {}", sys_num);
            usize::MAX
        }
    }
}

// ----------------------------------------------------------------------------
// Low-Level Syscall Assembly Entry Point
// ----------------------------------------------------------------------------

// When user-space executes `syscall`, the CPU jumps here.
// R11 = RFLAGS, RCX = RIP.
// The syscall number is in RAX. Arguments in RDI, RSI, RDX, R10, R8, R9.
global_asm!(r#"
.global syscall_entry
syscall_entry:
    // Swap GS base to load kernel stack pointer
    swapgs
    
    // Save user stack pointer
    mov gs:[0x10], rsp
    // Load kernel stack pointer
    mov rsp, gs:[0x08]
    
    // Preserve registers according to System V ABI
    push rcx // User RIP
    push r11 // User RFLAGS
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    
    // We have:
    // sys_num = RAX
    // arg1 = RDI
    // arg2 = RSI
    // arg3 = RDX
    // arg4 = R10
    // arg5 = R8
    // arg6 = R9
    
    // We need for System V ABI:
    // arg1 (sys_num) = RDI
    // arg2 (arg1)    = RSI
    // arg3 (arg2)    = RDX
    // arg4 (arg3)    = RCX
    // arg5 (arg4)    = R8
    // arg6 (arg5)    = R9
    // arg7 (arg6)    = stack
    
    push r9      // arg6 on stack
    mov r9, r8   // arg5 to R9
    mov r8, r10  // arg4 to R8
    mov rcx, rdx // arg3 to RCX
    mov rdx, rsi // arg2 to RDX
    mov rsi, rdi // arg1 to RSI
    mov rdi, rax // sys_num to RDI
    
    call syscall_handler
    
    // Clean up the 7th argument from stack
    add rsp, 8
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    pop r11
    pop rcx
    
    // Restore user stack
    mov rsp, gs:[0x10]
    
    // Swap GS back to user space
    swapgs
    
    // Return to user space
    sysretq
"#);
