// =============================================================================
// Ainux ACPI Subsystem — Ported from legacy C to Rust
// Parses RSDP → RSDT → MADT for SMP, FADT for shutdown
// =============================================================================

use core::fmt::Write;
use spin::Mutex;

// ---- ACPI Table Structures (packed, matching hardware layout) ----

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct AcpiHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct Rsdp {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub revision: u8,
    pub rsdt_address: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct Fadt {
    pub header: AcpiHeader,
    pub firmware_ctrl: u32,
    pub dsdt: u32,
    pub _reserved: u8,
    pub preferred_pm_profile: u8,
    pub sci_interrupt: u16,
    pub smi_command_port: u32,
    pub acpi_enable: u8,
    pub acpi_disable: u8,
    pub s4bios_req: u8,
    pub pstate_cnt: u8,
    pub pm1a_event_block: u32,
    pub pm1b_event_block: u32,
    pub pm1a_control_block: u32,
    pub pm1b_control_block: u32,
    pub pm2_control_block: u32,
    pub pm_timer_block: u32,
    pub gpe0_block: u32,
    pub gpe1_block: u32,
    pub pm1_event_length: u8,
    pub pm1_control_length: u8,
    pub pm2_control_length: u8,
    pub pm_timer_length: u8,
    pub gpe0_length: u8,
    pub gpe1_length: u8,
    pub gpe1_base: u8,
    pub cstate_control: u8,
    pub worst_c2_latency: u16,
    pub worst_c3_latency: u16,
    pub flush_size: u16,
    pub flush_stride: u16,
    pub duty_offset: u8,
    pub duty_width: u8,
    pub day_alarm: u8,
    pub month_alarm: u8,
    pub century: u8,
    pub boot_architecture_flags: u16,
    pub _reserved2: u8,
    pub flags: u32,
}

// ---- TPM2 (Trusted Platform Module) ----

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct Tpm2Table {
    pub header: AcpiHeader,
    pub flags: u16,
    pub control_area_addr: u64,
    pub start_method: u32,
}

// ---- MADT (Multiple APIC Description Table) ----

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct Madt {
    pub header: AcpiHeader,
    pub local_apic_addr: u32,
    pub flags: u32,
    // Variable-length entries follow
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct MadtEntryHeader {
    pub entry_type: u8,
    pub length: u8,
}

// MADT Entry Type 0: Processor Local APIC
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct MadtLocalApic {
    pub header: MadtEntryHeader,
    pub acpi_processor_id: u8,
    pub apic_id: u8,
    pub flags: u32,
}

// MADT Entry Type 1: I/O APIC
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct MadtIoApic {
    pub header: MadtEntryHeader,
    pub io_apic_id: u8,
    pub _reserved: u8,
    pub io_apic_addr: u32,
    pub global_system_interrupt_base: u32,
}

// MADT Entry Type 2: Interrupt Source Override
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct MadtIso {
    pub header: MadtEntryHeader,
    pub bus_source: u8,
    pub irq_source: u8,
    pub global_system_interrupt: u32,
    pub flags: u16,
}

// ---- ACPI State ----

#[derive(Clone, Copy, Debug)]
pub struct CpuInfo {
    pub apic_id: u8,
    pub acpi_id: u8,
    pub enabled: bool,
    pub is_bsp: bool,
}

pub struct AcpiState {
    pub cpu_count: usize,
    pub cpus: [CpuInfo; 256],
    pub local_apic_addr: u64,
    pub io_apic_addr: u64,
    pub io_apic_id: u8,
    pub pm1a_control_block: u32,
    pub pm1b_control_block: u32,
    pub slp_typa: u16,
    pub slp_typb: u16,
    pub smi_cmd: u32,
    pub acpi_enable_val: u8,
    pub shutdown_ready: bool,
    pub tpm2_addr: u64,
}

impl AcpiState {
    pub const fn new() -> Self {
        AcpiState {
            cpu_count: 0,
            cpus: [CpuInfo { apic_id: 0, acpi_id: 0, enabled: false, is_bsp: false }; 256],
            local_apic_addr: 0xFEE00000, // Default
            io_apic_addr: 0,
            io_apic_id: 0,
            pm1a_control_block: 0,
            pm1b_control_block: 0,
            slp_typa: 0,
            slp_typb: 0,
            smi_cmd: 0,
            acpi_enable_val: 0,
            shutdown_ready: false,
            tpm2_addr: 0,
        }
    }
}

pub static ACPI: Mutex<AcpiState> = Mutex::new(AcpiState::new());

// ---- RSDP Scanner ----

/// Scan the BIOS data areas for the RSDP signature "RSD PTR "
unsafe fn find_rsdp() -> Option<*const Rsdp> {
    let signature = b"RSD PTR ";

    // Search EBDA (Extended BIOS Data Area) — first KB at segment address from 0x040E
    let ebda_seg = *(0x040E as *const u16) as usize;
    let ebda_base = ebda_seg << 4;
    if ebda_base > 0 && ebda_base < 0xA0000 {
        for offset in (0..1024).step_by(16) {
            let ptr = (ebda_base + offset) as *const [u8; 8];
            if &*ptr == signature {
                return Some((ebda_base + offset) as *const Rsdp);
            }
        }
    }

    // Search main BIOS area: 0xE0000 - 0xFFFFF
    for addr in (0xE0000..0x100000).step_by(16) {
        let ptr = addr as *const [u8; 8];
        if &*ptr == signature {
            // Verify checksum
            let bytes = core::slice::from_raw_parts(addr as *const u8, 20);
            let sum: u8 = bytes.iter().fold(0u8, |a, b| a.wrapping_add(*b));
            if sum == 0 {
                return Some(addr as *const Rsdp);
            }
        }
    }

    None
}

/// Find a table in the RSDT by its 4-byte signature
unsafe fn find_table(rsdt: *const AcpiHeader, sig: &[u8; 4]) -> Option<*const AcpiHeader> {
    let rsdt_len = core::ptr::read_unaligned(core::ptr::addr_of!((*rsdt).length));
    let entry_count = (rsdt_len as usize - core::mem::size_of::<AcpiHeader>()) / 4;
    let entries = (rsdt as usize + core::mem::size_of::<AcpiHeader>()) as *const u32;

    for i in 0..entry_count {
        let table_phys = *entries.add(i) as usize;
        if table_phys == 0 { continue; }
        let table = table_phys as *const AcpiHeader;
        if &(*table).signature == sig {
            return Some(table);
        }
    }
    None
}

// ---- DSDT _S5_ Parser (for shutdown) ----

unsafe fn parse_dsdt_s5(dsdt: *const AcpiHeader) -> Option<(u16, u16)> {
    let dsdt_len = core::ptr::read_unaligned(core::ptr::addr_of!((*dsdt).length));
    let data_start = (dsdt as usize + core::mem::size_of::<AcpiHeader>()) as *const u8;
    let data_len = dsdt_len as usize - core::mem::size_of::<AcpiHeader>();

    // Scan for "_S5_" byte pattern
    for i in 0..(data_len.saturating_sub(12)) {
        let p = data_start.add(i);
        if *p == b'_' && *p.add(1) == b'S' && *p.add(2) == b'5' && *p.add(3) == b'_' {
            // Find PackageOp (0x12) within the next 16 bytes
            let mut pp = p.add(4);
            let mut found_pkg = false;
            for k in 0..16 {
                if *pp.add(k) == 0x12 {
                    pp = pp.add(k);
                    found_pkg = true;
                    break;
                }
            }
            if !found_pkg { continue; }

            pp = pp.add(1); // Skip 0x12

            // Parse PkgLength
            let b0 = *pp;
            let bytes_read = if (b0 & 0xC0) == 0 { 1 } else if (b0 & 0xC0) == 0x40 { 2 } else { 1 };
            pp = pp.add(bytes_read);
            pp = pp.add(1); // NumElements

            // SLP_TYPa
            if *pp == 0x0A { pp = pp.add(1); }
            let slp_typa = (*pp as u16) << 10;
            pp = pp.add(1);

            // SLP_TYPb
            if *pp == 0x0A { pp = pp.add(1); }
            let slp_typb = (*pp as u16) << 10;

            return Some((slp_typa, slp_typb));
        }
    }
    None
}

// ---- Public API ----

/// Initialize ACPI subsystem. Scans for RSDP, parses RSDT, MADT, FADT.
pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "ACPI: Scanning for RSDP...\n");

    unsafe {
        let rsdp_ptr = match find_rsdp() {
            Some(p) => p,
            None => {
                let _ = write!(serial, "ACPI: RSDP not found! SMP disabled.\n");
                return;
            }
        };

        let rsdt_addr = core::ptr::read_unaligned(core::ptr::addr_of!((*rsdp_ptr).rsdt_address));
        let _ = write!(serial, "ACPI: RSDP found at {:#x}, RSDT at {:#x}\n",
            rsdp_ptr as usize, rsdt_addr);

        let rsdt = rsdt_addr as *const AcpiHeader;
        if rsdt.is_null() {
            let _ = write!(serial, "ACPI: RSDT address is NULL!\n");
            return;
        }

        // ---- Parse MADT for CPU enumeration ----
        if let Some(madt_ptr) = find_table(rsdt, b"APIC") {
            let _ = write!(serial, "ACPI: MADT found at {:#x}\n", madt_ptr as usize);
            parse_madt(madt_ptr as *const Madt, &mut serial);
        } else {
            let _ = write!(serial, "ACPI: MADT not found. Assuming single CPU.\n");
            let mut state = ACPI.lock();
            state.cpu_count = 1;
            state.cpus[0] = CpuInfo { apic_id: 0, acpi_id: 0, enabled: true, is_bsp: true };
        }

        // ---- Parse FADT for power management ----
        if let Some(fadt_ptr) = find_table(rsdt, b"FACP") {
            let _ = write!(serial, "ACPI: FADT found at {:#x}\n", fadt_ptr as usize);
            let fadt = fadt_ptr as *const Fadt;

            let mut state = ACPI.lock();
            state.pm1a_control_block = core::ptr::read_unaligned(core::ptr::addr_of!((*fadt).pm1a_control_block));
            state.pm1b_control_block = core::ptr::read_unaligned(core::ptr::addr_of!((*fadt).pm1b_control_block));
            state.smi_cmd = core::ptr::read_unaligned(core::ptr::addr_of!((*fadt).smi_command_port));
            state.acpi_enable_val = core::ptr::read_unaligned(core::ptr::addr_of!((*fadt).acpi_enable));

            // Parse DSDT for _S5_ shutdown values
            let dsdt_addr = core::ptr::read_unaligned(core::ptr::addr_of!((*fadt).dsdt));
            if dsdt_addr != 0 {
                let dsdt = dsdt_addr as *const AcpiHeader;
                if let Some((slp_a, slp_b)) = parse_dsdt_s5(dsdt) {
                    state.slp_typa = slp_a;
                    state.slp_typb = slp_b;
                    state.shutdown_ready = true;
                    let _ = write!(serial, "ACPI: Shutdown ready (SLP_TYPa={:#x})\n", slp_a);
                } else {
                    let _ = write!(serial, "ACPI: _S5_ not found in DSDT\n");
                }
            }
        }

        // ---- Parse TPM2 for Hardware Trust Module ----
        if let Some(tpm2_ptr) = find_table(rsdt, b"TPM2") {
            let tpm2 = tpm2_ptr as *const Tpm2Table;
            let caa = core::ptr::read_unaligned(core::ptr::addr_of!((*tpm2).control_area_addr));
            let _ = write!(serial, "ACPI: TPM 2.0 Hardware Trust Module detected at {:#x}\n", tpm2_ptr as usize);
            let _ = write!(serial, "ACPI: TPM 2.0 Control Area Address: {:#x}\n", caa);
            
            let mut state = ACPI.lock();
            state.tpm2_addr = tpm2_ptr as u64;
        } else {
            let _ = write!(serial, "ACPI: TPM 2.0 not found.\n");
        }

        let state = ACPI.lock();
        let _ = write!(serial, "ACPI: Detected {} CPU(s), LAPIC at {:#x}\n",
            state.cpu_count, state.local_apic_addr);
    }
}

unsafe fn parse_madt(madt: *const Madt, serial: &mut crate::drivers::serial::SerialPort) {
    let madt_lapic_addr = core::ptr::read_unaligned(core::ptr::addr_of!((*madt).local_apic_addr));
    let mut state = ACPI.lock();
    state.local_apic_addr = madt_lapic_addr as u64;
    let _ = write!(serial, "ACPI: Local APIC address: {:#x}\n", state.local_apic_addr);

    let data_start = (madt as usize) + core::mem::size_of::<Madt>();
    let madt_header_len = core::ptr::read_unaligned(core::ptr::addr_of!((*madt).header.length));
    let data_end = (madt as usize) + madt_header_len as usize;
    let mut offset = data_start;

    while offset + 2 <= data_end {
        let entry = &*(offset as *const MadtEntryHeader);
        if entry.length == 0 { break; }

        match entry.entry_type {
            0 => {
                // Processor Local APIC
                let lapic = offset as *const MadtLocalApic;
                let lapic_flags = core::ptr::read_unaligned(core::ptr::addr_of!((*lapic).flags));
                let lapic_apic_id = core::ptr::read_unaligned(core::ptr::addr_of!((*lapic).apic_id));
                let lapic_acpi_id = core::ptr::read_unaligned(core::ptr::addr_of!((*lapic).acpi_processor_id));
                let enabled = (lapic_flags & 1) != 0 || (lapic_flags & 2) != 0;
                if enabled {
                    let idx = state.cpu_count;
                    if idx < 256 {
                        state.cpus[idx] = CpuInfo {
                            apic_id: lapic_apic_id,
                            acpi_id: lapic_acpi_id,
                            enabled: true,
                            is_bsp: idx == 0,
                        };
                        state.cpu_count += 1;
                        let _ = write!(serial, "ACPI:   CPU {} — APIC ID {}, ACPI ID {}\n",
                            idx, lapic_apic_id, lapic_acpi_id);
                    }
                }
            }
            1 => {
                // I/O APIC
                let ioapic = offset as *const MadtIoApic;
                let ioapic_addr = core::ptr::read_unaligned(core::ptr::addr_of!((*ioapic).io_apic_addr));
                let ioapic_id = core::ptr::read_unaligned(core::ptr::addr_of!((*ioapic).io_apic_id));
                state.io_apic_addr = ioapic_addr as u64;
                state.io_apic_id = ioapic_id;
                let _ = write!(serial, "ACPI:   I/O APIC ID {} at {:#x}\n",
                    ioapic_id, ioapic_addr);
            }
            2 => {
                // Interrupt Source Override
                let iso = offset as *const MadtIso;
                let irq_src = core::ptr::read_unaligned(core::ptr::addr_of!((*iso).irq_source));
                let gsi = core::ptr::read_unaligned(core::ptr::addr_of!((*iso).global_system_interrupt));
                let _ = write!(serial, "ACPI:   ISO: IRQ {} → GSI {}\n",
                    irq_src, gsi);
            }
            _ => {
                let _ = write!(serial, "ACPI:   MADT entry type {} len {}\n",
                    entry.entry_type, entry.length);
            }
        }

        offset += entry.length as usize;
    }

    if state.cpu_count == 0 {
        // Fallback: at least the BSP exists
        state.cpu_count = 1;
        state.cpus[0] = CpuInfo { apic_id: 0, acpi_id: 0, enabled: true, is_bsp: true };
    }
}

/// ACPI shutdown (ported from legacy C acpi_power_off)
pub fn power_off() {
    let state = ACPI.lock();
    if !state.shutdown_ready {
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        let _ = write!(serial, "ACPI: Shutdown not available\n");
        return;
    }

    let pm1a = state.pm1a_control_block;
    let pm1b = state.pm1b_control_block;
    let slp_a = state.slp_typa;
    let slp_b = state.slp_typb;
    drop(state); // Release lock before I/O

    unsafe {
        // ---- Fallback 1: QEMU / Bochs / VirtualBox specific IO port ----
        // This is a common "exit" port for emulators to quit immediately.
        core::arch::asm!("out dx, ax", in("dx") 0x604u16, in("ax") 0x2000u16);
        
        // ---- Main: ACPI S5 Transition ----
        // Write SLP_TYP | SLP_EN (bit 13)
        let val_a = slp_a | 0x2000;
        core::arch::asm!("out dx, ax", in("dx") pm1a as u16, in("ax") val_a);

        if pm1b != 0 {
            let val_b = slp_b | 0x2000;
            core::arch::asm!("out dx, ax", in("dx") pm1b as u16, in("ax") val_b);
        }

        // ---- Final Fallback: Halt loop ----
        loop {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}

/// Get the current battery percentage (0-100).
/// Currently a mock implementation as DSDT AML parsing is not yet implemented.
pub fn get_battery_percentage() -> u8 {
    // In a real OS, we would execute the _BST (Battery Status) method 
    // inside the DSDT using an AML interpreter. 
    100 // Simulated: 100%
}

/// Get the current CPU temperature in Celsius.
/// Currently a mock implementation.
pub fn get_temperature() -> u8 {
    // In a real OS, we would execute the _TMP (Temperature) method 
    // inside the DSDT thermal zone using an AML interpreter.
    45 // Simulated: 45°C
}
