use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

pub mod fs;
pub mod mm;

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
pub const SYS_EXIT: usize = 60;
pub const SYS_UNAME: usize = 63;
pub const SYS_FCNTL: usize = 72;
pub const SYS_GETUID: usize = 102;
pub const SYS_GETGID: usize = 104;
pub const SYS_GETEUID: usize = 107;
pub const SYS_GETEGID: usize = 108;
pub const SYS_ARCH_PRCTL: usize = 158;
pub const SYS_GETTID: usize = 186;
pub const SYS_SET_TID_ADDRESS: usize = 218;
pub const SYS_EXIT_GROUP: usize = 231;
pub const SYS_OPENAT: usize = 257;
pub const SYS_SET_ROBUST_LIST: usize = 273;
pub const SYS_PRLIMIT64: usize = 302;
pub const SYS_GETRANDOM: usize = 318;
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
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "Syscall: {}\n", sys_no);

    match sys_no {
        SYS_READ => fs::sys_read(arg1, arg2 as *mut u8, arg3),
        SYS_WRITE => fs::sys_write(arg1, arg2 as *const u8, arg3),
        SYS_OPEN => fs::sys_open(arg1 as *const u8, arg2 as i32, arg3 as i32),
        SYS_CLOSE => fs::sys_close(arg1),
        SYS_FSTAT => {
            // Stub: zero-fill the stat buffer
            let buf = arg2 as *mut u8;
            if !buf.is_null() {
                unsafe { core::ptr::write_bytes(buf, 0, 144); } // sizeof(struct stat) = 144
            }
            0
        },
        SYS_LSEEK => 0, // Stub: pretend we're at offset 0
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
        SYS_OPENAT => fs::sys_openat(arg1 as i32, arg2 as *const u8, arg3 as i32, arg4 as i32),
        SYS_DISK_READ => fs::sys_disk_read(arg1 as u32, arg2 as u8, arg3 as *mut u8),
        SYS_DISK_WRITE => fs::sys_disk_write(arg1 as u32, arg2 as u8, arg3 as *const u8),
        SYS_DISK_IDENTIFY => fs::sys_disk_identify(arg1 as *mut u8),
        SYS_GETPID => 1,           // PID 1 (init)

        SYS_GETTID => 1,           // TID 1
        SYS_GETUID | SYS_GETEUID => 0, // root
        SYS_GETGID | SYS_GETEGID => 0, // root
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
            let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
            use core::fmt::Write;
            let _ = write!(serial, "Process exited with status {}\n", arg1);
            // TODO: properly terminate the task and schedule next
            crate::hlt();
            0
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
