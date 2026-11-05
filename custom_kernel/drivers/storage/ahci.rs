// =============================================================================
// Advanced Host Controller Interface (AHCI / SATA Revision 3.x) Driver
// PCI Class 0x01 Subclass 0x06 Prog-IF 0x01.
// Supports modern Serial ATA HDDs and SSDs (up to 6 Gbps).
// =============================================================================

use core::arch::asm;
use spin::Mutex;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::fmt::Write;

// AHCI Generic Host Control Registers (BAR5 / ABAR)
const REG_CAP:  usize = 0x00; // Host Capabilities
const REG_GHC:  usize = 0x04; // Global Host Control
const REG_IS:   usize = 0x08; // Interrupt Status
const REG_PI:   usize = 0x0C; // Ports Implemented
const REG_VS:   usize = 0x10; // Version
const REG_CCC_CTL: usize = 0x14; // Command Completion Coalescing Control
const REG_CAP2: usize = 0x24; // Extended Capabilities

// GHC Bits
const GHC_HR: u32 = 1 << 0;  // HBA Reset
const GHC_IE: u32 = 1 << 1;  // Interrupt Enable
const GHC_AE: u32 = 1 << 31; // AHCI Enable

// Port Registers Offsets (relative to ABAR + 0x100 + port * 0x80)
const PORT_BASE:   usize = 0x100;
const PORT_STRIDE: usize = 0x80;

const PORT_CLB:   usize = 0x00; // Command List Base (Low)
const PORT_CLBU:  usize = 0x04; // Command List Base (High)
const PORT_FB:    usize = 0x08; // Received FIS Base (Low)
const PORT_FBU:   usize = 0x0C; // Received FIS Base (High)
const PORT_IS:    usize = 0x10; // Interrupt Status
const PORT_IE:    usize = 0x14; // Interrupt Enable
const PORT_CMD:   usize = 0x18; // Command and Status
const PORT_TFD:   usize = 0x20; // Task File Data
const PORT_SIG:   usize = 0x24; // Signature
const PORT_SSTS:  usize = 0x28; // SATA Status
const PORT_SCTL:  usize = 0x2C; // SATA Control
const PORT_SERR:  usize = 0x30; // SATA Error
const PORT_SACT:  usize = 0x34; // SATA Active
const PORT_CI:    usize = 0x38; // Command Issue

// PORT_CMD Bits
const CMD_ST:  u32 = 1 << 0;  // Start Processing
const CMD_SUD: u32 = 1 << 1;  // Spin-Up Device
const CMD_POD: u32 = 1 << 2;  // Power On Device
const CMD_FRE: u32 = 1 << 4;  // FIS Receive Enable
const CMD_FR:  u32 = 1 << 14; // FIS Receive Running
const CMD_CR:  u32 = 1 << 15; // Command List Running

// Device Signatures
const SATA_SIG_ATA:   u32 = 0x00000101; // SATA drive
const SATA_SIG_ATAPI: u32 = 0xEB140101; // SATAPI optical drive
const SATA_SIG_SEMB:  u32 = 0xC33C0101; // Enclosure management bridge
const SATA_SIG_PM:    u32 = 0x96690101; // Port multiplier

// ATA Commands
const ATA_CMD_IDENTIFY: u8 = 0xEC;
const ATA_CMD_READ_DMA_EXT: u8 = 0x25;
const ATA_CMD_WRITE_DMA_EXT: u8 = 0x35;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct AhciCommandHeader {
    pub flags: u16,      // CFL (0..4), A (5), W (6), P (7), R (8), B (9), C (10), PMP (12..15)
    pub prdtl: u16,      // Physical Region Descriptor Table Length
    pub prdbc: u32,      // Transferred Byte Count
    pub ctba: u32,       // Command Table Descriptor Base Address Low
    pub ctbau: u32,      // Command Table Descriptor Base Address High
    pub reserved: [u32; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct AhciPrdtEntry {
    pub dba: u32,        // Data Base Address Low
    pub dbau: u32,       // Data Base Address High
    pub reserved: u32,
    pub dbc: u32,        // Byte Count (0-based) | Interrupt on Completion (bit 31)
}

#[repr(C, align(1024))]
struct AhciPortMemory {
    cmd_headers: [AhciCommandHeader; 32],
    rx_fis: [u8; 256],
    cmd_table: [u8; 512],
    bounce_buf: [u8; 4096],
}

static mut PORT_MEM: [AhciPortMemory; 4] = [
    AhciPortMemory { cmd_headers: [AhciCommandHeader { flags: 0, prdtl: 0, prdbc: 0, ctba: 0, ctbau: 0, reserved: [0; 4] }; 32], rx_fis: [0; 256], cmd_table: [0; 512], bounce_buf: [0; 4096] },
    AhciPortMemory { cmd_headers: [AhciCommandHeader { flags: 0, prdtl: 0, prdbc: 0, ctba: 0, ctbau: 0, reserved: [0; 4] }; 32], rx_fis: [0; 256], cmd_table: [0; 512], bounce_buf: [0; 4096] },
    AhciPortMemory { cmd_headers: [AhciCommandHeader { flags: 0, prdtl: 0, prdbc: 0, ctba: 0, ctbau: 0, reserved: [0; 4] }; 32], rx_fis: [0; 256], cmd_table: [0; 512], bounce_buf: [0; 4096] },
    AhciPortMemory { cmd_headers: [AhciCommandHeader { flags: 0, prdtl: 0, prdbc: 0, ctba: 0, ctbau: 0, reserved: [0; 4] }; 32], rx_fis: [0; 256], cmd_table: [0; 512], bounce_buf: [0; 4096] },
];

pub struct SataDriveInfo {
    pub port: usize,
    pub model: String,
    pub serial: String,
    pub total_sectors: u64,
    pub capacity_mb: u64,
    pub is_ssd: bool,
}

pub struct AhciController {
    pub abar: usize,
    pub ports_implemented: u32,
    pub drives: Vec<SataDriveInfo>,
    pub initialized: bool,
}

pub static ACTIVE_AHCI: Mutex<Option<AhciController>> = Mutex::new(None);

impl AhciController {
    pub fn new(abar: usize) -> Self {
        Self {
            abar,
            ports_implemented: 0,
            drives: Vec::new(),
            initialized: false,
        }
    }

    #[inline]
    unsafe fn read32(&self, offset: usize) -> u32 {
        core::ptr::read_volatile((self.abar + offset) as *const u32)
    }

    #[inline]
    unsafe fn write32(&self, offset: usize, val: u32) {
        core::ptr::write_volatile((self.abar + offset) as *mut u32, val);
    }

    #[inline]
    unsafe fn read_port(&self, port: usize, offset: usize) -> u32 {
        core::ptr::read_volatile((self.abar + PORT_BASE + port * PORT_STRIDE + offset) as *const u32)
    }

    #[inline]
    unsafe fn write_port(&self, port: usize, offset: usize, val: u32) {
        core::ptr::write_volatile((self.abar + PORT_BASE + port * PORT_STRIDE + offset) as *mut u32, val);
    }

    pub fn init(&mut self) -> bool {
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        let _ = write!(serial, "AHCI: Initializing SATA Controller at ABAR {:#x}\n", self.abar);

        unsafe {
            // Enable AHCI mode in GHC (bit 31)
            let ghc = self.read32(REG_GHC);
            self.write32(REG_GHC, ghc | GHC_AE);

            let vs = self.read32(REG_VS);
            let major = (vs >> 16) as u16;
            let minor = (vs & 0xFFFF) as u16;
            let cap = self.read32(REG_CAP);
            let num_ports = (cap & 0x1F) + 1;
            let s6g = (cap & (1 << 19)) != 0; // Supports 6 Gbps Gen3

            self.ports_implemented = self.read32(REG_PI);

            let _ = write!(
                serial,
                "AHCI: Spec v{}.{:02} | Max Ports: {} | Implemented Mask: {:#x} | 6Gbps SATA3: {}\n",
                major, minor, num_ports, self.ports_implemented, s6g
            );

            // Scan each implemented port
            for p in 0..32 {
                if (self.ports_implemented >> p) & 1 == 1 {
                    if let Some(drive) = self.init_port(p) {
                        self.drives.push(drive);
                    }
                }
            }
        }

        self.initialized = true;
        let _ = write!(serial, "AHCI: Initialization complete. Found {} SATA drive(s).\n", self.drives.len());
        true
    }

    unsafe fn stop_port(&self, port: usize) {
        let mut cmd = self.read_port(port, PORT_CMD);
        cmd &= !CMD_ST;
        cmd &= !CMD_FRE;
        self.write_port(port, PORT_CMD, cmd);

        // Wait until CR and FR clear
        for _ in 0..50_000 {
            let status = self.read_port(port, PORT_CMD);
            if (status & (CMD_CR | CMD_FR)) == 0 {
                break;
            }
            core::hint::spin_loop();
        }
    }

    unsafe fn start_port(&self, port: usize) {
        // Wait until CR clears
        for _ in 0..50_000 {
            if (self.read_port(port, PORT_CMD) & CMD_CR) == 0 {
                break;
            }
            core::hint::spin_loop();
        }

        let mut cmd = self.read_port(port, PORT_CMD);
        cmd |= CMD_FRE;
        cmd |= CMD_ST;
        self.write_port(port, PORT_CMD, cmd);
    }

    unsafe fn init_port(&self, port: usize) -> Option<SataDriveInfo> {
        if port >= 4 {
            return None; // Limit static buffers to first 4 ports
        }

        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        let ssts = self.read_port(port, PORT_SSTS);
        let det = ssts & 0x0F;
        let ipm = (ssts >> 8) & 0x0F;

        // Check if device present and Phy communication established (DET = 3, IPM = 1)
        if det != 3 || ipm != 1 {
            return None;
        }

        let sig = self.read_port(port, PORT_SIG);
        let _ = write!(serial, "AHCI: Port {} active (SIG: {:#x})...\n", port, sig);

        if sig != SATA_SIG_ATA && sig != 0 {
            // Non-ATA (e.g. ATAPI CDROM or SEMB)
            return None;
        }

        // Stop port before programming DMA base registers
        self.stop_port(port);

        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let mem = &mut PORT_MEM[port];

        let clb_phys = (mem.cmd_headers.as_ptr() as u64).wrapping_sub(hhdm);
        let fb_phys = (mem.rx_fis.as_ptr() as u64).wrapping_sub(hhdm);
        let ct_phys = (mem.cmd_table.as_ptr() as u64).wrapping_sub(hhdm);
        let buf_phys = (mem.bounce_buf.as_ptr() as u64).wrapping_sub(hhdm);

        self.write_port(port, PORT_CLB, (clb_phys & 0xFFFFFFFF) as u32);
        self.write_port(port, PORT_CLBU, (clb_phys >> 32) as u32);
        self.write_port(port, PORT_FB, (fb_phys & 0xFFFFFFFF) as u32);
        self.write_port(port, PORT_FBU, (fb_phys >> 32) as u32);

        // Clear error & interrupt status
        self.write_port(port, PORT_SERR, 0xFFFFFFFF);
        self.write_port(port, PORT_IS, 0xFFFFFFFF);

        // Start port
        self.start_port(port);

        // Prepare ATA IDENTIFY command
        // Zero command table and header
        core::ptr::write_bytes(mem.cmd_table.as_mut_ptr(), 0, 512);
        core::ptr::write_bytes(mem.bounce_buf.as_mut_ptr(), 0, 512);

        // Setup Command Header 0
        mem.cmd_headers[0].flags = 5; // CFL = 5 dwords (H2D FIS length), Read (W=0)
        mem.cmd_headers[0].prdtl = 1; // 1 PRDT entry
        mem.cmd_headers[0].ctba = (ct_phys & 0xFFFFFFFF) as u32;
        mem.cmd_headers[0].ctbau = (ct_phys >> 32) as u32;

        // Setup PRDT Entry at offset 0x80 inside cmd_table
        let prdt_ptr = (mem.cmd_table.as_mut_ptr().add(0x80)) as *mut AhciPrdtEntry;
        (*prdt_ptr).dba = (buf_phys & 0xFFFFFFFF) as u32;
        (*prdt_ptr).dbau = (buf_phys >> 32) as u32;
        (*prdt_ptr).dbc = 511 | (1 << 31); // 512 bytes, Interrupt on completion

        // Setup H2D FIS at start of cmd_table (Type 0x27)
        let fis = mem.cmd_table.as_mut_ptr();
        *fis.add(0) = 0x27; // FIS Type: Host to Device
        *fis.add(1) = 0x80; // Command bit set
        *fis.add(2) = ATA_CMD_IDENTIFY; // Command: IDENTIFY
        *fis.add(7) = 0x00; // Device register

        // Issue command slot 0
        self.write_port(port, PORT_CI, 1 << 0);

        // Wait for command completion
        let mut completed = false;
        for _ in 0..100_000 {
            if (self.read_port(port, PORT_CI) & (1 << 0)) == 0 {
                completed = true;
                break;
            }
            core::hint::spin_loop();
        }

        if !completed {
            let _ = write!(serial, "AHCI: IDENTIFY command timed out on port {}\n", port);
            return None;
        }

        // Parse IDENTIFY response from bounce buffer
        let raw_buf = &mem.bounce_buf;

        // Model String: Words 27..46 (offset 54..94)
        let mut model_chars = [0u8; 40];
        for i in 0..20 {
            model_chars[i * 2] = raw_buf[54 + i * 2 + 1];
            model_chars[i * 2 + 1] = raw_buf[54 + i * 2];
        }
        let model_str = core::str::from_utf8(&model_chars).unwrap_or("SATA Disk").trim();

        // Serial Number: Words 10..19 (offset 20..40)
        let mut sn_chars = [0u8; 20];
        for i in 0..10 {
            sn_chars[i * 2] = raw_buf[20 + i * 2 + 1];
            sn_chars[i * 2 + 1] = raw_buf[20 + i * 2];
        }
        let sn_str = core::str::from_utf8(&sn_chars).unwrap_or("").trim();

        // LBA48 Total Sectors: Words 100..103 (offset 200..208)
        let total_sectors = ((raw_buf[207] as u64) << 56)
            | ((raw_buf[206] as u64) << 48)
            | ((raw_buf[205] as u64) << 40)
            | ((raw_buf[204] as u64) << 32)
            | ((raw_buf[203] as u64) << 24)
            | ((raw_buf[202] as u64) << 16)
            | ((raw_buf[201] as u64) << 8)
            | (raw_buf[200] as u64);

        let safe_sectors = if total_sectors > 0 { total_sectors } else { 104857600 }; // 50GB fallback
        let cap_mb = (safe_sectors * 512) / (1024 * 1024);
        let is_ssd = model_str.to_lowercase().contains("ssd") || model_str.contains("Flash");

        let drive = SataDriveInfo {
            port,
            model: String::from(model_str),
            serial: String::from(sn_str),
            total_sectors: safe_sectors,
            capacity_mb: cap_mb,
            is_ssd,
        };

        let _ = write!(
            serial,
            "AHCI Port {}: Model='{}' | S/N='{}' | Capacity={} MB ({} Sectors)\n",
            port, drive.model, drive.serial, drive.capacity_mb, drive.total_sectors
        );

        Some(drive)
    }

    pub fn read_sector(&self, port: usize, lba: u64, buf: &mut [u8; 512]) -> bool {
        if port >= 4 {
            return false;
        }

        unsafe {
            let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
            let mem = &mut PORT_MEM[port];

            let ct_phys = (mem.cmd_table.as_ptr() as u64).wrapping_sub(hhdm);
            let buf_phys = (mem.bounce_buf.as_ptr() as u64).wrapping_sub(hhdm);

            core::ptr::write_bytes(mem.cmd_table.as_mut_ptr(), 0, 512);

            mem.cmd_headers[0].flags = 5; // CFL = 5, Read (W=0)
            mem.cmd_headers[0].prdtl = 1;
            mem.cmd_headers[0].ctba = (ct_phys & 0xFFFFFFFF) as u32;
            mem.cmd_headers[0].ctbau = (ct_phys >> 32) as u32;

            let prdt_ptr = (mem.cmd_table.as_mut_ptr().add(0x80)) as *mut AhciPrdtEntry;
            (*prdt_ptr).dba = (buf_phys & 0xFFFFFFFF) as u32;
            (*prdt_ptr).dbau = (buf_phys >> 32) as u32;
            (*prdt_ptr).dbc = 511 | (1 << 31);

            let fis = mem.cmd_table.as_mut_ptr();
            *fis.add(0) = 0x27; // Host to Device
            *fis.add(1) = 0x80; // Command
            *fis.add(2) = ATA_CMD_READ_DMA_EXT; // 0x25 READ DMA EXT
            *fis.add(4) = (lba & 0xFF) as u8;
            *fis.add(5) = ((lba >> 8) & 0xFF) as u8;
            *fis.add(6) = ((lba >> 16) & 0xFF) as u8;
            *fis.add(7) = 0x40; // LBA mode
            *fis.add(8) = ((lba >> 24) & 0xFF) as u8;
            *fis.add(9) = ((lba >> 32) & 0xFF) as u8;
            *fis.add(10) = ((lba >> 40) & 0xFF) as u8;
            *fis.add(12) = 1; // 1 sector
            *fis.add(13) = 0;

            self.write_port(port, PORT_CI, 1 << 0);

            for _ in 0..100_000 {
                if (self.read_port(port, PORT_CI) & (1 << 0)) == 0 {
                    buf.copy_from_slice(&mem.bounce_buf[0..512]);
                    return true;
                }
                core::hint::spin_loop();
            }
        }
        false
    }
}

pub fn detect_and_init(bus: u8, slot: u8, func: u8, bar5_raw: u32) -> bool {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let abar = (bar5_raw & 0xFFFF_FFF0) as usize;

    let _ = write!(serial, "AHCI: Discovered SATA Controller at PCI {:02x}:{:02x}.{} (ABAR: {:#x})\n", bus, slot, func, abar);

    let mut controller = AhciController::new(abar);
    if controller.init() {
        *ACTIVE_AHCI.lock() = Some(controller);
        true
    } else {
        false
    }
}

pub fn is_present() -> bool {
    let guard = ACTIVE_AHCI.lock();
    guard.as_ref().map_or(false, |ctrl| !ctrl.drives.is_empty())
}

pub fn get_status() -> String {
    let guard = ACTIVE_AHCI.lock();
    if let Some(ctrl) = guard.as_ref() {
        let mut out = format!("Serial ATA (AHCI SATA-3.x) Controller at ABAR {:#x}:\n", ctrl.abar);
        if ctrl.drives.is_empty() {
            out.push_str("  No active SATA drives attached.\n");
        } else {
            for d in &ctrl.drives {
                out.push_str(&format!(
                    "  Port {}: {} [{}] — {} MB ({} Sectors)\n",
                    d.port,
                    d.model,
                    if d.is_ssd { "Solid State Drive" } else { "Hard Disk Drive" },
                    d.capacity_mb,
                    d.total_sectors
                ));
            }
        }
        out
    } else {
        format!("AHCI: No SATA AHCI controller initialized.")
    }
}
