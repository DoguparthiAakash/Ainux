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
pub mod object;
pub mod semantic;
pub mod manager;
pub mod net;
pub mod wasm;

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

static PANIC_RECURSION: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let depth = PANIC_RECURSION.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    
    // 1. Emergency Serial Output
    let mut serial = drivers::serial::SerialPort::new(0x3F8);
    if depth == 0 {
        let _ = write!(serial, "\n\n!!! KERNEL PANIC !!!\n");
        let _ = write!(serial, "Info: {:?}\n", _info);
    } else if depth < 3 {
        let _ = write!(serial, "\n[RECURSIVE PANIC {}] Info: {:?}\n", depth, _info);
    } else {
        loop { hlt(); }
    }
    
    // 2. Emergency Video Output (Lockless & Allocation-Free)
    unsafe {
        crate::drivers::video::panic_clear();
        crate::drivers::video::emergency_put_str("               --- KERNEL PANIC ---\n\n");
        
        if depth == 0 {
            crate::drivers::video::emergency_put_str("A sovereign system failure has occurred.\n\n");
            // We can't easily format! without allocation, so we just put some static text 
            // and hope the serial log has the details.
            crate::drivers::video::emergency_put_str("Check serial output (0x3F8) for full PanicInfo.\n");
            crate::drivers::video::emergency_put_str("System Halted.\n");
        } else {
            crate::drivers::video::emergency_put_str("RECURSIVE PANIC DETECTED. HALTING.\n");
        }
    }
    
    loop {
        hlt();
    }
}


#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "D");
    let _ = write!(serial, "\n");
    let _ = write!(serial, "╔══════════════════════════════════════════════════╗\n");
    let _ = write!(serial, "║         AINUX KERNEL v0.2 — GRUB/SMP            ║\n");
    let _ = write!(serial, "║    Advanced Interrupt & Nucleus Unix eXtended    ║\n");
    let _ = write!(serial, "╚══════════════════════════════════════════════════╝\n");
    let _ = write!(serial, "\n");

    // ─── Phase 0: CPU Detection ───
    let _ = write!(serial, "── Phase 0: CPU/BSS Init ──\n");
    cpu::cpuid::init();

    unsafe {
        extern "C" {
            static mut __bss_start: u8;
            static mut __bss_end: u8;
        }
        let start_ptr = &mut __bss_start as *mut u8;
        let end_ptr = &mut __bss_end as *mut u8;
        let size = end_ptr as usize - start_ptr as usize;

        let _ = write!(serial, "BSS: Start={:p}, End={:p}, Size={}\n", start_ptr, end_ptr, size);

        if size > 0 && size < 0x10000000 {
             let _ = write!(serial, "BSS: Zeroing...\n");
             core::ptr::write_bytes(start_ptr, 0, size);
             let _ = write!(serial, "BSS: Done.\n");
        }

        // Initialize GS_BASE for the BSP
        let bsp_prcb_addr = cpu::percpu::get_prcb_addr(0);
        let _ = write!(serial, "GS_BASE: PRCB[0] at {:p}\n", bsp_prcb_addr as *const u8);
        
        cpu::percpu::CPUS[0].init(bsp_prcb_addr, 0);
        let _ = write!(serial, "GS_BASE: CPU State Init Done.\n");
        
        cpu::percpu::write_gs_base(bsp_prcb_addr);
        let _ = write!(serial, "GS_BASE: Set.\n");
        
        cpu::percpu::write_kernel_gs_base(bsp_prcb_addr);
    }
    let _ = write!(serial, "Kernel State Initialized.\n");
    
    // ─── Phase 1: Memory Subsystem (Multiboot) ───
    let _ = write!(serial, "\n── Phase 1: Memory Subsystem ──\n");
    let _ = write!(serial, "Initializing PMM (Multiboot)...\n");
    mm::pmm::BitmapPmm::init();
    let _ = write!(serial, "PMM Initialized.\n");

    let _ = write!(serial, "Initializing VMM...\n");
    mm::vmm::init();
    let _ = write!(serial, "VMM Initialized.\n");

    let _ = write!(serial, "Initializing Heap (32 MB)...\n");
    mm::heap::init_custom(32 * 1024 * 1024);
    let _ = write!(serial, "Heap Initialized.\n");

    let _ = write!(serial, "Initializing Slab Allocator...\n");
    mm::slab::init();
    let _ = write!(serial, "Slab Allocator Initialized.\n");

    // Initialize Sovereign Managers (NOW THAT HEAP IS READY)
    crate::manager::log::init();
    crate::manager::neural::init();

    // ─── Phase 2: Core CPU Structures ───
    let _ = write!(serial, "\n── Phase 2: Core CPU Structures ──\n");
    let _ = write!(serial, "Initializing GDT...\n");
    unsafe { cpu::gdt::init(); }
    let _ = write!(serial, "GDT Initialized.\n");

    let _ = write!(serial, "Initializing IDT...\n");
    cpu::idt::init();
    let _ = write!(serial, "IDT Initialized.\n");


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
    
    // Initialize GUI Compositor
    gui::compositor::Compositor::init();
    let _ = write!(serial, "GUI Compositor Initialized.\n");
    
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
    
    // Initialize Rooms & Scheduler
    process::room::init();
    process::scheduler::init();
    crate::sem::init(); // Initialize Semantic Core
    let _ = write!(serial, "Scheduler & Rooms Initialized.\n");

    // ─── Phase 5: Autonomous Manager (Sentinel) ───
    let _ = write!(serial, "DEBUG: Spawning Sentinel...\n");
    crate::process::scheduler::spawn_kernel_task(crate::manager::sentinel::start as u64);
    let _ = write!(serial, "DEBUG: Sentinel Spawned.\n");

    // Initialize Syscalls (MUST be before STI to avoid timer interrupts during MSR setup)
    let _ = write!(serial, "DEBUG: Initializing Syscalls...\n");
    unsafe { cpu::syscall::init(); }
    let _ = write!(serial, "DEBUG: Syscalls Enabled.\n");

    // Initialize VFS
    let _ = write!(serial, "DEBUG: Initializing VFS...\n");
    match fs::ext4::parse_superblock() {
        Ok(sb) => {
            let _ = write!(serial, "Ext4: Superblock found. Init VFS...\n");
            let ext4 = alloc::sync::Arc::new(fs::ext4::Ext4FileSystem::new(sb));
            fs::vfs::init(ext4);
            let _ = write!(serial, "VFS: Ext4 Root Initialized.\n");
            
            // Mount Procfs (Industrial Standard)
            let procfs = alloc::sync::Arc::new(fs::procfs::ProcFileSystem);
            fs::vfs::mount("/proc", procfs);
            let _ = write!(serial, "VFS: Procfs mounted at /proc.\n");
            
            // Initialize System Configuration
            if !LOAD_SAFE_DEFAULTS.load(core::sync::atomic::Ordering::SeqCst) {
                config::load();
                let _ = write!(serial, "Config: Loaded from disk.\n");
            } else {
                config::reset_to_defaults();
                let _ = write!(serial, "Config: Safety Mode (Defaults Loaded).\n");
            }

            // Enable Interrupts
            let _ = write!(serial, "DEBUG: Enabling Interrupts (STI)...\n");
            unsafe { asm!("sti"); }
            let _ = write!(serial, "DEBUG: Interrupts Enabled.\n");

            // Proceed to manual boot menu/shell
            let _ = write!(serial, "System ready for manual operation.\n");
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
    drivers::video::put_str("5. Load Default Config & Start Shell\n");
    drivers::video::put_str("6. Start Graphical Desktop (GUI)\n\n");
    drivers::video::put_str("Select option [1-6]: ");

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
                '6' => {
                    drivers::video::put_str("6\nStarting GUI Desktop...\n");
                    crate::apps::awm::cmd_awm(&[]);
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
                '6' => {
                    drivers::video::put_str("6\nStarting GUI Desktop...\n");
                    crate::apps::awm::cmd_awm(&[]);
                },
               _ => {}
            }
        }
    }
}


