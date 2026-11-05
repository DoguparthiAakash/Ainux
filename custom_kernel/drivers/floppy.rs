// =============================================================================
// Floppy Disk Controller (Intel 82077AA / NEC uPD765) Driver
// Supports legacy 3.5" 1.44MB & 5.25" floppy disk drives via ISA I/O ports
// and ISA DMA Channel 2. Dropped by modern OSes (Win11/modern Linux).
// =============================================================================

use core::arch::asm;
use spin::Mutex;
use alloc::string::String;
use alloc::format;
use core::fmt::Write;

// FDC I/O Port Definitions (Primary Controller)
pub const FDC_DOR:  u16 = 0x3F2; // Digital Output Register
pub const FDC_MSR:  u16 = 0x3F4; // Main Status Register (Read)
pub const FDC_DSR:  u16 = 0x3F4; // Datarate Select Register (Write)
pub const FDC_FIFO: u16 = 0x3F5; // Data FIFO / Command Register
pub const FDC_DIR:  u16 = 0x3F7; // Digital Input Register (Read)
pub const FDC_CCR:  u16 = 0x3F7; // Configuration Control Register (Write)

// MSR Bit Masks
pub const MSR_ACTA: u8 = 0x01; // Drive 0 Busy in seek mode
pub const MSR_ACTB: u8 = 0x02; // Drive 1 Busy in seek mode
pub const MSR_ACTC: u8 = 0x04; // Drive 2 Busy in seek mode
pub const MSR_ACTD: u8 = 0x08; // Drive 3 Busy in seek mode
pub const MSR_CB:   u8 = 0x10; // Command Busy
pub const MSR_NDMA: u8 = 0x20; // Non-DMA mode
pub const MSR_DIO:  u8 = 0x40; // Data Direction (0 = CPU->FDC, 1 = FDC->CPU)
pub const MSR_RQM:  u8 = 0x80; // Request for Master (Data register ready)

// FDC Commands
pub const CMD_SPECIFY:         u8 = 0x03;
pub const CMD_WRITE_DATA:      u8 = 0xC5;
pub const CMD_READ_DATA:       u8 = 0xE6;
pub const CMD_RECALIBRATE:     u8 = 0x07;
pub const CMD_SENSE_INTERRUPT: u8 = 0x08;
pub const CMD_SEEK:            u8 = 0x0F;
pub const CMD_VERSION:         u8 = 0x10;
pub const CMD_CONFIGURE:       u8 = 0x13;
pub const CMD_LOCK:            u8 = 0x94;

// ISA DMA Channel 2 Ports (8-bit DMA controller 1)
const DMA_CH2_ADDR:  u16 = 0x04;
const DMA_CH2_COUNT: u16 = 0x05;
const DMA_PAGE_CH2:  u16 = 0x81;
const DMA1_MASK:     u16 = 0x0A;
const DMA1_MODE:     u16 = 0x0B;
const DMA1_CLEAR_FF: u16 = 0x0C;

// 1.44MB Floppy Geometry
pub const FLOPPY_SECTORS_PER_TRACK: u8 = 18;
pub const FLOPPY_HEADS:             u8 = 2;
pub const FLOPPY_CYLINDERS:         u8 = 80;
pub const FLOPPY_SECTOR_SIZE:      usize = 512;
pub const FLOPPY_TOTAL_SECTORS:    u32 = (FLOPPY_CYLINDERS as u32) * (FLOPPY_HEADS as u32) * (FLOPPY_SECTORS_PER_TRACK as u32); // 2880

// Static aligned DMA bounce buffer in BSS (under 16MB boundary)
#[repr(C, align(4096))]
struct DmaBuffer {
    data: [u8; FLOPPY_SECTOR_SIZE],
}

static mut FLOPPY_DMA_BUF: DmaBuffer = DmaBuffer {
    data: [0u8; FLOPPY_SECTOR_SIZE],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloppyDriveType {
    None,
    Drive360Kb525,
    Drive12Mb525,
    Drive720Kb35,
    Drive144Mb35,
    Drive288Mb35,
    Unknown(u8),
}

impl FloppyDriveType {
    pub fn name(&self) -> &'static str {
        match self {
            FloppyDriveType::None => "No Floppy Drive",
            FloppyDriveType::Drive360Kb525 => "5.25\" 360 KB DD Floppy",
            FloppyDriveType::Drive12Mb525 => "5.25\" 1.2 MB HD Floppy",
            FloppyDriveType::Drive720Kb35 => "3.5\" 720 KB DD Floppy",
            FloppyDriveType::Drive144Mb35 => "3.5\" 1.44 MB HD Floppy",
            FloppyDriveType::Drive288Mb35 => "3.5\" 2.88 MB ED Floppy",
            FloppyDriveType::Unknown(_) => "Unknown Floppy Drive",
        }
    }
}

pub struct FloppyController {
    pub drive0_type: FloppyDriveType,
    pub drive1_type: FloppyDriveType,
    pub motor_on: bool,
    pub current_cylinder: [u8; 2],
    pub initialized: bool,
}

pub static FLOPPY: Mutex<FloppyController> = Mutex::new(FloppyController {
    drive0_type: FloppyDriveType::None,
    drive1_type: FloppyDriveType::None,
    motor_on: false,
    current_cylinder: [0xFF, 0xFF],
    initialized: false,
});

// Port I/O helpers
#[inline]
unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let mut val: u8;
    asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

#[inline]
fn io_wait() {
    unsafe {
        // Port 0x80 is standard diagnostic dummy delay
        outb(0x80, 0x00);
    }
}

// Query CMOS register 0x10 for Floppy Drive Types
fn detect_cmos_floppy() -> (FloppyDriveType, FloppyDriveType) {
    unsafe {
        outb(0x70, 0x10);
        io_wait();
        let types = inb(0x71);

        let d0 = match (types >> 4) & 0x0F {
            0 => FloppyDriveType::None,
            1 => FloppyDriveType::Drive360Kb525,
            2 => FloppyDriveType::Drive12Mb525,
            3 => FloppyDriveType::Drive720Kb35,
            4 => FloppyDriveType::Drive144Mb35,
            5 => FloppyDriveType::Drive288Mb35,
            v => FloppyDriveType::Unknown(v),
        };

        let d1 = match types & 0x0F {
            0 => FloppyDriveType::None,
            1 => FloppyDriveType::Drive360Kb525,
            2 => FloppyDriveType::Drive12Mb525,
            3 => FloppyDriveType::Drive720Kb35,
            4 => FloppyDriveType::Drive144Mb35,
            5 => FloppyDriveType::Drive288Mb35,
            v => FloppyDriveType::Unknown(v),
        };

        (d0, d1)
    }
}

// Wait for MSR to indicate ready for write (RQM=1, DIO=0)
fn wait_msr_write() -> bool {
    for _ in 0..100_000 {
        unsafe {
            let status = inb(FDC_MSR);
            if (status & (MSR_RQM | MSR_DIO)) == MSR_RQM {
                return true;
            }
        }
        core::hint::spin_loop();
    }
    false
}

// Wait for MSR to indicate ready for read (RQM=1, DIO=1)
fn wait_msr_read() -> bool {
    for _ in 0..100_000 {
        unsafe {
            let status = inb(FDC_MSR);
            if (status & (MSR_RQM | MSR_DIO)) == (MSR_RQM | MSR_DIO) {
                return true;
            }
        }
        core::hint::spin_loop();
    }
    false
}

// Send byte to FDC FIFO
fn write_cmd(cmd: u8) -> bool {
    if !wait_msr_write() {
        return false;
    }
    unsafe {
        outb(FDC_FIFO, cmd);
    }
    true
}

// Read byte from FDC FIFO
fn read_data() -> Option<u8> {
    if !wait_msr_read() {
        return None;
    }
    unsafe {
        Some(inb(FDC_FIFO))
    }
}

// Setup ISA DMA Controller Channel 2 for floppy transfer
fn setup_dma(buffer_phys: usize, length: usize, is_read: bool) {
    unsafe {
        // Mask Channel 2 (disable)
        outb(DMA1_MASK, 0x04 | 2);

        // Clear byte pointer flip-flop
        outb(DMA1_CLEAR_FF, 0x00);

        // Set Address (low byte, high byte)
        outb(DMA_CH2_ADDR, (buffer_phys & 0xFF) as u8);
        outb(DMA_CH2_ADDR, ((buffer_phys >> 8) & 0xFF) as u8);

        // Clear flip-flop again
        outb(DMA1_CLEAR_FF, 0x00);

        // Set Count (length - 1)
        let count = (length - 1) as u16;
        outb(DMA_CH2_COUNT, (count & 0xFF) as u8);
        outb(DMA_CH2_COUNT, ((count >> 8) & 0xFF) as u8);

        // Set DMA Page Register (bits 16..23 of physical addr)
        outb(DMA_PAGE_CH2, ((buffer_phys >> 16) & 0xFF) as u8);

        // Set DMA Mode:
        // Read from disk to memory: Single mode (0x40), Increment (0x00), Auto-init off (0x00), Write to mem (0x04), Channel 2 (0x02) = 0x46
        // Write from memory to disk: Single mode (0x40), Increment (0x00), Read from mem (0x08), Channel 2 (0x02) = 0x4A
        let mode = if is_read { 0x46 } else { 0x4A };
        outb(DMA1_MODE, mode);

        // Unmask Channel 2 (enable)
        outb(DMA1_MASK, 2);
    }
}

impl FloppyController {
    // Reset FDC controller
    pub fn reset(&mut self) -> bool {
        unsafe {
            // Pulse reset in DOR: Stop motor, disable DMA/IRQ, hold reset (bit 2 = 0)
            outb(FDC_DOR, 0x00);
            for _ in 0..2000 { core::hint::spin_loop(); }

            // Release reset, enable DMA & IRQ (bit 2 = 1, bit 3 = 1)
            outb(FDC_DOR, 0x0C);
            for _ in 0..2000 { core::hint::spin_loop(); }

            // Data rate: 500 kbps for 1.44MB (CCR = 0)
            outb(FDC_CCR, 0x00);
        }

        // Sense interrupt 4 times for the 4 drives
        for _ in 0..4 {
            let _ = self.sense_interrupt();
        }

        // Specify drive parameters: Step rate = 3ms, Head unload = 240ms, Head load = 16ms, Non-DMA = 0
        if !write_cmd(CMD_SPECIFY) { return false; }
        if !write_cmd(0xDF) { return false; } // SRT=3ms, HUT=240ms
        if !write_cmd(0x02) { return false; } // HLT=16ms, NDMA=0

        true
    }

    pub fn sense_interrupt(&mut self) -> Option<(u8, u8)> {
        if !write_cmd(CMD_SENSE_INTERRUPT) {
            return None;
        }
        let st0 = read_data()?;
        let pcn = read_data()?;
        Some((st0, pcn))
    }

    pub fn set_motor(&mut self, drive: u8, on: bool) {
        let drive_bit = drive & 0x03;
        let motor_mask = if on { (0x10 << drive_bit) } else { 0 };
        unsafe {
            outb(FDC_DOR, motor_mask | 0x0C | drive_bit);
        }
        self.motor_on = on;

        if on {
            // Motor spin-up delay (~300ms busy loop)
            for _ in 0..300_000 { core::hint::spin_loop(); }
        }
    }

    pub fn recalibrate(&mut self, drive: u8) -> bool {
        self.set_motor(drive, true);

        for _ in 0..2 {
            if !write_cmd(CMD_RECALIBRATE) { return false; }
            if !write_cmd(drive & 0x03) { return false; }

            // Wait for recalibrate completion
            for _ in 0..500_000 { core::hint::spin_loop(); }

            if let Some((st0, pcn)) = self.sense_interrupt() {
                if (st0 & 0xC0) == 0 && pcn == 0 {
                    self.current_cylinder[(drive & 1) as usize] = 0;
                    return true;
                }
            }
        }
        false
    }

    pub fn seek(&mut self, drive: u8, cylinder: u8, head: u8) -> bool {
        let idx = (drive & 1) as usize;
        if self.current_cylinder[idx] == cylinder {
            return true;
        }

        self.set_motor(drive, true);

        if !write_cmd(CMD_SEEK) { return false; }
        if !write_cmd(((head & 1) << 2) | (drive & 0x03)) { return false; }
        if !write_cmd(cylinder) { return false; }

        for _ in 0..500_000 { core::hint::spin_loop(); }

        if let Some((st0, pcn)) = self.sense_interrupt() {
            if (st0 & 0xC0) == 0 && pcn == cylinder {
                self.current_cylinder[idx] = cylinder;
                return true;
            }
        }
        false
    }

    pub fn read_sector(&mut self, drive: u8, lba: u32, out_buf: &mut [u8; 512]) -> bool {
        if lba >= FLOPPY_TOTAL_SECTORS {
            return false;
        }

        // Convert LBA to CHS
        let cylinder = (lba / (FLOPPY_HEADS as u32 * FLOPPY_SECTORS_PER_TRACK as u32)) as u8;
        let head = ((lba / FLOPPY_SECTORS_PER_TRACK as u32) % FLOPPY_HEADS as u32) as u8;
        let sector = ((lba % FLOPPY_SECTORS_PER_TRACK as u32) + 1) as u8;

        self.set_motor(drive, true);

        if !self.seek(drive, cylinder, head) {
            return false;
        }

        // Physical address of DMA bounce buffer
        let dma_phys = unsafe { (&FLOPPY_DMA_BUF.data as *const _ as usize) & 0x00FF_FFFF };
        setup_dma(dma_phys, FLOPPY_SECTOR_SIZE, true);

        // Send READ_DATA command
        // Multi-track (0x80) | MFM (0x40) | Skip (0x20) | READ (0x06) = 0xE6
        if !write_cmd(CMD_READ_DATA) { return false; }
        if !write_cmd(((head & 1) << 2) | (drive & 0x03)) { return false; }
        if !write_cmd(cylinder) { return false; }
        if !write_cmd(head) { return false; }
        if !write_cmd(sector) { return false; }
        if !write_cmd(2) { return false; } // Sector size: 2 = 512 bytes (128 << 2)
        if !write_cmd(FLOPPY_SECTORS_PER_TRACK) { return false; } // End of track (EOT)
        if !write_cmd(0x1B) { return false; } // Gap 3 length for 3.5"
        if !write_cmd(0xFF) { return false; } // Data length

        // Wait for execution and read result phase (7 bytes)
        let mut results = [0u8; 7];
        for i in 0..7 {
            match read_data() {
                Some(b) => results[i] = b,
                None => return false,
            }
        }

        // ST0 bits 6..7: 00 = normal completion
        let st0 = results[0];
        let st1 = results[1];
        let st2 = results[2];

        if (st0 & 0xC0) != 0 || (st1 & 0x80) != 0 || (st2 & 0x80) != 0 {
            return false;
        }

        // Copy from DMA bounce buffer to output buffer
        unsafe {
            out_buf.copy_from_slice(&FLOPPY_DMA_BUF.data);
        }

        true
    }
}

// Global public functions
pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let (d0, d1) = detect_cmos_floppy();

    let mut controller = FLOPPY.lock();
    controller.drive0_type = d0;
    controller.drive1_type = d1;

    let _ = write!(serial, "Floppy FDC: Drive 0 = {}, Drive 1 = {}\n", d0.name(), d1.name());

    if d0 != FloppyDriveType::None || d1 != FloppyDriveType::None {
        if controller.reset() {
            controller.initialized = true;
            let _ = write!(serial, "Floppy FDC (82077AA): Initialized successfully.\n");
        } else {
            let _ = write!(serial, "Floppy FDC: Controller reset failed or no controller responded.\n");
        }
    } else {
        let _ = write!(serial, "Floppy FDC: No floppy drives detected in CMOS.\n");
    }
}

pub fn is_present() -> bool {
    let controller = FLOPPY.lock();
    controller.drive0_type != FloppyDriveType::None || controller.drive1_type != FloppyDriveType::None
}

pub fn get_status() -> String {
    let controller = FLOPPY.lock();
    format!(
        "FDC 82077AA Legacy Floppy Controller:\n  Drive 0: {} [State: {}]\n  Drive 1: {} [State: {}]\n  Geometry: 80 Cylinders, 2 Heads, 18 Sectors/Track (1.44 MB)",
        controller.drive0_type.name(),
        if controller.initialized { "Ready" } else { "Offline" },
        controller.drive1_type.name(),
        if controller.initialized { "Ready" } else { "Offline" }
    )
}

pub fn read_sector(drive: u8, lba: u32, buf: &mut [u8; 512]) -> bool {
    FLOPPY.lock().read_sector(drive, lba, buf)
}

