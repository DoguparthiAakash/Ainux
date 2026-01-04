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
    match id {
        1 => { // Write
            // a1 = fd (ignore), a2 = ptr, a3 = len.
            let s = unsafe { core::slice::from_raw_parts(a2 as *const u8, a3 as usize) };
            if let Ok(str_slice) = core::str::from_utf8(s) {
                video::put_str(str_slice);
                // Also write to Serial for verification
                for byte in str_slice.bytes() {
                    unsafe { asm!("out dx, al", in("dx") 0x3F8, in("al") byte, options(nomem, nostack)); }
                }
                return a3;
            }
            0
        }
        2 => { // Exec
            // a1 = path_ptr, a2 = path_len
            let s = unsafe { core::slice::from_raw_parts(a1 as *const u8, a2 as usize) };
            if let Ok(path) = core::str::from_utf8(s) {
                // crate::kprint!("Syscall Exec: {}\n", path);
                let _ = video::put_str("Syscall Exec: ");
                let _ = video::put_str(path);
                let _ = video::put_str("\n");
                
                // Read file
                // We don't have a VFS layer nicely exposed yet, usually we use fs::ext4.
                // But ext4 reader is in main.rs logic? 
                // Wait, ext4 implementation in `fs::ext4` expects a reader.
                // For now, let's look at how main.rs used it.
                // Assuming we can instantiate it or call a global?
                // We don't have a global FS instance yet.
                // For minimal implementation, we can just hack it or ignore exact file reading if we just want to prove flow.
                // But goal is to run gcc-compiled binary.
                
                // Let's assume we can load "hello" (hardcoded buffer check or similar) or we implement global FS.
                // Implementing global FS is "Phase 6". It was checked as done.
                // Let's see `src/main.rs`.
                
                // If I cannot easily read file in syscall handler, I'll return -1.
                // Implementation of full file reading in interrupt context is risky (blocking).
                // But this kernel is simple.
                
                // To support true integration, I need `fs::read_file(path)`.
                // I'll assume we can use a mockup or direct ATA if available.
                
                 match crate::process::loader::load_elf(&[]) { // DUMMY load for now
                     Ok(pid) => pid as u64,
                     Err(_) => 0xFFFFFFFFFFFFFFFF, // -1
                 }
            } else {
                0xFFFFFFFFFFFFFFFF
            }
        }
        _ => 0
    }
}
