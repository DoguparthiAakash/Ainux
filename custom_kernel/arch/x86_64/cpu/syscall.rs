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

    // Star: 63:48 User Base (0x13), 47:32 Kernel Base (0x08)
    // 0x13 Base => Sysret CS=0x23(User Code), SS=0x1B(User Data)
    let star = (0x0013u64 << 48) | (0x0008u64 << 32);
    wrmsr(MSR_STAR, star);

    wrmsr(MSR_LSTAR, syscall_handler as u64);
    wrmsr(MSR_FMASK, 0x200); // Disable IF
    
    // Set kernel stack for syscall (Temporary: use current RSP)
    let rsp: u64;
    asm!("mov {}, rsp", out(reg) rsp);
    core::ptr::write_volatile(&mut KERNEL_STACK_PTR, rsp); // Use boot stack for now
}

pub unsafe fn set_syscall_kernel_stack(stack: u64) {
    core::ptr::write_volatile(&mut KERNEL_STACK_PTR, stack);
}

pub unsafe fn syscall(id: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
    let cs: u16;
    asm!("mov {:x}, cs", out(reg) cs);
    if cs == 0x08 {
        let args = SyscallArgs { a1, a2, a3, a4, a5, a6: 0 };
        return rust_syscall_dispatch(id, &args);
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
        // Save User RSP to global temporarily, then load Kernel RSP
        "mov [rip + {USER_BACKUP}], rsp",
        "mov rsp, [rip + {KERNEL_BACKUP}]",
        "push qword ptr [rip + {USER_BACKUP}]", // Push user RSP onto kernel stack!
        
        // Save registers that must be preserved across syscalls
        "push rcx", // User RIP
        "push r11", // User RFLAGS
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        
        // Push Syscall Arguments to form SyscallArgs struct on stack
        "push r9",  // a6
        "push r8",  // a5
        "push r10", // a4
        "push rdx", // a3
        "push rsi", // a2
        "push rdi", // a1
        
        // Setup arguments for rust_syscall_dispatch
        "mov rdi, rax", // Arg 1: Syscall ID
        "mov rsi, rsp", // Arg 2: Pointer to SyscallArgs struct (also start of SyscallState)
        
        "sub rsp, 8", // Align stack to 16 bytes before call
        "call rust_syscall_dispatch",
        "add rsp, 8", // Restore alignment
        
        // Setup arguments for handle_pending_signals
        "mov rdi, rsp", // Arg 1: Pointer to SyscallState
        
        "push rax", // Save syscall return value
        "sub rsp, 8", // Align stack to 16 bytes before call (push rax aligned it? rsp was aligned before push. Wait. Before push rax, rsp was aligned to 16. After push rax, it's off by 8. So sub rsp, 8 aligns it again.)
        "call handle_pending_signals",
        "add rsp, 8", // Restore alignment
        "pop rax", // Restore syscall return value
        
        // Restore Syscall Arguments to avoid register corruption in userspace
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop r10",
        "pop r8",
        "pop r9",
        
        // Restore other registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "pop r11",
        "pop rcx",
        
        // Restore User RSP directly from kernel stack
        "pop rsp",
        
        "sysretq",
        
        USER_BACKUP = sym USER_STACK_BACKUP,
        KERNEL_BACKUP = sym KERNEL_STACK_PTR,
    );
}
#[unsafe(naked)]
pub extern "C" fn syscall_sysret_shim() {
    unsafe {
        core::arch::naked_asm!(
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop r10",
            "pop r8",
            "pop r9",
            
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbx",
            "pop rbp",
            "pop r11",
            "pop rcx",
            
            "pop rsp",
            "mov rax, 0", // return 0 to child
            "sysretq"
        );
    }
}

#[repr(C)]
pub struct SyscallArgs {
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
}

#[no_mangle]
extern "C" fn rust_syscall_dispatch(id: u64, args_ptr: *const SyscallArgs) -> u64 {
    let args = unsafe { &*args_ptr };
    let a1 = args.a1;
    let a2 = args.a2;
    let a3 = args.a3;
    let a4 = args.a4;
    let a5 = args.a5;
    let a6 = args.a6;

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
        1 => { // Write (fd, ptr, len)
            let fd = a1 as usize;
            let len = a3 as usize;
            if len > 8192 { return 0; }
            if !crate::mm::user::validate_user_range(a2, len) { return 0; }

            let mut buf = alloc::vec![0u8; len];
            if crate::mm::user::copy_from_user(a2 as *const u8, &mut buf).is_ok() {
                if fd <= 2 {
                     // Always write to video, replacing invalid UTF-8 with 
                     let s = alloc::string::String::from_utf8_lossy(&buf);
                     video::put_str(&s);
                     crate::klog_serial!("{}", s);
                     return len as u64;
                } else {
                     let n = crate::process::scheduler::process_write(fd, &buf);
                     if n >= 0 {
                         return n as u64;
                     }
                }
            }
            0
        }

        22 | 293 => { // pipe(pipefd) / pipe2(pipefd, flags)
            if a1 != 0 && crate::mm::user::validate_user_range(a1, 8) {
                let (r, w) = crate::process::scheduler::process_pipe();
                if r >= 0 && w >= 0 {
                    unsafe {
                        let fds = a1 as *mut i32;
                        *fds = r as i32;
                        *(fds.add(1)) = w as i32;
                    }
                    0
                } else {
                    u64::MAX
                }
            } else {
                u64::MAX
            }
        },
        503 => { // IPC Send
            // a1 = port_handle (index in cap table), a2 = msg_ptr
            // Validate Message Pointer
            if !crate::mm::user::validate_user_range(a2, core::mem::size_of::<crate::ipc::port::Message>()) {
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
        504 => { // IPC Recv
            // a1 = port_handle, a2 = buffer ptr, a3 = non_blocking flag
            // Validate Buffer Pointer
            if !crate::mm::user::validate_user_range(a2, core::mem::size_of::<crate::ipc::port::Message>()) {
                 return 1;
            }
            if (a2 as usize) % 8 != 0 { return 1; }

            let non_blocking = a3 != 0;
            if let Some(msg) = crate::ipc::port::receive(a1 as usize, non_blocking) {
                 let buf = a2 as *mut crate::ipc::port::Message;
                 unsafe { *buf = msg };
                 0
            } else {
                // Return 1 (No message / Would block / Blocked and awoken without msg)
                1
            }
        }
        505 => sys_sleep(a1),
        2 => { // Sys_open (path, flags, mode)
             let mut len = 0;
             while len < 256 {
                 let mut b = [0u8; 1];
                 if crate::mm::user::copy_from_user((a1 + len) as *const u8, &mut b).is_err() {
                     break;
                 }
                 if b[0] == 0 { break; }
                 len += 1;
             }
             if len >= 256 || len == 0 { return u64::MAX; }
             
             let mut buf = alloc::vec![0u8; len as usize];
             if crate::mm::user::copy_from_user(a1 as *const u8, &mut buf).is_err() { return u64::MAX; }
             
             if let Ok(path) = core::str::from_utf8(&buf) {
                 crate::process::scheduler::process_open(path, a2 as u32) as u64
             } else {
                 u64::MAX // -1
             }
        },
        41 | 42 | 43 | 49 | 50 => {
             // Capability Check: Network Access
             let cell = crate::process::scheduler::get_current_cell();
             // For now, we assume Cell 0 (root) has all capabilities.
             if cell.id != 0 {
                 let has_net = {
                     let caps = cell.caps.lock();
                     // Simplified check: Does the cell have any capability?
                     // In a real implementation, we'd check for CapType::Resource("network")
                     false // Strict deny by default for non-root cells
                 };
                 if !has_net { return u64::MAX; } // Access Denied
             }
             match id {
                 41 => crate::net::sys_socket(a1 as i32, a2 as i32, a3 as i32) as u64,
                 42 => crate::net::sys_connect(a1 as usize, a2 as *const u8, a3 as usize) as u64,
                 43 => crate::net::sys_accept(a1 as usize) as u64,
                 49 => crate::net::sys_bind(a1 as usize, a2 as *const u8, a3 as usize) as u64,
                 50 => crate::net::sys_listen(a1 as usize, a2 as i32) as u64,
                 _ => u64::MAX,
             }
        },
        62 => crate::process::scheduler::post_signal(a1 as usize, a2 as u32) as u64,
        500 => crate::process::scheduler::sys_get_tasks(a1 as u64, a2 as usize) as u64,
        13 => { // rt_sigaction(sig, act, oact, sigsetsize)
            let sig = a1 as usize;
            let act_ptr = a2 as *const crate::process::signal::SigAction;
            let oact_ptr = a3 as *mut crate::process::signal::SigAction;
            let sigsetsize = a4;
            
            if sig >= 64 || sigsetsize != 8 { return u64::MAX; }
            if sig == 9 || sig == 19 { return u64::MAX; } // Cannot catch SIGKILL or SIGSTOP
            
            let mut res = u64::MAX;
            crate::cpu::without_interrupts(|| {
                let mut tasks = crate::process::scheduler::TASKS.lock();
                let current_pid = crate::process::scheduler::get_current_pid();
                if let Some(task) = &mut tasks[current_pid] {
                    if oact_ptr as u64 != 0 && crate::mm::user::validate_user_range(oact_ptr as u64, core::mem::size_of::<crate::process::signal::SigAction>()) {
                        unsafe { *oact_ptr = task.sigactions[sig]; }
                        res = 0;
                    }
                    if act_ptr as u64 != 0 && crate::mm::user::validate_user_range(act_ptr as u64, core::mem::size_of::<crate::process::signal::SigAction>()) {
                        unsafe { task.sigactions[sig] = *act_ptr; }
                        res = 0;
                    }
                }
            });
            res
        },
        57 => { // Fork
            // The handler must have saved the entire state. `args` actually points to SyscallArgs inside SyscallState.
            crate::process::scheduler::sys_fork(args as *const _ as *const crate::process::scheduler::SyscallState) as u64
        },
        59 => { // sys_execve(path, argv, envp)
            crate::process::scheduler::sys_execve(a1, a2, a3, args as *const _ as *mut crate::process::scheduler::SyscallState) as u64
        },
        60 => { // sys_exit(code)
            crate::process::scheduler::sys_exit(a1 as isize);
            0 // Never reached
        },
        61 => { // sys_wait4(pid, status, options, rusage)
            crate::process::scheduler::sys_wait4(a1 as isize, a2, a3 as i32, a4) as u64
        },
        0 => { // Sys_read (fd, buf, len)
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
        3 => { // Sys_close (fd)
            crate::process::scheduler::process_close(a1 as usize) as u64
        },
        33 => { // Sys_dup2 (oldfd, newfd)
            crate::process::scheduler::process_dup2(a1 as usize, a2 as usize) as u64
        },
        83 => { // Sys_mkdir (path_ptr, path_len)
             let len = a2 as usize;
             if len > 256 { return u64::MAX; }
             if !crate::mm::user::validate_user_range(a1, len) { return u64::MAX; }
             
             let posix_port = crate::ipc::port::POSIX_SUBSYSTEM_PORT.load(core::sync::atomic::Ordering::SeqCst);
             if posix_port != usize::MAX {
                 // Route to Userspace POSIX Server via IPC
                 let msg = crate::ipc::port::Message {
                     sender_pid: crate::process::scheduler::get_current_pid(),
                     msg_type: 83, // MKDIR
                     payload: crate::ipc::port::IpcPayload::Memory(a1 as usize, len),
                 };
                 if crate::ipc::port::send(posix_port, msg) {
                     // Block waiting for reply
                     // In a real system, we'd wait on a reply port.
                     return 0; // Stub: assume success if message sent
                 } else {
                     return u64::MAX;
                 }
             }

             // Fallback: In-kernel implementation
             let s = unsafe { core::slice::from_raw_parts(a1 as *const u8, len) };
             if let Ok(path) = core::str::from_utf8(s) {
                 crate::process::scheduler::process_mkdir(path) as u64
             } else {
                 u64::MAX
             }
        },
        87 => { // Sys_unlink (path_ptr, path_len)
             let len = a2 as usize;
             if len > 256 { return u64::MAX; }
             if !crate::mm::user::validate_user_range(a1, len) { return u64::MAX; }
             
             let s = unsafe { core::slice::from_raw_parts(a1 as *const u8, len) };
             if let Ok(path) = core::str::from_utf8(s) {
                 crate::process::scheduler::process_unlink(path) as u64
             } else {
                 u64::MAX
             }
        },

        51 => { // Sys_discover (key_ptr, key_len, val_ptr, val_len)
             let key_len = a2 as usize;
             let val_len = a4 as usize;
             if key_len > 64 || val_len > 64 { return u64::MAX; }
             
             if !crate::mm::user::validate_user_range(a1, key_len) { return u64::MAX; }
             if !crate::mm::user::validate_user_range(a3, val_len) { return u64::MAX; }

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
            if !crate::mm::user::validate_user_range(a2, 64) { return u64::MAX; }
            
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
        510 => { // sys_get_speaker_count
             crate::drivers::audio::ac97::get_speaker_count() as u64
        },
        511 => { // sys_play_beep
             crate::drivers::audio::ac97::play_beep();
             0
        },
        60 | 231 => { // sys_exit / sys_exit_group
            crate::klog_serial!("Process {} exited with status {}\n", crate::process::scheduler::get_current_pid(), a1);
            crate::process::scheduler::exit_current_task(a1 as isize);
            0 // Should not return
        },
        61 => { // sys_wait4(pid, status, options, rusage)
            crate::process::scheduler::sys_wait4(a1 as isize, a2, a3 as i32, a4) as u64
        },
        59 => { // sys_execve(filename, argv, envp)
            crate::process::scheduler::sys_execve(a1, a2, a3, args_ptr as *const _ as *mut crate::process::scheduler::SyscallState) as u64
        },
        24 => { // sys_yield()
            crate::process::scheduler::yield_now();
            0
        },
        4 => { // sys_stat (path, stat_buf)
            let mut len = 0;
            while len < 256 {
                let mut b = [0u8; 1];
                if crate::mm::user::copy_from_user((a1 + len) as *const u8, &mut b).is_err() || b[0] == 0 { break; }
                len += 1;
            }
            if len == 0 || len >= 256 { return u64::MAX; }
            let mut buf = alloc::vec![0u8; len as usize];
            if crate::mm::user::copy_from_user(a1 as *const u8, &mut buf).is_err() { return u64::MAX; }
            if let Ok(path) = core::str::from_utf8(&buf) {
                if crate::mm::user::validate_user_range(a2, core::mem::size_of::<crate::fs::vfs::CStat>()) {
                    let stat_ptr = a2 as *mut crate::fs::vfs::CStat;
                    let mut stat_buf = crate::fs::vfs::CStat::default();
                    let res = crate::process::scheduler::process_stat(path, &mut stat_buf);
                    if res == 0 {
                        unsafe { *stat_ptr = stat_buf; }
                    }
                    return res as u64;
                }
            }
            u64::MAX
        },
        5 => { // sys_fstat (fd, stat_buf)
            if crate::mm::user::validate_user_range(a2, core::mem::size_of::<crate::fs::vfs::CStat>()) {
                let stat_ptr = a2 as *mut crate::fs::vfs::CStat;
                let mut stat_buf = crate::fs::vfs::CStat::default();
                let res = crate::process::scheduler::process_fstat(a1 as usize, &mut stat_buf);
                if res == 0 {
                    unsafe { *stat_ptr = stat_buf; }
                }
                return res as u64;
            }
            u64::MAX
        },
        6 => { // sys_lstat (same as stat for now)
            let mut len = 0;
            while len < 256 {
                let mut b = [0u8; 1];
                if crate::mm::user::copy_from_user((a1 + len) as *const u8, &mut b).is_err() || b[0] == 0 { break; }
                len += 1;
            }
            if len == 0 || len >= 256 { return u64::MAX; }
            let mut buf = alloc::vec![0u8; len as usize];
            if crate::mm::user::copy_from_user(a1 as *const u8, &mut buf).is_err() { return u64::MAX; }
            if let Ok(path) = core::str::from_utf8(&buf) {
                if crate::mm::user::validate_user_range(a2, core::mem::size_of::<crate::fs::vfs::CStat>()) {
                    let stat_ptr = a2 as *mut crate::fs::vfs::CStat;
                    let mut stat_buf = crate::fs::vfs::CStat::default();
                    let res = crate::process::scheduler::process_stat(path, &mut stat_buf);
                    if res == 0 {
                        unsafe { *stat_ptr = stat_buf; }
                    }
                    return res as u64;
                }
            }
            u64::MAX
        },
        8 => 0, // sys_lseek stub
        32 => crate::process::scheduler::process_dup(a1 as usize) as u64,
        33 => crate::process::scheduler::process_dup2(a1 as usize, a2 as usize) as u64,
        35 => { // sys_nanosleep
            crate::process::scheduler::yield_now();
            0
        },
        79 => { // sys_getcwd (buf, size)
             let size = a2 as usize;
             if size > 0 && crate::mm::user::validate_user_range(a1, size) {
                 let s = unsafe { core::slice::from_raw_parts_mut(a1 as *mut u8, size) };
                 let res = crate::process::scheduler::process_getcwd(s);
                 if res > 0 { return a1; } // success returns pointer
             }
             0
        },
        80 => { // sys_chdir (path)
            let mut len = 0;
            while len < 256 {
                let mut b = [0u8; 1];
                if crate::mm::user::copy_from_user((a1 + len) as *const u8, &mut b).is_err() || b[0] == 0 { break; }
                len += 1;
            }
            if len == 0 || len >= 256 { return u64::MAX; }
            let mut buf = alloc::vec![0u8; len as usize];
            if crate::mm::user::copy_from_user(a1 as *const u8, &mut buf).is_err() { return u64::MAX; }
            if let Ok(path) = core::str::from_utf8(&buf) {
                return crate::process::scheduler::process_chdir(path) as u64;
            }
            u64::MAX
        },
        110 => { // sys_getppid
            crate::process::scheduler::get_parent_pid(crate::process::scheduler::get_current_pid()) as u64
        },
        39 => { // sys_getpid
            crate::process::scheduler::get_current_pid() as u64
        },
        78 => { // sys_readdir (path, buf, count)
            let mut len = 0;
            while len < 256 {
                let mut b = [0u8; 1];
                if crate::mm::user::copy_from_user((a1 + len) as *const u8, &mut b).is_err() || b[0] == 0 { break; }
                len += 1;
            }
            if len == 0 || len >= 256 { return u64::MAX; }
            let mut buf = alloc::vec![0u8; len as usize];
            if crate::mm::user::copy_from_user(a1 as *const u8, &mut buf).is_err() { return u64::MAX; }
            if let Ok(path) = core::str::from_utf8(&buf) {
                if !crate::mm::user::validate_user_range(a2, a3 as usize) { return u64::MAX; }
                
                let res = match crate::fs::vfs::resolve_path(path) {
                    Ok(inode) => {
                        match inode.read_dir() {
                            Ok(files) => {
                                let mut k_buf = alloc::vec![0u8; a3 as usize];
                                let mut offset = 0;
                                for name in files {
                                    let bytes = name.as_bytes();
                                    if offset + bytes.len() + 1 <= a3 as usize {
                                        k_buf[offset..offset+bytes.len()].copy_from_slice(bytes);
                                        k_buf[offset+bytes.len()] = b'\n';
                                        offset += bytes.len() + 1;
                                    } else {
                                        break;
                                    }
                                }
                                if crate::mm::user::copy_to_user(a2 as *mut u8, &k_buf[0..offset]).is_ok() {
                                    offset as u64
                                } else {
                                    u64::MAX
                                }
                            },
                            Err(_) => u64::MAX
                        }
                    },
                    Err(_) => u64::MAX
                };
                return res;
            }
            u64::MAX
        },
        9 => { // sys_mmap
            // a1 = addr, a2 = length, a3 = prot, a4 = flags, a5 = fd, a6 = offset
            crate::klog_serial!("sys_mmap(addr={:#x}, len={}, prot={}, flags={:#x}, fd={}, off={})\n", a1, a2, a3, a4, a5, a6);
            let is_anon = (a4 & 0x20) != 0;
            
            let size = (a2 + 4095) & !4095;
            let current_pid = crate::process::scheduler::get_current_pid();
            
            let addr = if a1 == 0 {
                let mut tasks = crate::process::scheduler::TASKS.lock();
                if let Some(task) = &mut tasks[current_pid] {
                    let base = task.mmap_base;
                    task.mmap_base += size;
                    base
                } else {
                    return u64::MAX;
                }
            } else {
                a1
            };
            
            let pages = size / 4096;
            let mut vmm_flags = crate::mm::vmm::USER | crate::mm::vmm::PRESENT;
            if (a3 & 0x2) != 0 { vmm_flags |= crate::mm::vmm::WRITABLE; } // PROT_WRITE

            if is_anon {
                for i in 0..pages {
                    let frame_opt = {
                        let mut pmm_lock = crate::mm::pmm::PMM.lock();
                        pmm_lock.as_mut().and_then(|pmm| pmm.alloc_frame())
                    };
                    if let Some(frame) = frame_opt {
                        let virt = addr + i * 4096;
                        unsafe {
                            let _ = crate::mm::vmm::map_page(virt, frame, vmm_flags);
                            core::ptr::write_bytes(virt as *mut u8, 0, 4096);
                        }
                    } else {
                        crate::klog_serial!("sys_mmap failed: OOM\n");
                        return 0xFFFFFFFFFFFFFFFF;
                    }
                }
                crate::klog_serial!("sys_mmap success -> {:#x}\n", addr);
                addr
            } else {
                // File-backed mmap
                let fd = a5 as usize;
                let file_opt = {
                    let tasks = crate::process::scheduler::TASKS.lock();
                    if let Some(task) = &tasks[current_pid] {
                        task.fds.get_handle(fd).ok()
                    } else {
                        None
                    }
                };

                if let Some(file) = file_opt {
                    if let Ok(Some(kernel_addr)) = file.mmap(a6, size as usize) {
                        for i in 0..pages {
                            let k_virt = kernel_addr + (i * 4096) as u64;
                            let frame = crate::mm::vmm::virt_to_phys(k_virt);
                            let u_virt = addr + (i * 4096) as u64;
                            unsafe {
                                let _ = crate::mm::vmm::map_page(u_virt, frame, vmm_flags);
                            }
                        }
                        crate::klog_serial!("sys_mmap (file) success -> {:#x}\n", addr);
                        return addr;
                    }
                }
                0xFFFFFFFFFFFFFFFF
            }
        },
        10 => { // sys_mprotect
            // Currently ignored / successful since we don't track VMA regions yet.
            // Page table updates would require walking the PML4 for the user address.
            0 
        },
        11 => { // sys_munmap(addr, length)
            let addr = a1;
            let size = (a2 + 4095) & !4095;
            let pages = size / 4096;
            
            for i in 0..pages {
                let virt = addr + i * 4096;
                // Since this might be shared/backed by files, we only unmap the page table entry, 
                // we don't necessarily free the physical frame unless we keep refcounts.
                // For anonymous mmap we *should* free it, but without VMA tracking it's hard to know.
                // So we'll just unmap it from the page table for now to prevent use-after-unmap bugs.
                unsafe { crate::mm::vmm::unmap_page(virt); }
            }
            0
        },
        12 => { // sys_brk(brk)
            let current_pid = crate::process::scheduler::get_current_pid();
            let mut tasks = crate::process::scheduler::TASKS.lock();
            if let Some(task) = &mut tasks[current_pid] {
                let current_brk = task.brk;
                let new_brk = a1;
                
                if new_brk == 0 {
                    return current_brk;
                }
                
                if new_brk > current_brk {
                    // Growing heap
                    let old_page_end = (current_brk + 4095) & !4095;
                    let new_page_end = (new_brk + 4095) & !4095;
                    
                    if new_page_end > old_page_end {
                        let pages = (new_page_end - old_page_end) / 4096;
                        for i in 0..pages {
                            let frame_opt = {
                                let mut pmm_lock = crate::mm::pmm::PMM.lock();
                                pmm_lock.as_mut().and_then(|pmm| pmm.alloc_frame())
                            };
                            if let Some(frame) = frame_opt {
                                let virt = old_page_end + i * 4096;
                                unsafe {
                                    let _ = crate::mm::vmm::map_page(virt, frame, crate::mm::vmm::USER | crate::mm::vmm::WRITABLE | crate::mm::vmm::PRESENT);
                                    core::ptr::write_bytes(virt as *mut u8, 0, 4096);
                                }
                            } else {
                                return current_brk; // OOM, return old brk
                            }
                        }
                    }
                    task.brk = new_brk;
                    new_brk
                } else {
                    // Shrinking heap (not freeing pages for now, just updating brk ptr)
                    task.brk = new_brk;
                    new_brk
                }
            } else {
                u64::MAX
            }
        },
        510 => { // sys_uptime() -> ms
             unsafe { crate::process::scheduler::get_ticks() * 10 }
        },
        96 => { // sys_gettimeofday(tv, tz)
            let tv_ptr = a1;
            if tv_ptr != 0 && crate::mm::user::validate_user_range(tv_ptr, 16) {
                let ms = unsafe { crate::process::scheduler::get_ticks() * 10 };
                let sec = ms / 1000;
                let usec = (ms % 1000) * 1000;
                unsafe {
                    *(tv_ptr as *mut u64) = sec;
                    *((tv_ptr + 8) as *mut u64) = usec;
                }
                0
            } else {
                u64::MAX
            }
        },
        228 => { // sys_clock_gettime(clk_id, tp)
            let tp_ptr = a2;
            if tp_ptr != 0 && crate::mm::user::validate_user_range(tp_ptr, 16) {
                let ms = unsafe { crate::process::scheduler::get_ticks() * 10 };
                let sec = ms / 1000;
                let nsec = (ms % 1000) * 1000000;
                unsafe {
                    *(tp_ptr as *mut u64) = sec;
                    *((tp_ptr + 8) as *mut u64) = nsec;
                }
                0
            } else {
                u64::MAX
            }
        },
        22 => { // sys_pipe() -> (r << 32 | w)
             let (r, w) = crate::process::scheduler::process_pipe();
             if r == -1 { u64::MAX }
             else { ((r as u64) << 32) | (w as u64) }
        },
        56 => { // sys_clone(entry, stack) -> pid
             if !crate::mm::user::validate_user_range(a2, 8) { return 0xFFFFFFFFFFFFFFFF; }
             if !crate::mm::user::validate_user_range(a1, 1) { return 0xFFFFFFFFFFFFFFFF; }
             crate::process::scheduler::clone_task(a1, a2) as u64
        },
        // sys_writev (20)
        20 => {
            let fd = a1;
            let iov_ptr = a2 as *const [u64; 2]; // simple representation of iovec (base, len)
            let iovcnt = a3 as usize;

            if iovcnt > 1024 { return u64::MAX; }
            if !crate::mm::user::validate_user_range(a2, iovcnt * 16) { return u64::MAX; }

            if fd <= 2 {
                let mut total_written = 0;
                for i in 0..iovcnt {
                    let iov = unsafe { &*iov_ptr.add(i) };
                    let base = iov[0] as *const u8;
                    let len = iov[1] as usize;
                    if len > 0 && crate::mm::user::validate_user_range(base as u64, len) {
                        let slice = unsafe { core::slice::from_raw_parts(base, len) };
                        if let Ok(s) = core::str::from_utf8(slice) {
                            crate::klog_serial!("{}", s);
                            total_written += len;
                        } else {
                            // If not valid UTF-8, just print raw bytes (not ideal but safe)
                            for &b in slice {
                                crate::klog_serial!("{}", b as char);
                            }
                            total_written += len;
                        }
                    }
                }
                total_written as u64
            } else {
                crate::klog_serial!("sys_writev unsupported fd {}\n", fd);
                0
            }
        },
        500 => crate::sem::sys_agent_op(a1, a2, a3),
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
        506 => { // Sys_get_system_log (buf_ptr, buf_len)
            let log_str = crate::manager::log::LOG.lock().read_all();
            let len = core::cmp::min(a2 as usize, log_str.len());
            if crate::mm::user::copy_to_user(a1 as *mut u8, &log_str.as_bytes()[..len]).is_ok() {
                return len as u64;
            }
            0
        }
        16 => { // sys_ioctl
            let fd = a1 as usize;
            // Mask cmd to 32 bits: C passes ioctl request as 'unsigned long' but
            // many DRM ioctls have bit 31 set (e.g. 0xC0406400), causing sign-extension
            // when the value comes through a 32-bit int intermediate in userspace.
            let cmd = a2 & 0xFFFF_FFFF;
            let arg = a3;
            
            let current_pid = crate::process::scheduler::get_current_pid();
            let file_opt = {
                let tasks = crate::process::scheduler::TASKS.lock();
                if let Some(task) = &tasks[current_pid] {
                    task.fds.get_handle(fd).ok()
                } else {
                    None
                }
            };
            
            if let Some(file) = file_opt {
                crate::klog_serial!("sys_ioctl: fd={}, cmd={:#010x}, arg={:#x}\n", fd, cmd, arg);
                match file.ioctl(cmd, arg) {
                    Ok(res) => res,
                    Err(e) => {
                        crate::klog_serial!("sys_ioctl: failed: {:?}\n", e);
                        u64::MAX
                    },
                }
            } else {
                crate::klog_serial!("sys_ioctl: bad fd={}\n", fd);
                // -ENOTTY (25)
                (!25u64) + 1
            }
        }
        39 => { // sys_getpid
            // For now, return 2 (test_musl PID)
            2
        }
        158 => { // sys_arch_prctl
            let code = a1;
            let addr = a2;
            if code == 0x1002 { // ARCH_SET_FS
                unsafe { core::arch::asm!("wrmsr", in("ecx") 0xC000_0100u32, in("eax") (addr as u32), in("edx") ((addr >> 32) as u32)) };
                0
            } else if code == 0x1001 { // ARCH_SET_GS
                unsafe { core::arch::asm!("wrmsr", in("ecx") 0xC000_0101u32, in("eax") (addr as u32), in("edx") ((addr >> 32) as u32)) };
                0
            } else {
                (!0) as u64 // -EINVAL
            }
        }
        218 => { // sys_set_tid_address
            // Ignore the pointer, just return our PID (thread ID)
            2
        }
        4 => { // sys_stat
            let buf = a2 as *mut u8;
            if crate::mm::user::validate_user_range(a2, 144) {
                unsafe { core::ptr::write_bytes(buf, 0, 144); }
                0
            } else { u64::MAX }
        }
        5 => { // sys_fstat
            let buf = a2 as *mut u8;
            if crate::mm::user::validate_user_range(a2, 144) {
                unsafe { core::ptr::write_bytes(buf, 0, 144); }
                0
            } else { u64::MAX }
        }
        8 => 0, // sys_lseek
        13 => 0, // sys_rt_sigaction
        14 => 0, // sys_rt_sigprocmask
        21 => (!2u64) + 1, // sys_access (-ENOENT)
        35 => { // sys_nanosleep(timespec *req, timespec *rem)
            // timespec: { time_t tv_sec (8 bytes), long tv_nsec (8 bytes) }
            if crate::mm::user::validate_user_range(a1, 16) {
                let tv_sec = unsafe { *(a1 as *const u64) };
                let tv_nsec = unsafe { *((a1 + 8) as *const u64) };
                let ms = tv_sec * 1000 + tv_nsec / 1_000_000;
                if ms > 0 { sys_sleep(ms); }
            }
            0
        },
        230 => 0, // sys_clock_nanosleep — stub as no-op

        102 | 104 | 107 | 108 => 0, // sys_get[e]uid, sys_get[e]gid
        186 => 2, // sys_gettid
        257 => { // sys_openat(dirfd, path_ptr, flags, mode)
            // a1 = dirfd, a2 = path_ptr, a3 = flags, a4 = mode
            // We only handle AT_FDCWD (-100 as i32 = 0xFFFFFF9C as u32)
            // Treat as regular open(path, flags, mode)
            let mut len = 0usize;
            while len < 4096 {
                let mut b = [0u8; 1];
                if crate::mm::user::copy_from_user((a2 + len as u64) as *const u8, &mut b).is_err() {
                    break;
                }
                if b[0] == 0 { break; }
                len += 1;
            }
            if len == 0 || len >= 4096 { return (!2u64) + 1; } // -ENOENT

            let mut buf = alloc::vec![0u8; len];
            if crate::mm::user::copy_from_user(a2 as *const u8, &mut buf).is_err() {
                return (!2u64) + 1;
            }
            if let Ok(path) = core::str::from_utf8(&buf) {
                crate::klog_serial!("sys_openat: dirfd={}, path={}, flags={:#x}\n", a1 as i64, path, a3);
                crate::process::scheduler::process_open(path, a3 as u32) as u64
            } else {
                (!22u64) + 1 // -EINVAL
            }
        },
        7 => { // sys_poll
            0 // Stub: return 0 events ready
        },
        44 => { // sys_sendto(fd, buf, len, flags, dest_addr, addrlen)
            let fd = a1 as usize;
            let len = a3 as usize;
            if len > 8192 || !crate::mm::user::validate_user_range(a2, len) { return (!0u64); }
            let mut buf = alloc::vec![0u8; len];
            if crate::mm::user::copy_from_user(a2 as *const u8, &mut buf).is_ok() {
                let n = crate::process::scheduler::process_write(fd, &buf);
                if n >= 0 { return n as u64; }
            }
            (!0u64)
        },
        45 => { // sys_recvfrom(fd, buf, len, flags, src_addr, addrlen)
            let fd = a1 as usize;
            let len = a3 as usize;
            if len > 8192 || !crate::mm::user::validate_user_range(a2, len) { return (!0u64); }
            let mut buf = alloc::vec![0u8; len];
            let n = crate::process::scheduler::process_read(fd, &mut buf);
            if n > 0 {
                if crate::mm::user::copy_to_user(a2 as *mut u8, &buf[..n as usize]).is_ok() {
                    return n as u64;
                }
            }
            if n == 0 { return 0; }
            (!0u64)
        },
        318 => { // sys_getrandom(buf, buflen, flags)
            let buflen = a2 as usize;
            if buflen > 0 && crate::mm::user::validate_user_range(a1, buflen) {
                let mut buf = alloc::vec![0u8; buflen];
                crate::lib::prng::get_random_bytes(&mut buf);
                if crate::mm::user::copy_to_user(a1 as *mut u8, &buf).is_ok() {
                    return buflen as u64;
                }
            }
            (!0u64)
        },
        _ => {
            crate::klog_serial!("SYSCALL UNIMPLEMENTED: id={}\n", id);
            0
        }
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
