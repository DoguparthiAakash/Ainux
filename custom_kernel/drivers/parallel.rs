// =============================================================================
// IEEE 1284 Standard Parallel Port (LPT1 / LPT2) Driver
// Supports legacy line printers, parallel zip drives, and diagnostic dongles.
// Base Ports: LPT1 (0x378), LPT2 (0x278).
// Dropped by modern OSes (Win11/modern Linux) - Revived for Ainux Sovereign
// =============================================================================

use core::arch::asm;
use spin::Mutex;
use alloc::string::String;
use alloc::format;
use core::fmt::Write;

pub const LPT1_BASE: u16 = 0x378;
pub const LPT2_BASE: u16 = 0x278;

// Register Offsets
const LPT_DATA:    u16 = 0; // Data Register (R/W)
const LPT_STATUS:  u16 = 1; // Status Register (R)
const LPT_CONTROL: u16 = 2; // Control Register (R/W)

// Status Bits (Port + 1)
const STATUS_FAULT:   u8 = 0x08; // Pin 15: Error (Active low: 0 = error)
const STATUS_SELECT:  u8 = 0x10; // Pin 13: Selected / On-Line (1 = online)
const STATUS_PERROR:  u8 = 0x20; // Pin 12: Paper Out (1 = out of paper)
const STATUS_ACK:     u8 = 0x40; // Pin 10: Acknowledge (Active low)
const STATUS_BUSY:    u8 = 0x80; // Pin 11: Busy (Active low: 0 = busy, 1 = not busy)

// Control Bits (Port + 2)
const CONTROL_STROBE:   u8 = 0x01; // Pin 1: Strobe (Active low in hardware)
const CONTROL_AUTOFD:   u8 = 0x02; // Pin 14: Auto linefeed
const CONTROL_INIT:     u8 = 0x04; // Pin 16: Initialize printer (Active low)
const CONTROL_SELECTIN: u8 = 0x08; // Pin 17: Select printer
const CONTROL_IRQ_EN:   u8 = 0x10; // IRQ Enable (IRQ 7 for LPT1, IRQ 5 for LPT2)
const CONTROL_BIDIR:    u8 = 0x20; // Bi-directional port direction (0=write, 1=read)

#[derive(Clone, Copy, Debug)]
pub struct LptPortState {
    pub base: u16,
    pub present: bool,
    pub online: bool,
    pub busy: bool,
    pub paper_out: bool,
    pub fault: bool,
}

pub struct ParallelPortManager {
    pub lpt1: LptPortState,
    pub lpt2: LptPortState,
}

pub static PARALLEL: Mutex<ParallelPortManager> = Mutex::new(ParallelPortManager {
    lpt1: LptPortState { base: LPT1_BASE, present: false, online: false, busy: false, paper_out: false, fault: false },
    lpt2: LptPortState { base: LPT2_BASE, present: false, online: false, busy: false, paper_out: false, fault: false },
});

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
    unsafe { outb(0x80, 0x00); }
}

// Probe for hardware presence by writing test bit patterns to the data register
fn probe_lpt(base: u16) -> bool {
    unsafe {
        let original = inb(base + LPT_DATA);

        outb(base + LPT_DATA, 0x55);
        io_wait();
        let read1 = inb(base + LPT_DATA);

        outb(base + LPT_DATA, 0xAA);
        io_wait();
        let read2 = inb(base + LPT_DATA);

        outb(base + LPT_DATA, original);

        read1 == 0x55 && read2 == 0xAA
    }
}

// Read and decode status register
fn read_lpt_status(base: u16) -> (bool, bool, bool, bool) {
    unsafe {
        let status = inb(base + LPT_STATUS);
        let not_busy = (status & STATUS_BUSY) != 0;
        let online = (status & STATUS_SELECT) != 0;
        let paper_out = (status & STATUS_PERROR) != 0;
        let fault = (status & STATUS_FAULT) == 0;
        (not_busy, online, paper_out, fault)
    }
}

// Reset printer by pulsing the ~INIT pin low
fn reset_printer(base: u16) {
    unsafe {
        // Drop INIT low (bit 2 = 0)
        outb(base + LPT_CONTROL, CONTROL_SELECTIN);
        for _ in 0..10_000 { core::hint::spin_loop(); }

        // Raise INIT high (bit 2 = 1) and select printer
        outb(base + LPT_CONTROL, CONTROL_INIT | CONTROL_SELECTIN);
        for _ in 0..10_000 { core::hint::spin_loop(); }
    }
}

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let mut mgr = PARALLEL.lock();

    // Probe LPT1
    if probe_lpt(LPT1_BASE) {
        mgr.lpt1.present = true;
        reset_printer(LPT1_BASE);
        let (not_busy, online, paper_out, fault) = read_lpt_status(LPT1_BASE);
        mgr.lpt1.busy = !not_busy;
        mgr.lpt1.online = online;
        mgr.lpt1.paper_out = paper_out;
        mgr.lpt1.fault = fault;

        let _ = write!(
            serial,
            "LPT1: Parallel Port found at I/O {:#x} [Online: {}, Busy: {}, PaperOut: {}]\n",
            LPT1_BASE, online, !not_busy, paper_out
        );
    } else {
        let _ = write!(serial, "LPT1: No parallel port hardware at I/O {:#x}\n", LPT1_BASE);
    }

    // Probe LPT2
    if probe_lpt(LPT2_BASE) {
        mgr.lpt2.present = true;
        reset_printer(LPT2_BASE);
        let (not_busy, online, paper_out, fault) = read_lpt_status(LPT2_BASE);
        mgr.lpt2.busy = !not_busy;
        mgr.lpt2.online = online;
        mgr.lpt2.paper_out = paper_out;
        mgr.lpt2.fault = fault;

        let _ = write!(
            serial,
            "LPT2: Parallel Port found at I/O {:#x} [Online: {}, Busy: {}]\n",
            LPT2_BASE, online, !not_busy
        );
    }
}

pub fn write_byte(base: u16, byte: u8) -> bool {
    unsafe {
        // Wait for printer to be ready (!BUSY) with timeout
        let mut ready = false;
        for _ in 0..100_000 {
            if (inb(base + LPT_STATUS) & STATUS_BUSY) != 0 {
                ready = true;
                break;
            }
            core::hint::spin_loop();
        }

        if !ready {
            return false;
        }

        // Put byte onto data lines
        outb(base + LPT_DATA, byte);
        for _ in 0..10 { io_wait(); }

        // Assert STROBE (bit 0 = 1 in software triggers active low pin)
        outb(base + LPT_CONTROL, CONTROL_INIT | CONTROL_SELECTIN | CONTROL_STROBE);
        for _ in 0..15 { io_wait(); }

        // De-assert STROBE
        outb(base + LPT_CONTROL, CONTROL_INIT | CONTROL_SELECTIN);
        for _ in 0..10 { io_wait(); }

        true
    }
}

pub fn print_str(s: &str) -> usize {
    let base = LPT1_BASE;
    let mut count = 0;
    for b in s.bytes() {
        if b == b'\n' {
            let _ = write_byte(base, b'\r');
        }
        if write_byte(base, b) {
            count += 1;
        } else {
            break;
        }
    }
    count
}

pub fn is_present() -> bool {
    let mgr = PARALLEL.lock();
    mgr.lpt1.present || mgr.lpt2.present
}

pub fn get_status() -> String {
    let mgr = PARALLEL.lock();
    let mut out = String::from("IEEE 1284 Parallel Port (LPT):\n");
    if mgr.lpt1.present {
        let (not_busy, online, paper_out, fault) = read_lpt_status(LPT1_BASE);
        out.push_str(&format!(
            "  LPT1 (0x378) : Detected | Online: {} | State: {} | Paper: {} | Fault: {}\n",
            if online { "Yes" } else { "No/Loopback" },
            if not_busy { "Ready" } else { "Busy" },
            if paper_out { "Empty" } else { "OK" },
            if fault { "Error" } else { "None" }
        ));
    } else {
        out.push_str("  LPT1 (0x378) : Not Present\n");
    }

    if mgr.lpt2.present {
        let (not_busy, online, paper_out, fault) = read_lpt_status(LPT2_BASE);
        out.push_str(&format!(
            "  LPT2 (0x278) : Detected | Online: {} | State: {} | Paper: {} | Fault: {}\n",
            if online { "Yes" } else { "No" },
            if not_busy { "Ready" } else { "Busy" },
            if paper_out { "Empty" } else { "OK" },
            if fault { "Error" } else { "None" }
        ));
    } else {
        out.push_str("  LPT2 (0x278) : Not Present\n");
    }
    out
}

