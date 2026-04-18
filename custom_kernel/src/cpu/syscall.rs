use core::arch::{asm, naked_asm};
use crate::drivers::video;

const MSR_EFER: u32 = 0xC0000080;
const MSR_STAR: u32 = 0xC0000081;
const MSR_LSTAR: u32 = 0xC0000082;
const MSR_FMASK: u32 = 0xC0000084;
const EFER_SCE: u64 = 1;

static mut KERNEL_STACK_PTR: u64 = 0;
static mut USER_STACK_BACKUP: u64 = 0;

pub unsafe fn init() {
    let efer = rdmsr(MSR_EFER);
    wrmsr(MSR_EFER, efer | EFER_SCE);

    // Star: 63:48 User Base (0x10), 47:32 Kernel Base (0x08)
    // 0x10 Base => Sysret CS=0x20(User Code), SS=0x18(User Data)
    let star = (0x0010u64 << 48) | (0x0008u64 << 32);
    wrmsr(MSR_STAR, star);

    wrmsr(MSR_LSTAR, syscall_handler as u64);
    wrmsr(MSR_FMASK, 0x200); // Disable IF
    
    // Set kernel stack for syscall (Temporary: use current RSP)
    let rsp: u64;
    asm!("mov {}, rsp", out(reg) rsp);
    KERNEL_STACK_PTR = rsp; // Use boot stack for now
}

unsafe fn wrmsr(msr: u32, val: u64) {
    let low = val as u32;
    let high = (val >> 32) as u32;
    asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nostack));
}

unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    asm!("rdmsr", out("eax") low, out("edx") high, in("ecx") msr, options(nostack));
    ((high as u64) << 32) | (low as u64)
}

pub unsafe fn syscall(id: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        in("rax") id,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        out("rcx") _,
        out("r11") _,
        lateout("rax") ret,
        options(nostack)
    );
    ret
}

#[unsafe(naked)]
extern "C" fn syscall_handler() {
    naked_asm!(
        // Save User RSP
        "mov [rip + {USER_BACKUP}], rsp",
        
        // Load Kernel RSP
        "mov rsp, [rip + {KERNEL_BACKUP}]",
        
        // Save scratch
        "push rcx", // User RIP
        "push r11", // User RFLAGS
        "push rbp",
        
        // Call Handler(ID=RAX, Arg1=RDI, Arg2=RSI, Arg3=RDX)
        // Rust ABI: RDI, RSI, RDX, RCX.
        // We want: RDI=RAX, RSI=RDI, RDX=RSI, RCX=RDX.
        "push rdx", // Save RDX (Arg3)
        "mov rcx, rdx", // Arg4 = Arg3
        "mov rdx, rsi", // Arg3 = Arg2
        "mov rsi, rdi", // Arg2 = Arg1
        "mov rdi, rax", // Arg1 = ID
        
        "sub rsp, 8", // Align
        "call rust_syscall_dispatch",
        "add rsp, 8", // Align
        
        "pop rdx", // Restore RDX? No, result in RAX. 
        "pop rbp",
        "pop r11",
        "pop rcx",
        
        // Restore User RSP
        "mov rsp, [rip + {USER_BACKUP}]",
        
        "sysretq",
        
        USER_BACKUP = sym USER_STACK_BACKUP,
        KERNEL_BACKUP = sym KERNEL_STACK_PTR,
    );
}

#[no_mangle]
extern "C" fn rust_syscall_dispatch(id: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    // Hot path — no debug output, no heap allocation
    match id {
        1 => { // Write
            // a1 = fd, a2 = ptr, a3 = len.
            let fd = a1 as usize;
            let len = a3 as usize;
            if len > 8192 { return 0; } // Limit for safety
            
            // Validate Pointer
            if !crate::mm::user::validate_user_ptr(a2, len) {
                 return 0; // Invalid Pointer
            }

            // Use stack buffer for small writes (avoids heap allocation)
            // Falls back to heap for larger writes
            if len <= 512 {
                let mut stack_buf = [0u8; 512];
                if crate::mm::user::copy_from_user(a2 as *const u8, &mut stack_buf[0..len]).is_err() {
                    return 0;
                }
                if fd <= 2 {
                    if let Ok(s) = core::str::from_utf8(&stack_buf[..len]) {
                        video::put_str(s);
                        return a3;
                    } else {
                        for &b in &stack_buf[..len] {
                            video::put_char(b as char);
                        }
                        return a3;
                    }
                } else {
                    let n = crate::process::scheduler::process_write(fd, &stack_buf[..len]);
                    if n >= 0 { return n as u64; }
                }
            } else {
                // Large write: fall back to heap
                let mut buf = alloc::vec![0u8; len];
                if crate::mm::user::copy_from_user(a2 as *const u8, &mut buf[0..len]).is_err() {
                    return 0;
                }
                if fd <= 2 {
                    if let Ok(s) = core::str::from_utf8(&buf) {
                        video::put_str(s);
                        return a3;
                    } else {
                        for &b in &buf {
                            video::put_char(b as char);
                        }
                        return a3;
                    }
                } else {
                    let n = crate::process::scheduler::process_write(fd, &buf);
                    if n >= 0 { return n as u64; }
                }
            }
            0
        }
        2 => { // Exec
            // a1 = path_ptr, a2 = path_len
            let len = a2 as usize;
            if len > 256 { return 0xFFFFFFFFFFFFFFFF; }
            
            if !crate::mm::user::validate_user_ptr(a1, len) {
                 return 0xFFFFFFFFFFFFFFFF;
            }

            let s = unsafe { core::slice::from_raw_parts(a1 as *const u8, len) };
            if let Ok(path) = core::str::from_utf8(s) {
                 let _ = video::put_str("Syscall Exec: ");
                 let _ = video::put_str(path);
                 let _ = video::put_str("\n");
                
                 let _ = video::put_str("\n");
                
                 match crate::process::loader::load_elf_from_file(path) {
                     Ok(pid) => pid as u64,
                     Err(_) => 0xFFFFFFFFFFFFFFFF, // -1
                 }
            } else {
                0xFFFFFFFFFFFFFFFF
            }
        }
        3 => { // IPC Send
            // a1 = port_handle (index in cap table), a2 = msg_ptr
            // Validate Message Pointer
            if !crate::mm::user::validate_user_ptr(a2, core::mem::size_of::<crate::ipc::port::Message>()) {
                 return 1; // Invalid Pointer
            }

            // Temporary: Direct Port ID usage (a1 = port_id)
            let msg_ptr = a2 as *const crate::ipc::port::Message;
            // Verify alignment? validate_user_ptr does not check alignment generally, but we should.
            if (msg_ptr as usize) % 8 != 0 { return 1; }

            let msg = unsafe { (*msg_ptr).clone() };
            if crate::ipc::port::send(a1 as usize, msg) {
                0
            } else {
                1 // Fail
            }
        }
        4 => { // IPC Recv
            // a1 = port_handle, a2 = buffer ptr
            // Validate Buffer Pointer
            if !crate::mm::user::validate_user_ptr(a2, core::mem::size_of::<crate::ipc::port::Message>()) {
                 return 1;
            }
            if (a2 as usize) % 8 != 0 { return 1; }

            if let Some(msg) = crate::ipc::port::receive(a1 as usize) {
                 let buf = a2 as *mut crate::ipc::port::Message;
                 unsafe { *buf = msg };
                 0
            } else {
                // Return 1 (No message / Would block)
                // Real implementation should BLOCK.
                1
            }
        }
        5 => sys_sleep(a1),
        6 => { // Sys_open (path, flags)
             // Need to copy path
             let len = a2 as usize;
             if len > 256 { return u64::MAX; }
             
             if !crate::mm::user::validate_user_ptr(a1, len) { return u64::MAX; }
             
             let s = unsafe { core::slice::from_raw_parts(a1 as *const u8, len) };
             if let Ok(path) = core::str::from_utf8(s) {
                 crate::process::scheduler::process_open(path, 0) as u64
             } else {
                 u64::MAX // -1
             }
        },
        7 => { // Sys_read (fd, buf, len)
             let fd = a1 as usize;
             let len = a3 as usize;
             if len == 0 || len > 8192 { return u64::MAX; }
             
             // Check if STDIN
             if fd == 0 {
                 if let Some(c) = crate::drivers::keyboard::pop_char() {
                     let mut buf = [c as u8; 1];
                     if crate::mm::user::copy_to_user(a2 as *mut u8, &buf[0..1]).is_ok() {
                         return 1;
                     }
                 }
                 return 0; // Would block (0 bytes read for now)
             }
             
             let mut buf = alloc::vec![0u8; len];
             let n = crate::process::scheduler::process_read(fd, &mut buf);
             if n >= 0 {
                  // Copy back
                  if crate::mm::user::copy_to_user(a2 as *mut u8, &buf[0..n as usize]).is_ok() {
                      n as u64
                  } else {
                      u64::MAX
                  }
             } else {
                 u64::MAX
             }
        },
        8 => { // Sys_close (fd)
            crate::process::scheduler::process_close(a1 as usize) as u64
        },
        60 => { // sys_exit(code)
            crate::process::scheduler::exit_current_task(a1 as isize);
            0 // Should not return
        },
        61 => { // sys_wait(pid)
            crate::process::scheduler::wait_pid(a1 as usize) as u64
        },
        24 => { // sys_yield()
            crate::process::scheduler::yield_now();
            0
        },
        10 => { // sys_uptime() -> ms
             unsafe { crate::process::scheduler::get_ticks() * 10 }
        },
        12 => { // sys_clone(entry, stack) -> pid
            // a1 = entry, a2 = stack
            // Validate Stack Pointer
             if !crate::mm::user::validate_user_ptr(a2, 8) { return 0xFFFFFFFFFFFFFFFF; }
             // Validate Entry Point (Rough check)
             if !crate::mm::user::validate_user_ptr(a1, 1) { return 0xFFFFFFFFFFFFFFFF; }
             
             crate::process::scheduler::clone_task(a1, a2) as u64
        },
        20 => crate::sem::sys_agent_op(a1, a2, a3),
        40 => sys_socket(a1),
        41 => sys_bind(a1, a2),
        42 => sys_listen(a1),
        43 => sys_accept(a1),
        _ => 0
    }
}

fn sys_socket(proto: u64) -> u64 {
    // 1 = TCP
    if proto != 1 { return u64::MAX; }
    crate::net::syscall_socket_tcp() as u64
}

fn sys_bind(fd: u64, port: u64) -> u64 {
    crate::net::syscall_bind(fd as usize, port as u16) as u64
}

fn sys_listen(fd: u64) -> u64 {
    crate::net::syscall_listen(fd as usize) as u64
}

fn sys_accept(fd: u64) -> u64 {
    crate::net::syscall_accept(fd as usize) as u64
}

fn sys_sleep(ms: u64) -> u64 {
    // 100 Hz = 10 ms per tick.
    // Ticks = ms / 10.
    let ticks_needed = ms / 10;
    if ticks_needed == 0 {
        crate::process::scheduler::yield_now();
        return 0;
    }
    
    let current_ticks = crate::process::scheduler::get_ticks();
    let target_ticks = current_ticks + ticks_needed;
    
    unsafe {
        crate::process::scheduler::set_current_sleep(target_ticks);
    }
    
    crate::process::scheduler::yield_now();
    0
}
