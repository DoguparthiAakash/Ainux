#![no_std]
#![no_main]
#![allow(warnings)]

#[macro_use]
extern crate alloc;

use alloc::sync::Arc;
use core::panic::PanicInfo;
use core::fmt::Write;
use core::arch::asm;
mod cpu;
mod process;
mod mm;
pub mod drivers;
pub mod apps;
pub mod games;
pub mod fs;
pub mod object;
pub mod semantic;
pub mod manager;
pub mod net;

pub mod api;
pub mod shell;
pub mod tui_fm;
pub mod gui;
pub mod engine;
pub mod debug;
pub mod security;
pub mod ipc;
pub mod sysctl;

pub mod sem;
pub mod config;
pub mod lib;

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
            crate::drivers::video::emergency_put_str("A system failure has occurred.\n\n");
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
    let _ = write!(serial, "\n");
    let _ = write!(serial, "╔══════════════════════════════════════════════════╗\n");
    let _ = write!(serial, "║        AINUX KERNEL v0.3 — GRUB/MULTIBOOT       ║\n");
    let _ = write!(serial, "║    Advanced Interrupt & Nucleus Unix eXtended    ║\n");
    let _ = write!(serial, "╚══════════════════════════════════════════════════╝\n");
    let _ = write!(serial, "\n");

    // ─── Phase 0: CPU Detection & Architecture Foundation ───
    let _ = write!(serial, "── Phase 0: CPU/BSS Init ──\n");
    cpu::cpuid::init();

    unsafe {
        // Initialize GS_BASE for the BSP
        let bsp_prcb_addr = cpu::percpu::get_prcb_addr(0);
        cpu::percpu::CPUS[0].init(bsp_prcb_addr);
        cpu::percpu::write_gs_base(bsp_prcb_addr);
    }
    let _ = write!(serial, "GS_BASE set to PRCB[0].\n");
    
    let _ = write!(serial, "Initializing GDT...\n");
    // GDT must be initialized before PMM so that a known good segment and TSS
    // are available if exceptions occur during memory allocation.
    cpu::gdt::init();
    let _ = write!(serial, "GDT Initialized.\n");

    let _ = write!(serial, "Initializing IDT...\n");
    // IDT must be initialized before PMM so that exceptions (like page faults)
    // can be caught and handled gracefully in early boot.
    cpu::idt::init();
    let _ = write!(serial, "IDT Initialized.\n");

    let _ = write!(serial, "Phase 0 Complete.\n");

    let _ = write!(serial, "\n── Phase 1: Memory Subsystem ──\n");
    let _ = write!(serial, "Initializing PMM (Multiboot)...\n");
    mm::pmm::BitmapPmm::init();
    let _ = write!(serial, "PMM Initialized.\n");

    // Strict Memory Usage Checks
    unsafe {
        extern "C" { static __bss_end: u8; }
        let kernel_end_addr = &__bss_end as *const u8 as usize;
        let physical_end = kernel_end_addr - 0xFFFFFFFF80000000;
        let _ = write!(serial, "Kernel Physical End: {:#x} ({} KB)\n", physical_end, physical_end / 1024);
        
        // Assert strict kernel physical footprint < 8MB
        if physical_end > 8 * 1024 * 1024 {
            let _ = write!(serial, "PANIC: Kernel memory usage exceeded 8MB limit! Total: {} bytes\n", physical_end);
            loop { hlt(); }
        }
    }

    // ─── Phase 2: Virtual Memory & Heap ───
    let _ = write!(serial, "\n── Phase 2: Virtual Memory & Heap ──\n");
    
    let _ = write!(serial, "Initializing VMM...\n");
    mm::vmm::init();
    let _ = write!(serial, "VMM Initialized.\n");

    let _ = write!(serial, "Initializing Heap (1.9 MB)...\n");
    mm::heap::init_custom(1900 * 1024);
    let _ = write!(serial, "Heap Initialized.\n");

    let _ = write!(serial, "Initializing Slab Allocator...\n");
    mm::slab::init();
    let _ = write!(serial, "Slab Allocator Initialized.\n");



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
    drivers::partition::init();
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
    
    // ─── Phase 5: Sovereign Runtime & Filesystem ───
    let _ = write!(serial, "\n── Phase 5: Sovereign Runtime & VFS ──\n");

    // 1. Initialize VFS with the best available root
    let root_fs: Arc<dyn fs::vfs::FileSystem> = match fs::ext4::parse_superblock() {
        Ok(sb) => {
            let _ = write!(serial, "VFS: Ext4 partition detected. Using as root.\n");
            Arc::new(fs::ext4::Ext4FileSystem::new(sb))
        },
        Err(_) => {
            let _ = write!(serial, "VFS: No Ext4 partition found. Formatting disk...\n");
            if fs::ext4::Ext4FileSystem::format(0) {
                if let Ok(sb) = fs::ext4::parse_superblock() {
                    let _ = write!(serial, "VFS: Ext4 formatted successfully. Using as root.\n");
                    Arc::new(fs::ext4::Ext4FileSystem::new(sb))
                } else {
                    let _ = write!(serial, "VFS: Ext4 format failed to verify. Falling back to Procfs.\n");
                    Arc::new(fs::procfs::ProcFileSystem)
                }
            } else {
                let _ = write!(serial, "VFS: Ext4 format failed. Falling back to Procfs.\n");
                Arc::new(fs::procfs::ProcFileSystem)
            }
        }
    };
    
    fs::vfs::init(root_fs);
    let _ = write!(serial, "VFS: Root initialized.\n");

    // Mount additional system filesystems
    let procfs = Arc::new(fs::procfs::ProcFileSystem);
    fs::vfs::mount("/proc", procfs);
    let _ = write!(serial, "VFS: Procfs mounted at /proc.\n");

    let devfs = Arc::new(fs::devfs::DevFs);
    fs::vfs::mount("/dev", devfs);
    let _ = write!(serial, "VFS: DevFs mounted at /dev.\n");

    // Mount FAT32 Partitions
    let partitions = crate::drivers::mbr::parse_mbr();
    let mut fat_count = 0;
    for part in partitions {
        if part.partition_type == 0x0B || part.partition_type == 0x0C {
            if let Some(fat_fs) = fs::fat32::Fat32FileSystem::new(part.lba_start) {
                let mount_point = alloc::format!("/fat{}", fat_count);
                fs::vfs::mount(&mount_point, fat_fs);
                let _ = write!(serial, "VFS: FAT32 partition mounted at {}.\n", mount_point);
                fat_count += 1;
            }
        }
    }

    // 2. Initialize Sovereign Managers
    process::cell::init();
    process::scheduler::init();
    crate::sem::init(); // Initialize Semantic Core
    let _ = write!(serial, "Sovereign Runtime: Cell Manager & Scheduler Initialized.\n");

    crate::manager::log::init();
    crate::manager::neural::init();
    crate::manager::desktop_manager::init();
    crate::manager::network_manager::init();
    sysctl::init();
    let _ = write!(serial, "Sysctl Initialized.\n");
    // 3. Initialize Syscalls
    unsafe { cpu::syscall::init(); }
    let _ = write!(serial, "Syscalls: Enabled.\n");

    // 4. Spawn Sentinel (Autonomous Guardian)
    crate::process::scheduler::spawn_kernel_task(crate::manager::sentinel::start as u64, "sentinel");
    let _ = write!(serial, "Sentinel: Active.\n");

    // 5. Load Configuration
    if !LOAD_SAFE_DEFAULTS.load(core::sync::atomic::Ordering::SeqCst) {
        config::load();
        crate::drivers::video::load_theme();
        let _ = write!(serial, "Config & Theme: Loaded from disk.\n");
    } else {
        config::reset_to_defaults();
        let _ = write!(serial, "Config: Safety Mode (Defaults Loaded).\n");
    }

    // 6. Finalize Boot & Enable Preemption
    let _ = write!(serial, "DEBUG: Enabling Interrupts (STI)...\n");
    unsafe { asm!("sti"); }
    let _ = write!(serial, "Kernel: Fully Sovereign. STI successful.\n");

    // NOTE: STI is now before boot_menu so keyboard IRQ1 can fire during menu hlt spin.

    // TEST: Verify VFS access
    {
        let root = fs::vfs::ROOT.lock();
        if let Some(root_inode) = root.as_ref() {
             if let Ok(inode) = root_inode.lookup("hello.txt") {
                 let _ = write!(serial, "VFS: hello.txt found.\n");
                 if let Ok(handle) = inode.open(0) {
                     let mut buf = [0u8; 64];
                     if let Ok(n) = handle.read(&mut buf, 0) {
                         if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                             let _ = write!(serial, "VFS: Read: '{}'\n", s);
                         }
                     }
                 }
             }
        }
    }

    // ----------- Environment Selection Prompt -----------
    // Play startup sound
    crate::drivers::audio::ac97::play_startup_sound();

    // Prompt the user indefinitely with no timeout.
    boot_menu();

    loop {
        unsafe { core::arch::asm!("hlt"); }
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
    drivers::video::put_str("6. Start Desktop Environment (GNOME GUI)\n\n");
    drivers::video::put_str("Select option [1-6]: ");

    loop {
        if let Some(c) = drivers::keyboard::pop_char() {
            match c {
                '1' => {
                    drivers::video::put_str("1\n");
                    crate::shell::run();

                    return;
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
                    drivers::video::put_str("6\nStarting Desktop Environment...\n");
                    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
                    use core::fmt::Write;
                    let _ = write!(serial, "Launching test_drm.elf to verify DRM subsystem...\n");
                    let res = crate::process::loader::load_elf_from_file("test_drm.elf");
                    if res.is_err() {
                        let _ = write!(serial, "Failed to load test_drm.elf\n");
                    }
                    crate::gui::desktop::run();
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
                    crate::shell::run();
                    return;
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
                '5' => {
                    drivers::video::put_str("5\nLoading Defaults...\n");
                    LOAD_SAFE_DEFAULTS.store(true, core::sync::atomic::Ordering::SeqCst);
                    return;
                },
                '6' => {
                    drivers::video::put_str("6\nStarting Desktop Environment...\n");
                    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
                    use core::fmt::Write;
                    let _ = write!(serial, "Launching test_drm.elf to verify DRM subsystem...\n");
                    let res = crate::process::loader::load_elf_from_file("test_drm.elf");
                    if res.is_err() {
                        let _ = write!(serial, "Failed to load test_drm.elf\n");
                    }
                    crate::gui::desktop::run();
                },
               _ => {}
            }
        }
        unsafe { asm!("hlt"); }
    }
}
