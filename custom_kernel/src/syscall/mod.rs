use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

pub mod fs;
pub mod mm;
pub mod process;

/// System call numbers based on Linux x86_64
pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_STAT: usize = 4;
pub const SYS_FSTAT: usize = 5;
pub const SYS_LSEEK: usize = 8;
pub const SYS_MMAP: usize = 9;
pub const SYS_MPROTECT: usize = 10;
pub const SYS_MUNMAP: usize = 11;
pub const SYS_BRK: usize = 12;
pub const SYS_RT_SIGACTION: usize = 13;
pub const SYS_RT_SIGPROCMASK: usize = 14;
pub const SYS_IOCTL: usize = 16;
pub const SYS_ACCESS: usize = 21;
pub const SYS_READV: usize = 19;
pub const SYS_WRITEV: usize = 20;
pub const SYS_GETPID: usize = 39;
pub const SYS_CLONE: usize = 56;
pub const SYS_FORK: usize = 57;
pub const SYS_VFORK: usize = 58;
pub const SYS_EXECVE: usize = 59;
pub const SYS_EXIT: usize = 60;
pub const SYS_WAIT4: usize = 61;
pub const SYS_KILL: usize = 62;
pub const SYS_PIPE: usize = 22;
pub const SYS_DUP: usize = 32;
pub const SYS_DUP2: usize = 33;
pub const SYS_KILL: usize = 62;
pub const SYS_UNAME: usize = 63;
pub const SYS_FCNTL: usize = 72;
pub const SYS_RENAME: usize = 82;
pub const SYS_MKDIR: usize = 83;
pub const SYS_RMDIR: usize = 84;
pub const SYS_LINK: usize = 86;
pub const SYS_UNLINK: usize = 87;
pub const SYS_CHMOD: usize = 90;
pub const SYS_CHOWN: usize = 92;
pub const SYS_SYSINFO: usize = 99;
pub const SYS_GETUID: usize = 102;
pub const SYS_GETGID: usize = 104;
pub const SYS_SETUID: usize = 105;
pub const SYS_SETGID: usize = 106;
pub const SYS_GETEUID: usize = 107;
pub const SYS_GETEGID: usize = 108;
pub const SYS_SETREUID: usize = 113;
pub const SYS_SETREGID: usize = 114;
pub const SYS_ARCH_PRCTL: usize = 158;
pub const SYS_GETTID: usize = 186;
pub const SYS_SET_TID_ADDRESS: usize = 218;
pub const SYS_EXIT_GROUP: usize = 231;
pub const SYS_OPENAT: usize = 257;
pub const SYS_SET_ROBUST_LIST: usize = 273;
pub const SYS_PRLIMIT64: usize = 302;
pub const SYS_GETRANDOM: usize = 318;
pub const SYS_GET_TASKS: usize = 500;
pub const SYS_DISK_READ: usize = 505;
pub const SYS_DISK_WRITE: usize = 506;
pub const SYS_DISK_IDENTIFY: usize = 507;

// Need to link the assembly entry point

extern "C" {
    fn syscall_entry();
}

pub fn init() {
    unsafe {
        // IA32_EFER MSR = 0xC0000080
        // Set SCE bit (bit 0) to enable SYSCALL/SYSRET
        let mut efer_low: u32;
        let mut efer_high: u32;
        core::arch::asm!("rdmsr", in("ecx") 0xC0000080_u32, out("eax") efer_low, out("edx") efer_high);
        efer_low |= 1; // SCE
        core::arch::asm!("wrmsr", in("ecx") 0xC0000080_u32, in("eax") efer_low, in("edx") efer_high);

        // IA32_STAR MSR = 0xC0000081
        // SYSRET: CS = STAR[63:48]+16, SS = STAR[63:48]+8 (both |= 3 for RPL)
        // SYSCALL: CS = STAR[47:32], SS = STAR[47:32]+8
        // GDT: 0x08=KCode, 0x10=KData, 0x18=UData, 0x20=UCode
        // SYSRET with 0x10: CS=0x10+16=0x20|3=0x23(UCode), SS=0x10+8=0x18|3=0x1B(UData) ✓
        // SYSCALL with 0x08: CS=0x08(KCode), SS=0x08+8=0x10(KData) ✓
        let star_high: u32 = (0x10 << 16) | 0x08;
        let star_low: u32 = 0;
        core::arch::asm!("wrmsr", in("ecx") 0xC0000081_u32, in("eax") star_low, in("edx") star_high);

        // IA32_LSTAR MSR = 0xC0000082
        // Contains the RIP for SYSCALL
        let lstar = syscall_entry as usize as u64;
        core::arch::asm!("wrmsr", in("ecx") 0xC0000082_u32, in("eax") (lstar & 0xFFFFFFFF) as u32, in("edx") (lstar >> 32) as u32);

        // IA32_FMASK MSR = 0xC0000084
        // Mask for RFLAGS when entering syscall. Mask IF (bit 9) to disable interrupts on entry.
        let fmask: u32 = 0x200; // Interrupt Flag
        core::arch::asm!("wrmsr", in("ecx") 0xC0000084_u32, in("eax") fmask, in("edx") 0);
    }
}

/// Main entry point from assembly for system calls.
#[no_mangle]
pub extern "C" fn syscall_handler(sys_no: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize, arg6: usize) -> isize {
    match sys_no {
        SYS_READ => fs::sys_read(arg1, arg2 as *mut u8, arg3),
        SYS_WRITE => fs::sys_write(arg1, arg2 as *const u8, arg3),
        SYS_OPEN => fs::sys_open(arg1 as *const u8, arg2 as i32, arg3 as i32),
        SYS_CLOSE => fs::sys_close(arg1),
        SYS_FSTAT => fs::sys_fstat(arg1, arg2 as *mut crate::fs::vfs::CStat),
        SYS_STAT => fs::sys_stat(arg1 as *const u8, arg2 as *mut crate::fs::vfs::CStat),
        SYS_LSEEK => fs::sys_lseek(arg1, arg2 as isize, arg3 as i32),
        SYS_MMAP => mm::sys_mmap(arg1, arg2, arg3 as i32, arg4 as i32, arg5, arg6),
        SYS_MPROTECT => 0, // Stub: always succeed
        SYS_MUNMAP => mm::sys_munmap(arg1, arg2),
        SYS_BRK => mm::sys_brk(arg1),
        SYS_RT_SIGACTION => 0,     // Stub: succeed silently
        SYS_RT_SIGPROCMASK => 0,   // Stub: succeed silently
        SYS_IOCTL => fs::sys_ioctl(arg1, arg2, arg3),
        SYS_ACCESS => -2,          // ENOENT — file not found (stub)
        SYS_READV => fs::sys_readv(arg1, arg2 as *const u8, arg3),
        SYS_WRITEV => fs::sys_writev(arg1, arg2 as *const u8, arg3),
        SYS_RENAME => fs::sys_rename(arg1 as *const u8, arg2 as *const u8),
        SYS_MKDIR => fs::sys_mkdir(arg1 as *const u8, arg2 as u16),
        SYS_RMDIR => fs::sys_rmdir(arg1 as *const u8),
        SYS_LINK => fs::sys_link(arg1 as *const u8, arg2 as *const u8),
        SYS_UNLINK => fs::sys_unlink(arg1 as *const u8),
        SYS_CHMOD => fs::sys_chmod(arg1 as *const u8, arg2 as u16),
        SYS_CHOWN => fs::sys_chown(arg1 as *const u8, arg2 as u16, arg3 as u16),
        SYS_SYSINFO => {
            let buf = arg1 as *mut u8;
            if !buf.is_null() {
                // Struct matching Linux sysinfo exactly
                #[repr(C)]
                struct Sysinfo {
                    uptime: i64,
                    loads: [u64; 3],
                    totalram: u64,
                    freeram: u64,
                    sharedram: u64,
                    bufferram: u64,
                    totalswap: u64,
                    freeswap: u64,
                    procs: u16,
                    pad: u16,
                    totalhigh: u64,
                    freehigh: u64,
                    mem_unit: u32,
                }
                let ticks = crate::process::scheduler::get_ticks();
                let (used, total) = crate::mm::pmm::PMM.lock().as_ref().unwrap().get_stats_fast();
                let mut procs = 0;
                {
                    let tasks = crate::process::scheduler::TASKS.lock();
                    for i in 0..crate::process::scheduler::MAX_TASKS {
                        if let Some(t) = &tasks[i] {
                            if t.state != crate::process::task::TaskState::Free {
                                procs += 1;
                            }
                        }
                    }
                }
                let info = Sysinfo {
                    uptime: (ticks / 100) as i64, // Assume 100Hz = 10ms per tick
                    loads: [0, 0, 0],
                    totalram: (total * 4096) as u64,
                    freeram: ((total - used) * 4096) as u64,
                    sharedram: 0,
                    bufferram: 0,
                    totalswap: 0,
                    freeswap: 0,
                    procs,
                    pad: 0,
                    totalhigh: 0,
                    freehigh: 0,
                    mem_unit: 1, // Bytes
                };
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        &info as *const _ as *const u8,
                        buf,
                        core::mem::size_of::<Sysinfo>(),
                    );
                }
                0
            } else {
                -1 // EFAULT
            }
        },
        SYS_OPENAT => fs::sys_openat(arg1 as i32, arg2 as *const u8, arg3 as i32, arg4 as i32),
        SYS_GET_TASKS => {
            let buf = arg1 as *mut u8;
            let len = arg2;
            if buf.is_null() || len == 0 { return -1; }
            let mut writer = crate::lib::string_writer::StringWriter::new(unsafe { core::slice::from_raw_parts_mut(buf, len) });
            use core::fmt::Write;
            let _ = write!(writer, "PID    STATE      NAME\n");
            let tasks = crate::process::scheduler::TASKS.lock();
            for i in 0..crate::process::scheduler::MAX_TASKS {
                if let Some(t) = &tasks[i] {
                    if t.state != crate::process::task::TaskState::Free {
                        let state_str = match t.state {
                            crate::process::task::TaskState::Running => "R",
                            crate::process::task::TaskState::Ready => "S",
                            crate::process::task::TaskState::Waiting => "W",
                            crate::process::task::TaskState::Zombie => "Z",
                            _ => "?",
                        };
                        let _ = write!(writer, "{:<6} {:<10} {}\n", t.id, state_str, t.name);
                    }
                }
            }
            writer.len() as isize
        },
        SYS_DISK_READ => fs::sys_disk_read(arg1 as u32, arg2 as u8, arg3 as *mut u8),
        SYS_DISK_WRITE => fs::sys_disk_write(arg1 as u32, arg2 as u8, arg3 as *const u8),
        SYS_DISK_IDENTIFY => fs::sys_disk_identify(arg1 as *mut u8),
        SYS_GETPID => 1,           // PID 1 (init)
        SYS_FORK => process::sys_fork(),
        SYS_EXECVE => process::sys_execve(arg1 as *const u8, arg2 as *const *const u8, arg3 as *const *const u8),
        SYS_WAIT4 => process::sys_wait4(arg1 as i32, arg2 as *mut i32, arg3 as i32, arg4 as *mut u8),
        SYS_PIPE => fs::sys_pipe(arg1 as *mut i32),
        SYS_DUP => fs::sys_dup(arg1),
        SYS_DUP2 => fs::sys_dup2(arg1, arg2),

        SYS_GETTID => crate::process::scheduler::sys_gettid(),
        SYS_GETUID => crate::process::scheduler::sys_getuid(),
        SYS_GETEUID => crate::process::scheduler::sys_geteuid(),
        SYS_GETGID => crate::process::scheduler::sys_getgid(),
        SYS_GETEGID => crate::process::scheduler::sys_getegid(),
        SYS_SETUID => crate::process::scheduler::sys_setuid(arg1 as u32),
        SYS_SETGID => crate::process::scheduler::sys_setgid(arg1 as u32),
        SYS_SETREUID => crate::process::scheduler::sys_setreuid(arg1 as u32, arg2 as u32),
        SYS_SETREGID => crate::process::scheduler::sys_setregid(arg1 as u32, arg2 as u32),
        SYS_UNAME => {
            // Fill in a utsname struct at arg1
            let buf = arg1 as *mut u8;
            if !buf.is_null() {
                unsafe {
                    core::ptr::write_bytes(buf, 0, 390); // sizeof(utsname) = 65*6
                    // sysname
                    let sysname = b"Ainux\0";
                    core::ptr::copy_nonoverlapping(sysname.as_ptr(), buf, sysname.len());
                    // nodename at +65
                    let nodename = b"ainux\0";
                    core::ptr::copy_nonoverlapping(nodename.as_ptr(), buf.add(65), nodename.len());
                    // release at +130
                    let release = b"0.3.0\0";
                    core::ptr::copy_nonoverlapping(release.as_ptr(), buf.add(130), release.len());
                    // version at +195
                    let version = b"#1 SMP\0";
                    core::ptr::copy_nonoverlapping(version.as_ptr(), buf.add(195), version.len());
                    // machine at +260
                    let machine = b"x86_64\0";
                    core::ptr::copy_nonoverlapping(machine.as_ptr(), buf.add(260), machine.len());
                }
            }
            0
        },
        SYS_FCNTL => 0,            // Stub: succeed
        SYS_ARCH_PRCTL => mm::sys_arch_prctl(arg1 as i32, arg2),
        SYS_SET_TID_ADDRESS => 1,  // Return TID 1
        SYS_SET_ROBUST_LIST => 0,  // Stub: succeed
        SYS_EXIT | SYS_EXIT_GROUP => {
            // Terminate the current process and return to scheduler
            crate::process::scheduler::sys_exit(arg1 as isize);
            // sys_exit does not return — if we somehow get here, yield to scheduler
            loop { unsafe { core::arch::x86_64::_mm_pause(); } }
        },
        SYS_KILL => {
            // arg1 = pid, arg2 = sig
            crate::process::scheduler::kill_task(arg1)
        },
        SYS_PRLIMIT64 => {
            // Stub: return success, fill in default rlimits if needed
            0
        },
        SYS_GETRANDOM => {
            // Fill buffer with pseudo-random bytes (simple LFSR for now)
            let buf = arg1 as *mut u8;
            let count = arg2;
            if !buf.is_null() {
                let tsc = unsafe { core::arch::x86_64::_rdtsc() };
                let mut seed = tsc;
                unsafe {
                    for i in 0..count {
                        seed ^= seed << 13;
                        seed ^= seed >> 7;
                        seed ^= seed << 17;
                        *buf.add(i) = (seed & 0xFF) as u8;
                    }
                }
            }
            count as isize
        },
        _ => {
            let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
            use core::fmt::Write;
            let _ = write!(serial, "Warning: Unimplemented syscall {}\n", sys_no);
            -38 // ENOSYS
        }
    }
}
