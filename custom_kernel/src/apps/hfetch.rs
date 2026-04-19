// =============================================================================
// Ainux HFetch — System Information Display
// Displays hardware info with ASCII art logo, renamed from neofetch.
// =============================================================================

extern crate alloc;
use alloc::string::String;
use alloc::format;
use crate::drivers::video;
use core::sync::atomic::Ordering;

/// PCI configuration space read (duplicated from pci.rs for standalone use)
fn pci_config_read(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    unsafe {
        let address: u32 = (1 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11)
            | ((func as u32) << 8) | ((offset as u32) & 0xFC);
        core::arch::asm!("out dx, eax", in("dx") 0xCF8u16, in("eax") address,
            options(nostack, preserves_flags));
        let val: u32;
        core::arch::asm!("in eax, dx", out("eax") val, in("dx") 0xCFCu16,
            options(nostack, preserves_flags));
        val
    }
}

/// Detect GPU by scanning PCI for class 0x03 (Display Controller)
fn detect_gpu() -> String {
    for bus in 0u16..256 {
        for slot in 0u8..32 {
            let id_reg = pci_config_read(bus as u8, slot, 0, 0);
            let vendor = (id_reg & 0xFFFF) as u16;
            if vendor == 0xFFFF { continue; }

            let class_rev = pci_config_read(bus as u8, slot, 0, 0x08);
            let class_id = (class_rev >> 24) as u8;

            if class_id == 0x03 { // Display controller
                let device_id = (id_reg >> 16) as u16;
                let subclass = ((class_rev >> 16) & 0xFF) as u8;

                let vendor_name = match vendor {
                    0x10DE => "NVIDIA Corporation",
                    0x1002 => "AMD/ATI",
                    0x8086 => "Intel Corporation",
                    0x1234 => "QEMU/Bochs",
                    0x15AD => "VMware",
                    0x1AF4 => "VirtIO",
                    0x1B36 => "Red Hat",
                    _ => "Unknown Vendor",
                };

                let subclass_name = match subclass {
                    0x00 => "VGA Controller",
                    0x01 => "XGA Controller",
                    0x02 => "3D Controller",
                    0x80 => "Display Controller",
                    _ => "GPU",
                };

                return format!("{} {} [{:04x}:{:04x}]",
                    vendor_name, subclass_name, vendor, device_id);
            }
        }
    }
    String::from("None Detected")
}

fn detect_ram_mb() -> usize {
    let pmm_lock = crate::mm::pmm::PMM.lock();
    if let Some(pmm) = pmm_lock.as_ref() {
        let (_, total) = pmm.get_stats();
        return (total * 4096) / 1024 / 1024;
    }
    0
}

fn detect_used_ram_mb() -> usize {
    let pmm_lock = crate::mm::pmm::PMM.lock();
    if let Some(pmm) = pmm_lock.as_ref() {
        let (used, _) = pmm.get_stats();
        return (used * 4096) / 1024 / 1024;
    }
    0
}

fn detect_resolution() -> String {
    let (w, h) = crate::drivers::video::get_resolution();
    if w > 0 && h > 0 {
        format!("{}x{}", w, h)
    } else {
        String::from("Text Mode")
    }
}

fn get_uptime_str() -> String {
    let ticks = crate::process::scheduler::get_ticks();
    let total_secs = ticks / 100;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Main hfetch display
pub fn cmd_hfetch(args: &[&str]) {
    // If subcommands are provided (e.g. hfetch -mem -r), delegate to hinfo logic
    if args.len() > 1 {
        crate::apps::hinfo::main(args);
        return;
    }

    let (cpu_brand, cpu_vendor, cpu_cores, feat_str) = {
        let features = crate::cpu::cpuid::CPU_FEATURES.lock();
        let brand = String::from(features.brand_str());
        let vendor = String::from(features.vendor_str());
        let cores = features.logical_cores;

        let mut feat = String::new();
        if features.has_sse    { feat.push_str("SSE "); }
        if features.has_sse2   { feat.push_str("SSE2 "); }
        if features.has_sse3   { feat.push_str("SSE3 "); }
        if features.has_sse41  { feat.push_str("SSE4.1 "); }
        if features.has_sse42  { feat.push_str("SSE4.2 "); }
        if features.has_avx    { feat.push_str("AVX "); }
        if features.has_aes    { feat.push_str("AES "); }
        if features.has_nx     { feat.push_str("NX "); }
        if features.has_x2apic { feat.push_str("x2APIC "); }

        (brand, vendor, cores, feat)
    };

    let gpu = detect_gpu();
    let total_ram = detect_ram_mb();
    let used_ram = detect_used_ram_mb();
    let resolution = detect_resolution();
    let uptime = get_uptime_str();
    let cpu_count = crate::cpu::percpu::get_cpu_count();

    let logo: [&str; 16] = [
        "                                ",
        " ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ ",
        " ▓▓▓▓▓▓▓▓▓▓▓▓▓▒░░▓▓▓▓▓▓▓▓▓▓▓▓▓▓ ",
        " ▓▓▓▓▓▓▓▓▓▓▓▓▒░░░░▒▓▓▓▓▓▓▓▓▓▓▓▓ ",
        " ▓▓▓▓▓▓▓▓▓▓▓▒░░▓▓░░▒▓▓▓▓▓▓▓▓▓▓▓ ",
        " ▓▓▓▓▓▓▓▓▓▓▒░▒▒==▒▒░▒▓▓▓▓▓▓▓▓▓▓ ",
        " ▓▓▓▓▓▓▓▓▓▒▒▒▓▒+=▓▓▒▒▒▓▓▓▓▓▓▓▓▓ ",
        " ▓▓▓▓▓▓▓▓▒▒▒▓▓=++≡▒▒▒≡≡▓▓▓▓▓▓▓▓ ",
        " ▓▓▓▓▓▓▓▒▒▒▒▒=+▒▒:===≡▒▒▓▓▓▓▓▓▓ ",
        " ▓▓▓▓▓▒░░░≡≡≡≡░≡===░▒▓▓▒▒▒▓▓▓▓▓ ",
        " ▓▓▓▓▓▒▒▒▒░≡≡≡≡░▒▒▓▒▒▒▓▓▒▒▒▓▓▓▓ ",
        " ▓▓▓▓▓▓▓▒░░▒▓▓▓▓▒▒▒▓▒▒▒▓▓▒▒▒▓▓▓ ",
        " ▓▓▓▓▓▓▓▓▒▒▓▒▒▒▒▒▒▒▒▓▒▒▒▒▒▒░▒▓▓ ",
        " ▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒▒▒▒▒▓▓▓▓▓▓▓▒▓▓ ",
        "                                ",
        "                                ",
    ];

    let theme = video::THEME.lock();
    let r_col = theme.root;
    let a_col = 0x0000AAAA; // Industrial Cyan
    let f_col = theme.fg;
    let b_col = 0x00111122; // Midnight blue
    drop(theme);

    // --- RENDER HEADER ---
    video::put_str("\n");
    video::put_str_colored(logo[0], a_col, b_col); 
    video::put_str_colored(" root", r_col, b_col);
    video::put_str_colored("@", f_col, b_col);
    video::put_str_colored("ainux", f_col, b_col);
    video::put_str_colored(" (Sovereign HFetch)\n", a_col, b_col);

    // --- RENDER ROWS (SEQUENTIAL) ---
    // Mapping of logo line index to (Label, Value)
    let info = [
        ("───────────────────────────────", ""),                 // 1
        ("OS:         ", "Ainux v0.2 x86_64"),                    // 2
        ("Kernel:     ", "Ainux Sovereign Kernel"),               // 3
        ("Uptime:     ", &uptime),                                // 4
        ("Shell:      ", "ainux-sh (sovereign)"),                 // 5
        ("CPU:        ", &cpu_brand),                             // 6
        ("Cores:      ", &format!("{} ({} online)", cpu_cores, cpu_count)), // 7
        ("GPU:        ", &gpu),                                   // 8
        ("Memory:     ", &format!("{} / {} MiB", used_ram, total_ram)), // 9
        ("Resolution: ", &resolution),                            // 10
        ("Features:   ", feat_str.trim()),                        // 11
        ("Arch:       ", &format!("AMD64 ({})", cpu_vendor)),      // 12
    ];

    for i in 1..=12 {
        video::put_str_colored(logo[i], a_col, b_col);
        if info[i-1].1.is_empty() {
             video::put_str_colored(info[i-1].0, a_col, b_col);
        } else {
             video::put_str_colored(info[i-1].0, a_col, b_col);
             video::put_str_colored(info[i-1].1, f_col, b_col);
        }
        video::put_str("\n");
    }

    // --- RENDER FOOTER ---
    video::put_str_colored(logo[13], a_col, b_col); video::put_str("\n");
    video::put_str_colored(logo[14], a_col, b_col); video::put_str_colored("████████████████████████████\n", a_col, b_col);
    video::put_str_colored(logo[15], a_col, b_col); video::put_str_colored("████████████████████████████\n", a_col, b_col);
    video::put_str("\n");
}
