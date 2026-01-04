#![no_std]
#![no_main]

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

// Request a framebuffer from Limine
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

// Simple Serial Port Wrapper
struct SerialPort {
    port: u16,
}

impl SerialPort {
    fn new(port: u16) -> Self {
        Self { port }
    }

    fn write_byte(&mut self, byte: u8) {
        unsafe {
            asm!("out dx, al", in("dx") self.port, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

pub fn hlt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let mut serial = SerialPort::new(0x3F8);
    let _ = write!(serial, "PANIC: {:?}\n", _info);
    loop {
        hlt();
    }
}


#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut serial = SerialPort::new(0x3F8);
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

    // Initialize Heap (Skipped due to VMM infinite loop/crash)
    // mm::heap::init();
    
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
    drivers::keyboard::init();
    let _ = write!(serial, "Keyboard Initialized (IRQ1 Unmasked).\n");
    drivers::mouse::init();
    let _ = write!(serial, "Mouse Initialized (IRQ12 Unmasked).\n");
    
    drivers::video::init();
    drivers::video::put_str("Ainux Kernel v0.1\n");
    drivers::video::put_str("Initializing...\n");
    
    // Test ATA
    // let _ = write!(serial, "ATA TEST START\n");
    // if drivers::ata::identify() {
    //    let _ = write!(serial, "ATA Drive 0 Identified.\n");
    //    drivers::video::put_str("ATA Drive Found. Checking EXT4...\n");
    //    
    //    if let Some(sb) = fs::ext4::parse_superblock() {
    //         let magic = sb.magic;
    //         let inodes = sb.inodes_count;
    //         let blocks = sb.blocks_count_lo;
    //         let _ = write!(serial, "EXT4 Superblock Found! Magic: {:#x}\n", magic);
    //         let _ = write!(serial, "Inodes: {}, Blocks: {}\n", inodes, blocks);
    //         drivers::video::put_str("EXT4 Filesystem Detected!\n");
    //    } else {
    //         let _ = write!(serial, "EXT4 Superblock NOT Found (Magic mismatch).\n");
    //         drivers::video::put_str("EXT4 Not Found.\n");
    //    }
    // } else {
    //    let _ = write!(serial, "ATA Drive 0 NOT Found.\n");
    //    drivers::video::put_str("ATA Drive Check Failed.\n");
    // }
    // let _ = write!(serial, "ATA TEST END\n");
    
    // Enable Interrupts
    unsafe { asm!("sti"); }
    
    // Initialize Syscalls
    unsafe { cpu::syscall::init(); }
    let _ = write!(serial, "Syscalls Enabled.\n");

    // ----------- Enter Userspace (DISABLED FOR SHELL ACCESS) -----------
    // let _ = write!(serial, "Preparing Userspace...\n");
    // drivers::video::put_str("Entering Ring 3...\n");

    // // 1. Allocate a frame for User Code + Stack
    // // We'll map it to 0x400000 (standard load address usually, but arbitrary here)
    // let user_base = 0x400000;
    
    // {
    //     let mut pmm_lock = mm::pmm::PMM.lock();
    //     if let Some(ref mut pmm) = *pmm_lock {
    //          if let Some(frame) = pmm.alloc_frame() {
    //              unsafe {
    //                  // Map it as USER | WRITABLE | PRESENT
    //                  // User bit (bit 2) MUST be set.
    //                  // 0x01 (Present) | 0x02 (Writable) | 0x04 (User) = 0x07
    //                  let _ = mm::vmm::map_page(user_base, frame, 0x07);
                     
    //                  // 2. Copy code to this page.
    //                  let ptr = user_base as *mut u8;
                     
    //                  // Minimal User Payload (Assembly):
    //                  // mov rax, 1 (Syscall ID = Write)
    //                  // mov rdi, 0 (FD - ignored)
    //                  // lea rsi, [rip + offset] (String)
    //                  // mov rdx, 12 (Len)
    //                  // syscall
    //                  // jmp $
                     
    //                  let code: [u8; 40] = [
    //                      0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00, // mov rax, 1
    //                      0x48, 0x31, 0xff,                         // xor rdi, rdi
    //                      0x48, 0x8d, 0x35, 0x0c, 0x00, 0x00, 0x00, // lea rsi, [rip + 12]
    //                      0x48, 0xc7, 0xc2, 0x0c, 0x00, 0x00, 0x00, // mov rdx, 12
    //                      0x0f, 0x05,                               // syscall
    //                      0xeb, 0xfe,                               // jmp $
    //                      // String "Hello User!\n"
    //                      b'H', b'e', b'l', b'l', b'o', b' ', b'U', b's', b'e', b'r', b'!', b'\n' 
    //                  ];
                     
    //                  // Fix LEA offset:
    //                  // 0: mov rax (7)
    //                  // 7: xor rdi (3)
    //                  // 10: lea rsi (7). End is 17.
    //                  // 17: mov rdx (7).
    //                  // 24: syscall (2).
    //                  // 26: jmp (2).
    //                  // 28: String starts.
    //                  // Target: 28. Current RIP (after LEA): 17. Diff: 11 (0x0B).
    //                  // Encoded LEA: 48 8d 35 0c ... -> 0x0c is 12.
    //                  // We need 11.
                     
    //                  let mut final_code = code;
    //                  final_code[13] = 0x0B; 
                     
    //                  for i in 0..final_code.len() {
    //                      *ptr.add(i) = final_code[i];
    //                  }
                     
    //                  // 3. Jump!
    //                  // Stack at end of page (0x400000 + 4096 = 0x401000)
    //                  let stack_top = user_base + 4096;
                     
    //                  let kstack: u64;
    //                  asm!("mov {}, rsp", out(reg) kstack);
    //                  cpu::gdt::set_kernel_stack(kstack);
                     
    //                  cpu::userspace::enter_userspace(user_base, stack_top);
    //              }
    //          } else {
    //              let _ = write!(serial, "Failed to alloc user frame.\n");
    //          }
    //     }
    // }

    // Start Shell (replaces loop)
    let _ = write!(serial, "Debug: Starting Shell...\n");
    shell::run();
    
    loop {
         unsafe { asm!("hlt"); }
    }
}
