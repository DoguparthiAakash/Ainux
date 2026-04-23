use core::arch::{asm, naked_asm};
use crate::drivers::video;
use crate::cpu::control::{rdmsr, wrmsr};

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


pub unsafe fn syscall(id: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
    let cs: u16;
    asm!("mov {:x}, cs", out(reg) cs);
    if cs == 0x08 {
        return rust_syscall_dispatch(id, a1, a2, a3, a4, a5);
    }

    let ret: u64;
    asm!(
        "syscall",
        in("rax") id,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        in("r8")  a5,
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
        
        // Call Handler(ID=RAX, Arg1=RDI, Arg2=RSI, Arg3=RDX, Arg4=R10, Arg5=R8)
        // Rust ABI (AMD64): RDI, RSI, RDX, RCX, R8, R9.
        // We map to these. Note: Syscall R10 becomes RCX for Rust.
        "push r9",
        "push r8",
        "mov r9, r8",   // Arg6 (R9) = R8 (Syscall Arg5)
        "mov r8, r10",  // Arg5 (R8) = R10 (Syscall Arg4)
        "mov rcx, rdx", // Arg4 (RCX) = RDX (Syscall Arg3)
        "mov rdx, rsi", // Arg3 (RDX) = RSI (Syscall Arg2)
        "mov rsi, rdi", // Arg2 (RSI) = RDI (Syscall Arg1)
        "mov rdi, rax", // Arg1 (RDI) = ID (RAX)
        
        "sub rsp, 32", // Shadow space for Win64-like calls or alignment
        "call rust_syscall_dispatch",
        "add rsp, 32",
        
        "pop r8", // Pop back
        "pop r9",
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
extern "C" fn rust_syscall_dispatch(id: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
    // Increment Syscall Count (Maturity Metering)
    {
        let mut tasks = crate::process::scheduler::TASKS.lock();
        let current_pid = crate::process::scheduler::get_current_pid();
        if let Some(task) = &mut tasks[current_pid] {
            task.syscall_count += 1;
        }
    }

    // Hot path — no debug output, no heap allocation
    match id {
        1 => { // Write (handle_idx, ptr, len)
            let handle_idx = a1 as usize;
            let len = a3 as usize;
            if len > 8192 { return 0; }
            if !crate::mm::user::validate_user_ptr(a2, len) { return 0; }

            // Capability Check: Must have WRITE capability
            let pid = crate::process::scheduler::get_current_pid();
            let handle_idx = a1 as usize;
            
            let handle_table = {
                let tasks = crate::process::scheduler::TASKS.lock();
                if let Some(task) = &tasks[pid] {
                    // We need a way to keep handle_table alive or just use it here.
                    // Since Task is in a Mutex, we can't easily get a long-lived ref.
                    // But we can just use the indices.
                    // Better: just lock handle_table inside the tasks lock briefly.
                    let h_table = task.handle_table.lock();
                    h_table.get(handle_idx, crate::object::CapabilitySet::WRITE).is_some()
                } else {
                    false
                }
            };

            if handle_table {
                // Hack: if fd 1/2, still go to video
                if handle_idx <= 2 {
                     let mut buf = alloc::vec![0u8; len];
                     if crate::mm::user::copy_from_user(a2 as *const u8, &mut buf).is_ok() {
                         if let Ok(s) = core::str::from_utf8(&buf) {
                            video::put_str(s);
                            return a3;
                         }
                     }
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
        9 => { // Sys_kill (handle_idx, sig)
            let handle_idx = a1 as usize;
            let sig = a2 as u32;
            let pid = crate::process::scheduler::get_current_pid();

            let target_pid = {
                let tasks = crate::process::scheduler::TASKS.lock();
                if let Some(task) = &tasks[pid] {
                    let ht = task.handle_table.lock();
                    ht.get(handle_idx, crate::object::CapabilitySet::DESTROY).map(|obj| obj.id())
                } else {
                    None
                }
            };

            if let Some(target_id) = target_pid {
                crate::process::scheduler::post_signal(target_id, sig) as u64
            } else {
                u64::MAX // Permission Denied
            }
        },
        51 => { // Sys_discover (key_ptr, key_len, val_ptr, val_len)
             let key_len = a2 as usize;
             let val_len = a4 as usize;
             if key_len > 64 || val_len > 64 { return u64::MAX; }
             
             if !crate::mm::user::validate_user_ptr(a1, key_len) { return u64::MAX; }
             if !crate::mm::user::validate_user_ptr(a3, val_len) { return u64::MAX; }

             let mut key_buf = [0u8; 64];
             let mut val_buf = [0u8; 64];
             
             if crate::mm::user::copy_from_user(a1 as *const u8, &mut key_buf[..key_len]).is_err() { return u64::MAX; }
             if crate::mm::user::copy_from_user(a3 as *const u8, &mut val_buf[..val_len]).is_err() { return u64::MAX; }

             if let (Ok(key), Ok(val)) = (core::str::from_utf8(&key_buf[..key_len]), core::str::from_utf8(&val_buf[..val_len])) {
                 let registry = crate::semantic::core::REGISTRY.lock();
                 let results = registry.discover(key, val);
                 
                 if let Some(obj) = results.first() {
                     // Found one! Grant READ capability to current task
                     let pid = crate::process::scheduler::get_current_pid();
                     let mut tasks = crate::process::scheduler::TASKS.lock();
                     if let Some(task) = &mut tasks[pid] {
                         use crate::object::{ObjectHandle, CapabilitySet};
                         let handle = ObjectHandle::new(obj.clone(), CapabilitySet::READ);
                         let idx = task.handle_table.lock().insert(handle);
                         return idx as u64;
                     }
                 }
             }
             u64::MAX // Not found
        }
        52 => { // Sys_snapshot (handle_idx)
            let handle_idx = a1 as usize;
            let pid = crate::process::scheduler::get_current_pid();
            
            let snapshot = {
                let tasks = crate::process::scheduler::TASKS.lock();
                if let Some(task) = &tasks[pid] {
                    let ht = task.handle_table.lock();
                    if let Some(obj) = ht.get(handle_idx, crate::object::CapabilitySet::READ) {
                        obj.snapshot().ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(snap) = snapshot {
                let snap_id = crate::object::chronos::VAULT.lock().save(snap);
                return snap_id as u64;
            }
            u64::MAX
        },
        53 => { // Sys_restore (handle_idx, snapshot_id)
            let handle_idx = a1 as usize;
            let snap_id = a2 as usize;
            let pid = crate::process::scheduler::get_current_pid();

            let result = {
                let tasks = crate::process::scheduler::TASKS.lock();
                if let Some(task) = &tasks[pid] {
                    let ht = task.handle_table.lock();
                    if let Some(obj) = ht.get(handle_idx, crate::object::CapabilitySet::WRITE) {
                        if let Some(snap) = crate::object::chronos::VAULT.lock().load(snap_id) {
                            obj.restore(snap).is_ok()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if result {
                0
            } else {
                1
            }
        },
         54 => { // Sys_hw_out8 (handle_idx, port, val)
              let handle_idx = a1 as usize;
              let port = a2 as u16;
              let val = a3 as u8;
              let pid = crate::process::scheduler::get_current_pid();

              let allowed = {
                  let tasks = crate::process::scheduler::TASKS.lock();
                  if let Some(task) = &tasks[pid] {
                      let ht = task.handle_table.lock();
                      ht.get(handle_idx, crate::object::CapabilitySet::GRANT).is_some()
                  } else {
                      false
                  }
              };

              if allowed {
                  unsafe {
                      core::arch::asm!("out dx, al", in("dx") port, in("al") val);
                  }
                  return 0;
              }
              1 // Access Denied
         },
         55 => { // Sys_hw_in8 (handle_idx, port)
              let handle_idx = a1 as usize;
              let port = a2 as u16;
              let pid = crate::process::scheduler::get_current_pid();

              let allowed = {
                  let tasks = crate::process::scheduler::TASKS.lock();
                  if let Some(task) = &tasks[pid] {
                      let ht = task.handle_table.lock();
                      ht.get(handle_idx, crate::object::CapabilitySet::READ).is_some()
                  } else {
                      false
                  }
              };

              if allowed {
                  let ret: u8;
                  unsafe {
                      core::arch::asm!("in al, dx", out("al") ret, in("dx") port);
                  }
                  return ret as u64;
              }
              u64::MAX // Access Denied
         },
        30 => { // Sys_query_metrics (pid, buf_ptr)
            // a1 = pid, a2 = ptr to TaskMetrics
            if !crate::mm::user::validate_user_ptr(a2, 64) { return u64::MAX; }
            
            let mut tasks = crate::process::scheduler::TASKS.lock();
            if let Some(task) = &tasks[a1 as usize] {
                // Construct a metrics array/packed data to copy back
                // For simplicity, let's just copy the fields directly if possible
                // or use a temporary buffer.
                let mut metrics = [0u64; 8];
                metrics[0] = task.id as u64;
                metrics[1] = task.cpu_time_ticks;
                metrics[2] = task.total_cycles;
                metrics[3] = task.page_count as u64;
                metrics[4] = task.syscall_count;
                metrics[5] = task.state as u64;
                metrics[6] = task.priority as u64;
                metrics[7] = task.signals as u64;
                
                drop(tasks);
                if crate::mm::user::copy_to_user(a2 as *mut u8, unsafe { core::slice::from_raw_parts(&metrics as *const _ as *const u8, 64) }).is_ok() {
                    0
                } else {
                    u64::MAX
                }
            } else {
                u64::MAX
            }
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
        11 => { // sys_pipe() -> (r << 32 | w)
             let (r, w) = crate::process::scheduler::process_pipe();
             if r == -1 { u64::MAX }
             else { ((r as u64) << 32) | (w as u64) }
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
        43 => sys_accept(a1),
        44 => sys_connect(a1, a2, a3),
        45 => sys_resolve(a1, a2, a3),
        58 => { // Sys_hw_discover (index, buf_ptr)
            let index = a1 as usize;
            let devices = crate::manager::discovery::get_all_devices();
            if index < devices.len() {
                 let dev = &devices[index];
                 // Copy name to user buffer (a2)
                 if crate::mm::user::copy_to_user(a2 as *mut u8, dev.name.as_bytes()).is_ok() {
                     return dev.name.len() as u64;
                 }
            }
            0
        }
        59 => { // Sys_get_system_log (buf_ptr, buf_len)
            let log_str = crate::manager::log::LOG.lock().read_all();
            let len = core::cmp::min(a2 as usize, log_str.len());
            if crate::mm::user::copy_to_user(a1 as *mut u8, &log_str.as_bytes()[..len]).is_ok() {
                return len as u64;
            }
            0
        }
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

fn sys_connect(fd: u64, ip_ptr: u64, port: u64) -> u64 {
    crate::net::syscall_connect(fd as usize, ip_ptr, port as u16) as u64
}

fn sys_resolve(host_ptr: u64, ip_ptr: u64, host_len: u64) -> u64 {
    // 1. Get hostname with explicit length
    let len = if host_len > 64 { 64 } else { host_len as usize };
    let mut buf = [0u8; 64];
    if crate::mm::user::copy_from_user(host_ptr as *const u8, &mut buf[..len]).is_err() { return 1; }
    
    let hostname = match core::str::from_utf8(&buf[..len]) {
        Ok(s) => s.trim(),
        Err(_) => return 1,
    };
    
    if let Some(ip) = crate::net::dns::resolve(hostname) {
        if crate::mm::user::copy_to_user(ip_ptr as *mut u8, ip.as_bytes()).is_ok() {
            return 0;
        }
    }
    1
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
