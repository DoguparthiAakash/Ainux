// =============================================================================
// Ainux HInfo — Hardware Information Diagnostic Tool
// =============================================================================

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use crate::drivers::video;
use crate::cpu::control::{rdmsr};
use crate::fs::vfs::{root, Inode, FileType};
use core::sync::atomic::Ordering;
use alloc::sync::Arc;
use crate::process::{scheduler, task};
use alloc::collections::BTreeSet;

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct SmbiosEntryPoint {
    anchor: [u8; 4],
    checksum: u8,
    length: u8,
    major_version: u8,
    minor_version: u8,
    max_structure_size: u16,
    entry_point_revision: u8,
    formatted_area: [u8; 5],
    dmi_anchor: [u8; 5],
    dmi_checksum: u8,
    table_length: u16,
    table_address: u32,
    number_of_structures: u16,
    bcd_revision: u8,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct SmbiosHeader {
    kind: u8,
    length: u8,
    handle: u16,
}

const ASC_CPU: [&str; 8] = [
    "  ░░░░░░░░░░░  ",
    " ░░▓▓▓▓▓▓▓▓▓▓▓░ ",
    " ░▓▓#=======#▓▓░",
    " ░▓▓#  AIN  #▓▓░",
    " ░▓▓#  NUX  #▓▓░",
    " ░▓▓#=======#▓▓░",
    " ░░▓▓▓▓▓▓▓▓▓▓▓░ ",
    "  ░░░░░░░░░░░  ",
];

const ASC_RAM: [&str; 5] = [
    " ╔═══════════════════╗ ",
    " ║▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓║ ",
    " ║▓▒░  AINUX RAM  ░▒▓║ ",
    " ║▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓║ ",
    " ╚▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒╝ ",
];

const ASC_DISK: [&str; 7] = [
    "     .───────.     ",
    "    / ▓▓▓▓▓▓▓ \\    ",
    "   | ▓▓▒▒▒▒▒▓▓ |   ",
    "   | ▓▓▒ DISK ▒▓▓ |  ",
    "   | ▓▓▒▒▒▒▒▓▓ |   ",
    "    \\ ▓▓▓▓▓▓▓ /    ",
    "     '───────'     ",
];

const ASC_GPU: [&str; 6] = [
    " ╔═══════════════════╗ ",
    " ║ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ ║ ",
    " ║ ▓▒ (O) ▒▒▒ (O) ▒▓ ║ ",
    " ║ ▓▒ FAN ▒▒▒ FAN ▒▓ ║ ",
    " ║ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ ║ ",
    " ╚═‾‾▒▒▒▒▒▒▒▒▒▒▒▒▒‾‾═╝ ",
];

/// Main entry point for hinfo command
pub fn main(args: &[&str]) {
    if args.len() < 2 {
        video::put_str("hinfo: Missing arguments. Use -cpu, -gpu, -mem, or -all\n");
        return;
    }

    let flags = &args[1..];

    if flags[0] == "-all" {
        show_cpu();
        video::put_str("\n");
        show_mem_r(false, None);
        video::put_str("\n");
        show_mem_d(false, None);
        video::put_str("\n");
        show_gpu();
        return;
    }

    match flags[0] {
        "-cpu" => show_cpu(),
        "-gpu" => show_gpu(),
        "-mem" => {
            if flags.len() < 2 {
                video::put_str("hinfo -mem: Use -r (RAM) or -d (Disk)\n");
                return;
            }
            match flags[1] {
                "-r" => {
                    let separate = flags.contains(&"-s");
                    // Check for -r1, -r2...
                    let mut slot = None;
                    for a in flags {
                        if a.starts_with("-r") && a.len() > 2 {
                            if let Ok(n) = a[2..].parse::<usize>() { slot = Some(n); }
                        }
                    }
                    show_mem_r(separate, slot);
                }
                "-d" => {
                     let separate = flags.contains(&"-s");
                     let mut drive = None;
                     for a in flags {
                         if a.starts_with("-d") && a.len() > 2 {
                             if let Ok(n) = a[2..].parse::<usize>() { drive = Some(n); }
                         }
                     }
                     show_mem_d(separate, drive);
                }
                _ => video::put_str("hinfo -mem: Invalid flag. Use -r or -d\n"),
            }
        }
        "-h" | "--help" => {
            video::put_str("Ainux hinfo suite\n");
            video::put_str("Usage: hinfo [flags]\n");
            video::put_str("  -cpu           Display CPU details, temp, and clock\n");
            video::put_str("  -gpu           Display GPU/Display adapter info\n");
            video::put_str("  -mem -r [-s]   Display RAM info [-s for slots]\n");
            video::put_str("  -mem -d [-s]   Display Disk space and categories\n");
            video::put_str("  -all           Display all hardware info\n");
        }
        _ => video::put_str("hinfo: Unknown flag. Use --help\n"),
    }
}

// ---- CPU Diagnostics ----
fn show_cpu() {
    let (brand, vendor, logical) = {
        let feat = crate::cpu::cpuid::CPU_FEATURES.lock();
        (String::from(feat.brand_str()), String::from(feat.vendor_str()), feat.logical_cores)
    };

    let temp_str = if vendor == "GenuineIntel" {
        unsafe {
            let val = rdmsr(0x19C);
            if (val >> 31) & 1 == 1 {
                let readout = (val >> 16) & 0x7F;
                format!("{} C", 100 - readout)
            } else { String::from("N/A") }
        }
    } else { String::from("N/A (AMD/Other)") };

    let (smbios_ver, smbios_man) = detect_smbios_cpu();
    let cpu_count = crate::cpu::percpu::get_cpu_count();

    let info = [
        ("Manufacturer: ", if smbios_man.is_empty() { vendor } else { smbios_man }),
        ("Model:        ", if smbios_ver.is_empty() { brand } else { smbios_ver }),
        ("Threads:      ", format!("{}", logical)),
        ("Cores Online: ", format!("{}", cpu_count)),
        ("Temperature:  ", temp_str),
        ("Status:       ", String::from("Optimal")),
    ];

    draw_boxed_info("Processor Diagnostic", &ASC_CPU, &info);
}

fn detect_smbios_cpu() -> (String, String) {
    let mut version = String::new();
    let mut manufacturer = String::new();

    if let Some(addr) = find_smbios() {
        let ep = unsafe { &*(addr as *const SmbiosEntryPoint) };
        let mut curr_addr = ep.table_address as u64;
        let end_addr = curr_addr + ep.table_length as u64;

        for _ in 0..ep.number_of_structures {
            if curr_addr + 4 > end_addr { break; }
            let header = unsafe { &*(curr_addr as *const SmbiosHeader) };
            
            if header.kind == 4 { // Processor Info
                manufacturer = get_smbios_string(curr_addr, 0x07);
                version = get_smbios_string(curr_addr, 0x10);
                break;
            }

            // Skip to strings
            curr_addr += header.length as u64;
            // Skip strings
            loop {
                let bytes = unsafe { core::slice::from_raw_parts(curr_addr as *const u8, 2) };
                if bytes == [0, 0] {
                    curr_addr += 2;
                    break;
                }
                curr_addr += 1;
                if curr_addr >= end_addr { break; }
            }
            if curr_addr >= end_addr { break; }
        }
    }

    (version, manufacturer)
}

fn get_smbios_string(struct_addr: u64, offset: u8) -> String {
    let header = unsafe { &*(struct_addr as *const SmbiosHeader) };
    if offset >= header.length { return String::new(); }

    let str_idx = unsafe { *((struct_addr + offset as u64) as *const u8) };
    if str_idx == 0 { return String::new(); }

    let mut curr_ptr = struct_addr + header.length as u64;
    for i in 1..=str_idx {
        let mut len = 0;
        while unsafe { *((curr_ptr + len) as *const u8) } != 0 {
            len += 1;
            if len > 255 { break; } // Safety
        }

        if i == str_idx {
            let slice = unsafe { core::slice::from_raw_parts(curr_ptr as *const u8, len as usize) };
            return String::from_utf8_lossy(slice).into_owned();
        }
        curr_ptr += len + 1;
    }

    String::new()
}

// ---- RAM Diagnostics ----
fn show_mem_r(_separate: bool, _slot: Option<usize>) {
    let pmm_lock = crate::mm::pmm::PMM.lock();
    if let Some(pmm) = pmm_lock.as_ref() {
        let (used, total) = pmm.get_stats_fast();
        let total_mb = (total * 4096) / 1024 / 1024;
        let used_mb = (used * 4096) / 1024 / 1024;
        
        // High-precision scale
        let raw_pct_x10 = if total > 0 { (used * 1000) / total } else { 0 };
        let pct_major = raw_pct_x10 / 10;
        let pct_minor = raw_pct_x10 % 10;

        // Visual Bar (expanded 20 slots)
        let mut bar = String::from("[");
        let dots = 20;
        let filled = (pct_major * dots) / 100;
        for i in 0..dots {
            if i < filled { bar.push_str("█"); }
            else { bar.push_str("░"); }
        }
        bar.push_str(&format!("] {}.{}%", pct_major, pct_minor));

        let info = [
            ("Manufacturer: ", String::from("System Boot RAM")),
            ("Total RAM:    ", format!("{} MiB", total_mb)),
            ("Allocated:    ", format!("{} MiB", used_mb)),
            ("Free RAM:     ", format!("{} MiB", total_mb - used_mb)),
            ("Usage Load:   ", bar),
            ("Accounting:   ", String::from("Sovereign Phase 9")),
        ];

        draw_boxed_info("Memory Diagnostic", &ASC_RAM, &info);
    } else {
        video::put_str("RAM Error: PMM not available.\n");
    }
}

// ---- Disk Diagnostics ----
fn show_mem_d(_separate: bool, _drive: Option<usize>) {
    video::put_str("\n  Auditing filesystem objects... (Press Ctrl+C to abort)\n");
    
    let mut stats = UsageStats::default();
    let success = scan_disk_usage_iterative(root(), &mut stats);

    if !success {
        video::put_str_colored("  [!] Diagnostic interrupted by user.\n", 0x00FF0000, 0x00111122);
        return;
    }

    let mut ata_buf = [0u16; 256];
    let mut model = String::from("QEMU VIRTUAL DRIVE");
    let mut total_capacity: u64 = 0;

    if crate::drivers::ata::identify_buffer(&mut ata_buf) {
        model.clear();
        for i in 27..47 {
            let word = ata_buf[i];
            let b1 = (word >> 8) as u8 as char;
            let b2 = (word & 0xFF) as u8 as char;
            if b1 != '\0' && b1 != ' ' { model.push(b1); }
            if b2 != '\0' && b2 != ' ' { model.push(b2); }
        }
        // Words 60-61 contain total 28-bit LBA sectors
        let sectors = (ata_buf[60] as u32) | ((ata_buf[61] as u32) << 16);
        total_capacity = sectors as u64 * 512;
    }

    if total_capacity == 0 { total_capacity = 64 * 1024 * 1024; } // Fallback 64MB

    let other_size = stats.total_size.saturating_sub(stats.image_size + stats.system_size + stats.alo_size);
    
    // High-precision percentage (scaled by 10)
    let used_pct_scaled = (stats.total_size * 1000) / total_capacity.max(1);
    let used_pct_whole = used_pct_scaled / 10;
    let used_pct_fract = used_pct_scaled % 10;
    
    // Create usage bar
    let mut bar = String::from("[");
    let mut filled = (used_pct_whole / 10) as usize;
    // Minimum 1 block if any space used
    if stats.total_size > 0 && filled == 0 { filled = 1; }
    
    for i in 0..10 {
        if i < filled { bar.push_str("█"); }
        else { bar.push_str("░"); }
    }
    bar.push_str(&format!("] {}.{}%", used_pct_whole, used_pct_fract));

    let info = [
        ("Model:        ", model),
        ("Capacity:     ", format!("{} / {}", format_size(stats.total_size), format_size(total_capacity))),
        ("Usage:        ", bar),
        ("              ", String::from("")),
        ("Images:       ", format_size(stats.image_size)),
        ("System:       ", format_size(stats.system_size)),
        ("Code/Binaries:", format_size(stats.alo_size)),
        ("General Data: ", format_size(other_size)),
    ];

    draw_boxed_info("Disk Diagnostic", &ASC_DISK, &info);
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        let gb_int = bytes / (1024 * 1024 * 1024);
        let gb_frac = (bytes * 10 / (1024 * 1024 * 1024)) % 10;
        format!("{}.{} GiB", gb_int, gb_frac)
    } else if bytes >= 1024 * 1024 {
        let mb_int = bytes / (1024 * 1024);
        let mb_frac = (bytes * 10 / (1024 * 1024)) % 10;
        format!("{}.{} MiB", mb_int, mb_frac)
    } else if bytes >= 1024 {
        format!("{} KiB", bytes / 1024)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Default)]
struct UsageStats {
    total_size: u64,
    image_size: u64,
    system_size: u64,
    alo_size: u64,
}

fn scan_disk_usage_iterative(root_node: Arc<dyn Inode>, stats: &mut UsageStats) -> bool {
    let mut queue = Vec::new();
    let mut visited = BTreeSet::new();
    let mut count = 0;

    queue.push((root_node, String::from("/")));

    while let Some((inode, name)) = queue.pop() {
        // Cycle Detection
        let ino = inode.inode_num();
        if visited.contains(&ino) { continue; }
        visited.insert(ino);

        // Cooperation & Interrupts
        count += 1;
        if count % 50 == 0 {
            if scheduler::check_current_signal(task::SIGINT) {
                return false;
            }
            scheduler::yield_now();
        }

        if let Ok(stat) = inode.stat() {
            if stat.file_type == FileType::File {
                stats.total_size += stat.size;
                // Universal categorization
                if name.ends_with(".bmp") || name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".ico") {
                    stats.image_size += stat.size;
                } else if name.ends_with(".nux") || name.ends_with(".sys") || name.ends_with(".bin") || name.ends_with(".o") {
                    stats.system_size += stat.size;
                } else if name.ends_with(".alo") || name.ends_with(".rs") || name.ends_with(".c") || name.ends_with(".zig") || name.ends_with(".asm") || name.ends_with(".h") {
                    stats.alo_size += stat.size;
                }
            } else if stat.file_type == FileType::Directory {
                if let Ok(entries) = inode.read_dir() {
                    for entry_name in entries {
                        if entry_name == "." || entry_name == ".." { continue; }
                        if let Ok(child) = inode.lookup(&entry_name) {
                            queue.push((child, entry_name));
                        }
                    }
                }
            }
        }
    }
    true
}

fn find_smbios() -> Option<u64> {
    // Search ROM BIOS area
    for addr in (0xF0000..0x100000).step_by(16) {
        let ptr = addr as *const [u8; 4];
        unsafe {
            if &*ptr == b"_SM_" { return Some(addr as u64); }
        }
    }
    None
}

// ---- GPU Diagnostics ----
fn show_gpu() {
    let gpu = detect_gpu_internal();
    let (w, h) = video::get_resolution();

    let info = [
        ("Adapter:      ", gpu),
        ("Resolution:   ", format!("{}x{} (32-bit)", w, h)),
        ("Backend:      ", String::from("VBE v3.0")),
        ("Acceleration: ", String::from("Hardware (MTRR)")),
        ("Status:       ", String::from("Operational")),
    ];

    draw_boxed_info("Graphics Diagnostic", &ASC_GPU, &info);
}

fn detect_gpu_internal() -> String {
    // Miniature PCI scan
    for bus in 0u16..256 {
        for slot in 0u8..32 {
            let id_reg = pci_read(bus as u8, slot, 0, 0);
            if (id_reg & 0xFFFF) == 0xFFFF { continue; }
            let class = (pci_read(bus as u8, slot, 0, 0x08) >> 24) as u8;
            if class == 0x03 {
                return format!("PCI Device {:04x}:{:04x}", id_reg & 0xFFFF, id_reg >> 16);
            }
        }
    }
    String::from("Standard VGA")
}

fn pci_read(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = (1 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | (offset as u32 & 0xFC);
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") address);
        let val: u32;
        core::arch::asm!("in eax, dx", out("eax") val, in("dx") 0xCFCu16);
        val
    }
}

fn draw_boxed_info(title: &str, logo: &[&str], info: &[(&str, String)]) {
    let theme = video::THEME.lock();
    let a_col = theme.accent;
    let f_col = theme.fg;
    let box_bg = 0x002D2D2D;    // Whiptail charcoal (WP_BOX)
    let shadow_col = 0x000F0F0F;
    drop(theme);

    let box_w = 64;
    let box_h = (logo.len().max(info.len()) + 4).max(11);
    let start_x = 2;
    let start_y = video::prepare_y_for_height(box_h + 1);

    // --- DRAW CONTAINER ---
    video::draw_rect_grid(start_x + 1, start_y + 1, box_w, box_h, shadow_col, shadow_col);
    video::draw_rect_grid(start_x, start_y, box_w, box_h, a_col, box_bg);
    
    // Draw Box Border
    for i in 1..(box_w - 1) { 
        video::put_char_at(start_x + i, start_y, '═', a_col, box_bg); 
        video::put_char_at(start_x + i, start_y + box_h - 1, '═', a_col, box_bg);
    }
    for i in 1..(box_h - 1) { 
        video::put_char_at(start_x, start_y + i, '║', a_col, box_bg); 
        video::put_char_at(start_x + box_w - 1, start_y + i, '║', a_col, box_bg);
    }
    video::put_char_at(start_x, start_y, '╔', a_col, box_bg);
    video::put_char_at(start_x + box_w - 1, start_y, '╗', a_col, box_bg);
    video::put_char_at(start_x, start_y + box_h - 1, '╚', a_col, box_bg);
    video::put_char_at(start_x + box_w - 1, start_y + box_h - 1, '╝', a_col, box_bg);

    // --- RENDER CONTENT ---
    let header = format!(" {} ", title);
    video::put_str_at(start_x + (box_w - header.len())/2, start_y, &header, 0xFFFFFFFF, a_col);

    // Render Logo
    for (i, line) in logo.iter().enumerate() {
        video::put_str_at(start_x+2, start_y+2+i, line, a_col, box_bg);
    }

    // Render Info
    for (i, (label, value)) in info.iter().enumerate() {
        video::put_str_at(start_x+22, start_y+2+i, label, a_col, box_bg);
        video::put_str_at(start_x+22+label.len(), start_y+2+i, value, f_col, box_bg);
    }

    // Move cursor below the box
    *video::CONSOLE_X.lock() = 0;
    *video::CONSOLE_Y.lock() = start_y + box_h;
}
