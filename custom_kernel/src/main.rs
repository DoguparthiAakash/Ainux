#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use core::panic::PanicInfo;
use limine::request::FramebufferRequest;
use core::fmt::{self, Write};
use core::arch::asm;
mod fs;
mod mm;
mod cpu;
mod process;
mod drivers;
mod shell;
pub mod gui;
pub mod engine;
pub mod debug;
pub mod security;
pub mod ipc;
pub mod nux; // Enabled (compiler only)
pub mod sem;


// Request a framebuffer from Limine
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

pub fn hlt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // In panic, try to construct a fresh serial port to avoid deadlock if lock is held
    let mut serial = drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "PANIC: {:?}\n", _info);
    loop {
        hlt();
    }
}


#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "Hello from Ainux Core!\n");

    // C/ASM Integration test removed - not needed for shell

    // if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
    //    if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
    //        let bpp = framebuffer.bpp() as usize;
    //        let pitch = framebuffer.pitch() as usize;
    //        let height = framebuffer.height() as usize;
    //        let width = framebuffer.width() as usize;
    //        let buffer = framebuffer.addr();
    //        
    //        // Blue screen removed per user request for command level operation.
    //        // if bpp == 32 { ... }
    //        let _ = write!(serial, "Framebuffer Initialized (Visuals Disabled).\n");
    //    }
    // }

    // Framebuffer Disabled (Visuals Handled by C driver)
    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
       if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
           let bpp = framebuffer.bpp();
           let _ = write!(serial, "Debug: Framebuffer BPP: {}\n", bpp);
       }
    }
    let _ = write!(serial, "Debug: Framebuffer Block Skipped (C Driver Active).\n");

    // Initialize PMM
    let _ = write!(serial, "Initializing PMM...\n");
    unsafe { mm::pmm::BitmapPmm::init(); }
    let _ = write!(serial, "Debug: PMM Initialized.\n");
    
    // Initialize VMM
    let _ = write!(serial, "Initializing VMM...\n");
    mm::vmm::init();
    let _ = write!(serial, "Debug: VMM Initialized.\n");

    // Initialize Heap
    let _ = write!(serial, "Initializing Heap...\n");
    mm::heap::init();
    let _ = write!(serial, "Heap Initialized.\n");
    
    // Initialize GDT
    let _ = write!(serial, "Initializing GDT...\n");
    cpu::gdt::init();
    let _ = write!(serial, "GDT Initialized.\n");

    // Initialize IDT
    let _ = write!(serial, "Initializing IDT...\n");
    cpu::idt::init();
    let _ = write!(serial, "IDT Initialized.\n");
    
    // Initialize PIC
    let _ = write!(serial, "Initializing PIC...\n");
    unsafe { cpu::pic::init(); }
    let _ = write!(serial, "PIC Remapped.\n");
    
    // Initialize Drivers
    unsafe {
        extern "C" { fn ps2_init(); }
        ps2_init();
    }
    
    // Initialize Timer (100 Hz)
    drivers::timer::init(100);
    let _ = write!(serial, "Timer Initialized (100Hz).\n");
    
    drivers::keyboard::init();
    let _ = write!(serial, "Keyboard Initialized (IRQ1 Unmasked).\n");
    drivers::mouse::init();
    let _ = write!(serial, "Mouse Initialized (IRQ12 Unmasked).\n");
    
    drivers::video::init();
    drivers::video::put_str("Ainux Kernel v0.1\n");
    drivers::video::put_str("Initializing...\n");
    
    // Test ATA
    let _ = write!(serial, "ATA TEST START\n");
    if drivers::ata::identify() {
       let _ = write!(serial, "ATA Drive 0 Identified.\n");
       
       // DEBUG: Manually read LBA 2
       let mut buf = [0u16; 256]; // 1 sector (512 bytes)
       if drivers::ata::read_sectors(&mut buf, 2, 1) { // LBA 2, 1 sector
           let magic_val = buf[28]; // Offset 56 bytes / 2 = 28 words
           let _ = write!(serial, "DEBUG: LBA 2 Read. Word[28] (Magic?): {:#x}\n", magic_val);
           let _ = write!(serial, "DEBUG: Word[0]: {:#x}, Word[1]: {:#x}\n", buf[0], buf[1]);
       } else {
           let _ = write!(serial, "DEBUG: LBA 2 Read Failed.\n");
       }

       drivers::video::put_str("ATA Drive Found. Checking EXT4...\n");
       
       match fs::ext4::parse_superblock() {
           Ok(sb) => {
                let magic = sb.magic;
                let inodes = sb.inodes_count;
                let blocks = sb.blocks_count_lo;
                let _ = write!(serial, "EXT4 Superblock Found! Magic: {:#x}\n", magic);
                let _ = write!(serial, "Inodes: {}, Blocks: {}\n", inodes, blocks);
                drivers::video::put_str("EXT4 Filesystem Detected!\n");
                
                // Init VFS if not already done in original main block?
                // Actually I have VFS init block further down.
                // But this block is inside "Test ATA".
                // I should avoid duplicate logic?
                // The later block also calls parse_superblock.
                // I should update that one too.
           },
           Err(m) => {
                let _ = write!(serial, "EXT4 Superblock NOT Found. Read Magic: {:#x}\n", m);
                drivers::video::put_str("EXT4 Not Found.\n");
           }
       }
    } else {
       let _ = write!(serial, "ATA Drive 0 NOT Found.\n");
       drivers::video::put_str("ATA Drive Check Failed.\n");
    }
    let _ = write!(serial, "ATA TEST END\n");
    
    // Initialize Scheduler
    process::scheduler::init();
    crate::sem::init(); // Initialize Semantic Core
    let _ = write!(serial, "Scheduler Initialized.\n");

    // Enable Interrupts
    unsafe { asm!("sti"); }
    
    // Initialize Syscalls
    unsafe { cpu::syscall::init(); }
    let _ = write!(serial, "Syscalls Enabled.\n");

    // ----------- Enter Userspace -----------
    let _ = write!(serial, "Preparing Userspace...\n");
    crate::drivers::video::put_str("Entering Ring 3...\n");

    // 1. Allocate a frame for User Code + Stack
    // We'll map it to 0x400000 (standard load address usually, but arbitrary here)
    let user_base = 0x400000;
    
    {
        let mut pmm_lock = mm::pmm::PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
             if let Some(frame) = pmm.alloc_frame() {
                 unsafe {
                     // Unmap first just in case
                     unsafe { mm::vmm::unmap_page(user_base); }
                     
                     // Map it as USER | WRITABLE | PRESENT
                     // User bit (bit 2) MUST be set.
                     // 0x07 = Present | RW | User
                     match mm::vmm::map_page(user_base, frame, 0x07) {
                         Ok(_) => {},
                         Err(e) => {
                             let _ = write!(serial, "Failed to map user page: {}\n", e);
                         }
                     }
                     
                     // 2. Copy code to this page.
                     let ptr = user_base as *mut u8;
                     
                     // Minimal User Payload (Assembly):
                     // mov rax, 1 (Syscall ID = Write)
                     // mov rdi, 0 (FD - ignored)
                     // lea rsi, [rip + offset] (String)
                     // mov rdx, 12 (Len)
                     // syscall
                     // jmp $
                     
                     let code: [u8; 40] = [
                         0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00, // mov rax, 1
                         0x48, 0x31, 0xff,                         // xor rdi, rdi
                         0x48, 0x8d, 0x35, 0x0c, 0x00, 0x00, 0x00, // lea rsi, [rip + 12]
                         0x48, 0xc7, 0xc2, 0x0c, 0x00, 0x00, 0x00, // mov rdx, 12
                         0x0f, 0x05,                               // syscall
                         0xeb, 0xfe,                               // jmp $
                         // String "Hello User!\n"
                         b'H', b'e', b'l', b'l', b'o', b' ', b'U', b's', b'e', b'r', b'!', b'\n' 
                     ];
                     
                     // Fix LEA offset:
                     // 0: mov rax (7)
                     // 7: xor rdi (3)
                     // 10: lea rsi (7). End is 17.
                     // 17: mov rdx (7).
                     // 24: syscall (2).
                     // 26: jmp (2).
                     // 28: String starts.
                     // Target: 28. Current RIP (after LEA): 17. Diff: 11 (0x0B).
                     // Encoded LEA: 48 8d 35 0c ... -> 0x0c is 12.
                     // We need 11.
                     
                     let mut final_code = code;
                     final_code[13] = 0x0B; 
                     
                     for i in 0..final_code.len() {
                         *ptr.add(i) = final_code[i];
                     }
                     
                     // 3. Spawn Task (PID 1)
                     /*
                     let stack_top = user_base + 4096;
                     process::scheduler::spawn_user(user_base, stack_top, 0);
                     */
                 }
             } else {
                 let _ = write!(serial, "Failed to alloc user frame.\n");
             }
        }
    }

    // Initialize VFS
    match fs::ext4::parse_superblock() {
        Ok(sb) => {
            let _ = write!(serial, "Ext4: Superblock found. Init VFS...\n");
            let ext4 = alloc::sync::Arc::new(fs::ext4::Ext4FileSystem::new(sb));
            fs::vfs::init(ext4);
            let _ = write!(serial, "VFS: Initialized.\n");
            
            // TEST: Read hello.txt
            let root = fs::vfs::ROOT.lock();
            if let Some(root_inode) = root.as_ref() {
                 match root_inode.lookup("hello.txt") {
                     Ok(inode) => {
                         let _ = write!(serial, "VFS: Found hello.txt!\n");
                         if let Ok(handle) = inode.open(0) {
                             let mut buf = [0u8; 64];
                             if let Ok(n) = handle.read(&mut buf, 0) {
                                 let _ = write!(serial, "VFS: Read {} bytes: ", n);
                                 if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                                     let _ = write!(serial, "'{}'\n", s);
                                 } else {
                                      let _ = write!(serial, "<binary>\n");
                                 }
                             }
                         }
                     },
                     Err(e) => {
                          let _ = write!(serial, "VFS: hello.txt lookup failed: {:?}\n", e);
                     }
                 }
            }
        },
        Err(m) => {
            let _ = write!(serial, "Ext4: Superblock NOT found! Magic: {:#x}\n", m);
        }
    }

    // ----------- Boot Menu -----------
    boot_menu();

    // ----------- Start Shell -----------
    crate::drivers::video::put_str("Starting Shell...\n");
    crate::shell::run();

    loop {
        unsafe { asm!("hlt"); }
    }
}

fn boot_menu() {
    drivers::video::clear();
    drivers::video::put_str("\n=== Ainux Boot Menu (Rust) ===\n\n");
    drivers::video::put_str("1. Start Kernel (Shell)\n");
    drivers::video::put_str("2. Network Diagnostics\n");
    drivers::video::put_str("3. Reboot\n");
    drivers::video::put_str("4. Shutdown\n\n");
    drivers::video::put_str("Select option [1-4]: ");

    loop {
        if let Some(c) = drivers::keyboard::pop_char() {
            match c {
                '1' => {
                    drivers::video::put_str("1\n");
                    return; // Proceed to Shell
                },
                '2' => {
                    drivers::video::put_str("2\n");
                    drivers::video::put_str("Network Diagnostics not implemented yet.\n");
                    drivers::video::put_str("Select option [1-4]: ");
                },
                '3' => {
                    drivers::video::put_str("3\nRebooting...\n");
                    unsafe {
                        // Pulse 0xFE to 0x64 (CPU Reset)
                        loop {
                             let status: u8;
                             core::arch::asm!("in al, 0x64", out("al") status);
                             if status & 2 == 0 { break; }
                        }
                        core::arch::asm!("out 0x64, al", in("al") 0xFE as u8);
                        core::arch::asm!("hlt");
                    }
                },
                '4' => {
                    drivers::video::put_str("4\nShutting down...\n");
                    unsafe {
                        // QEMU Shutdown (0x2000 to 0x604)
                        core::arch::asm!("out dx, ax", in("dx") 0x604 as u16, in("ax") 0x2000 as u16);
                        loop { core::arch::asm!("hlt"); }
                    }
                },
                _ => {}
            }
        }
        unsafe { asm!("hlt"); }
    }
}
