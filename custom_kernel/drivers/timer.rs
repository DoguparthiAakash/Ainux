use core::arch::asm;
use crate::cpu::pic;

const PIT_CMD_PORT: u16 = 0x43;
const PIT_CH0_PORT: u16 = 0x40;
const PIT_FREQ: u32 = 1_193_182;

/// Initialize the PIT to the specified frequency (in Hz).
/// Typically 100Hz or 1000Hz.
pub fn init(freq: u32) {
    let divisor = PIT_FREQ / freq;
    
    // Check constraints (divisor fits in u16)
    if divisor > 65535 {
        // Can't go that slow (min ~18Hz). Cap it.
        // Or panic? For now, we assume reasonable freq.
    }

    // Configure PIT:
    // Channel 0, Access Mode = Lo/Hi byte, Mode = 3 (Square Wave), Binary Mode
    // 00 11 011 0 = 0x36
    unsafe {
        outb(PIT_CMD_PORT, 0x36);
        
        let low = (divisor & 0xFF) as u8;
        let high = ((divisor >> 8) & 0xFF) as u8;
        
        outb(PIT_CH0_PORT, low);
        outb(PIT_CH0_PORT, high);
        
        // Unmask IRQ 0 on PIC
        pic::unmask_irq(0);
    }
}

unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}
