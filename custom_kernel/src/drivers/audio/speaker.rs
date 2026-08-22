// =============================================================================
// PC Speaker Audio Driver (Port 0x61 & PIT Channel 2 0x42)
// =============================================================================

use core::arch::asm;

// I/O Port utilities
unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

unsafe fn inb(port: u16) -> u8 {
    let mut val: u8;
    asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

// Play sound using built-in speaker
pub fn play_sound(n_frequence: u32) {
    let div = 1193180 / n_frequence;
    unsafe {
        outb(0x43, 0xB6);
        outb(0x42, (div & 0xFF) as u8);
        outb(0x42, (div >> 8) as u8);

        let tmp = inb(0x61);
        if tmp != (tmp | 3) {
            outb(0x61, tmp | 3);
        }
    }
}

// Make it shut up
pub fn nosound() {
    unsafe {
        let tmp = inb(0x61) & 0xFC;
        outb(0x61, tmp);
    }
}

// Sleep utility (very rough busy loop for beep duration)
pub fn beep(freq: u32, duration_ms: u32) {
    play_sound(freq);
    // Simple busy loop (not accurate, depends on CPU speed)
    for _ in 0..(duration_ms * 10000) {
        core::hint::spin_loop();
    }
    nosound();
}

pub fn play_melody(notes: &[(u32, u32)]) {
    for &(freq, duration) in notes {
        if freq == 0 {
            nosound();
            for _ in 0..(duration * 10000) {
                core::hint::spin_loop();
            }
        } else {
            beep(freq, duration);
        }
    }
    nosound();
}
