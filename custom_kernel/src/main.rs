#![no_std]
#![no_main]
#![allow(warnings)]

#[macro_use]
extern crate alloc;

use core::panic::PanicInfo;
use core::fmt::Write;
use core::arch::asm;
mod cpu;
mod process;
mod mm;
pub mod drivers;
pub mod apps;
pub mod fs;
pub mod net;

pub mod api;
pub mod shell;
pub mod gui;
pub mod engine;
pub mod debug;
pub mod security;
pub mod ipc;
// pub mod nux; // Disabled (nux_portable missing)

pub mod sem;
pub mod config;

pub static LOAD_SAFE_DEFAULTS: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);


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
    
    let _ = write!(serial, "Attempting to recover to Shell...\n");
    crate::shell::run();
    
    loop {
        hlt();
    }
}


#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "\n");
    let _ = write!(serial, "╔══════════════════════════════════════════════════╗\n");
    let _ = write!(serial, "║         AINUX KERNEL v0.2 — GRUB/SMP            ║\n");
    let _ = write!(serial, "║    Advanced Interrupt & Nucleus Unix eXtended    ║\n");
    let _ = write!(serial, "╚══════════════════════════════════════════════════╝\n");
    let _ = write!(serial, "\n");

    // ─── Phase 0: CPU Detection ───
    let _ = write!(serial, "── Phase 0: CPU Detection ──\n");
    cpu::cpuid::init();

    // ─── Phase 1: Memory Subsystem (Multiboot) ───
    let _ = write!(serial, "\n── Phase 1: Memory Subsystem ──\n");
    let _ = write!(serial, "Initializing PMM (Multiboot)...\n");
    mm::pmm::BitmapPmm::init();
    let _ = write!(serial, "PMM Initialized.\n");

    let _ = write!(serial, "Initializing VMM...\n");
    mm::vmm::init();
    let _ = write!(serial, "VMM Initialized.\n");

    let _ = write!(serial, "Initializing Heap (32 MB)...\n");
    mm::heap::init();
    let _ = write!(serial, "Heap Initialized.\n");

    let _ = write!(serial, "Initializing Slab Allocator...\n");
    mm::slab::init();
    let _ = write!(serial, "Slab Allocator Initialized.\n");

    // ─── Phase 2: Core CPU Structures ───
    let _ = write!(serial, "\n── Phase 2: Core CPU Structures ──\n");
    let _ = write!(serial, "Initializing GDT...\n");
    cpu::gdt::init();
    let _ = write!(serial, "GDT Initialized.\n");

    let _ = write!(serial, "Initializing IDT...\n");
    cpu::idt::init();
    let _ = write!(serial, "IDT Initialized.\n");

    // Initialize BSP's PRCB for per-cpu storage
    unsafe {
        let bsp_prcb_addr = cpu::percpu::get_prcb_addr(0);
        cpu::percpu::CPUS[0].init(bsp_prcb_addr);
        cpu::percpu::write_gs_base(bsp_prcb_addr);
    }
    let _ = write!(serial, "BSP PRCB Initialized (GS_BASE set).\n");

    let _ = write!(serial, "Initializing PIC (legacy)...\n");
    unsafe { cpu::pic::init(); }
    let _ = write!(serial, "PIC Remapped.\n");

    // ─── Phase 3: ACPI + Multi-Core Bootstrap ───
    let _ = write!(serial, "\n── Phase 3: ACPI & Multi-Core ──\n");
    cpu::acpi::init();

    // Initialize APIC (replaces PIC for multi-core routing)
    cpu::apic::init_lapic();
    cpu::apic::init_ioapic();

    // Boot all Application Processors
    cpu::smp::init();

    // Now we know CPU count — init zone allocator with correct count
    let cpu_count = cpu::percpu::get_cpu_count();
    let _ = write!(serial, "Initializing Zone Allocator ({} CPUs)...\n", cpu_count);
    mm::zone::init(cpu_count.max(1));
    let _ = write!(serial, "Zone Allocator Initialized.\n");

    // ─── Phase 4: Drivers ───
    let _ = write!(serial, "\n── Phase 4: Device Drivers ──\n");
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
    
    // Initialize IO Kit
    let _ = write!(serial, "Initializing IO Kit...\n");
    drivers::iokit::registry::init();
    let _ = write!(serial, "IO Kit Registry Initialized.\n");
    
    // Initialize PCI
    drivers::pci::init();
    net::init();
    
    // Initialize Simulated WiFi (Atheros)
    if let Some(root) = drivers::iokit::registry::REGISTRY.lock().root.clone() {
        let wifi = drivers::net::atheros::AtherosHAL::new();
        let entry = drivers::iokit::registry::IORegistryEntry::new(wifi.clone());
        drivers::iokit::registry::IORegistryEntry::add_child(&root, &entry);
        use crate::drivers::iokit::service::IOService;
        let _ = IOService::start(&*wifi, &root.service);
    }

    // Auto-Run IO Kit Test
    drivers::iokit::test::run_test();
    
    // Test ATA
    let _ = write!(serial, "ATA TEST START\n");
    let mut ident_buf = [0u16; 256];
    if drivers::ata::identify_buffer(&mut ident_buf) {
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

    // Initialize Syscalls (MUST be before STI to avoid timer interrupts during MSR setup)
    unsafe { cpu::syscall::init(); }
    let _ = write!(serial, "Syscalls Enabled.\n");

    // Enable Interrupts
    unsafe { asm!("sti"); }
    let _ = write!(serial, "Interrupts Enabled.\n");

    // Userspace bootstrap will now be handled securely by loader.rs via VFS.
    // Proceed directly to VFS mounting and Shell...

    // Initialize VFS
    match fs::ext4::parse_superblock() {
        Ok(sb) => {
            let _ = write!(serial, "Ext4: Superblock found. Init VFS...\n");
            let ext4 = alloc::sync::Arc::new(fs::ext4::Ext4FileSystem::new(sb));
            fs::vfs::init(ext4);
            let _ = write!(serial, "VFS: Initialized.\n");
            
            // Initialize System Configuration
            if !LOAD_SAFE_DEFAULTS.load(core::sync::atomic::Ordering::SeqCst) {
                config::load();
                let _ = write!(serial, "Config: Loaded from disk.\n");
            } else {
                config::reset_to_defaults();
                let _ = write!(serial, "Config: Safety Mode (Defaults Loaded).\n");
            }

            // Apply Config (Example: Network)
            {
                if let Some(sys_config) = config::CONFIG.lock().as_ref() {
                    let _ = write!(serial, "Config Applied: Hostname='{}' IP='{}'\n", sys_config.hostname, sys_config.ip_address);
                }
            }
            
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
    drivers::video::put_str("4. Shutdown\n");
    drivers::video::put_str("5. Load Default Config & Start Shell\n\n");
    drivers::video::put_str("Select option [1-5]: ");

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
                '5' => {
                    drivers::video::put_str("5\nLoading Defaults...\n");
                    LOAD_SAFE_DEFAULTS.store(true, core::sync::atomic::Ordering::SeqCst);
                    return;
                },
                _ => {}
            }
        }
        
        // Serial Poll
        if drivers::serial::SERIAL.lock().data_ready() {
            let b = drivers::serial::SERIAL.lock().read_byte();
            let c = b as char;
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
