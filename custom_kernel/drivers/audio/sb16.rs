// =============================================================================
// Sound Blaster 16 & OPL3 FM Synthesizer Driver
// Authentic retro PC sound hardware: DSP Base 0x220, Mixer, OPL3 Ports 0x388-0x38B
// Dropped by modern OSes (Win11/modern Linux) - Revived for Ainux Sovereign
// =============================================================================

use core::arch::asm;
use spin::Mutex;
use alloc::string::String;
use alloc::format;
use core::fmt::Write;

// DSP Port Offsets relative to Base (0x220 or 0x240)
const DSP_MIXER_ADDR: u16 = 0x04;
const DSP_MIXER_DATA: u16 = 0x05;
const DSP_RESET:      u16 = 0x06;
const DSP_READ:       u16 = 0x0A;
const DSP_WRITE:      u16 = 0x0C; // Write Command / Data
const DSP_STATUS:     u16 = 0x0C; // Bit 7: 1 = DSP busy (write buffer full)
const DSP_DATA_AVAIL: u16 = 0x0E; // Bit 7: 1 = DSP has data to read
const DSP_ACK_16BIT:  u16 = 0x0F;

// DSP Commands
const CMD_GET_VERSION:   u8 = 0xE1;
const CMD_SPEAKER_ON:    u8 = 0xD1;
const CMD_SPEAKER_OFF:   u8 = 0xD3;
const CMD_SET_TIME_CST:  u8 = 0x40;
const CMD_SET_SAMPLERATE: u8 = 0x41; // SB16 high-speed sample rate

// OPL3 FM Synthesizer Ports (AdLib / Yamaha YMF262)
const OPL3_BASE_ADDR: u16 = 0x388;
const OPL3_BASE_DATA: u16 = 0x389;
const OPL3_EXP_ADDR:  u16 = 0x38A;
const OPL3_EXP_DATA:  u16 = 0x38B;

pub struct SoundBlaster16 {
    pub base_port: u16,
    pub present: bool,
    pub version_major: u8,
    pub version_minor: u8,
    pub opl3_present: bool,
}

pub static SB16: Mutex<SoundBlaster16> = Mutex::new(SoundBlaster16 {
    base_port: 0x220,
    present: false,
    version_major: 0,
    version_minor: 0,
    opl3_present: false,
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
    unsafe { outb(0x80, 0x00); }
}

// Reset DSP on specified base port; returns true if signature 0xAA received
fn reset_dsp(base: u16) -> bool {
    unsafe {
        // Write 1 to reset port
        outb(base + DSP_RESET, 1);
        for _ in 0..10_000 { core::hint::spin_loop(); }

        // Write 0 to reset port
        outb(base + DSP_RESET, 0);
        for _ in 0..10_000 { core::hint::spin_loop(); }

        // Poll Data Available port (0x22E)
        for _ in 0..50_000 {
            if (inb(base + DSP_DATA_AVAIL) & 0x80) != 0 {
                if inb(base + DSP_READ) == 0xAA {
                    return true;
                }
            }
            core::hint::spin_loop();
        }
    }
    false
}

// Write a byte / command to the DSP
fn dsp_write(base: u16, val: u8) -> bool {
    unsafe {
        for _ in 0..50_000 {
            if (inb(base + DSP_STATUS) & 0x80) == 0 {
                outb(base + DSP_WRITE, val);
                return true;
            }
            core::hint::spin_loop();
        }
    }
    false
}

// Read a byte from the DSP
fn dsp_read(base: u16) -> Option<u8> {
    unsafe {
        for _ in 0..50_000 {
            if (inb(base + DSP_DATA_AVAIL) & 0x80) != 0 {
                return Some(inb(base + DSP_READ));
            }
            core::hint::spin_loop();
        }
    }
    None
}

// Mixer register write
fn mixer_write(base: u16, reg: u8, val: u8) {
    unsafe {
        outb(base + DSP_MIXER_ADDR, reg);
        io_wait();
        outb(base + DSP_MIXER_DATA, val);
    }
}

// Mixer register read
fn mixer_read(base: u16, reg: u8) -> u8 {
    unsafe {
        outb(base + DSP_MIXER_ADDR, reg);
        io_wait();
        inb(base + DSP_MIXER_DATA)
    }
}

// OPL3 register write
pub fn opl3_write(reg: u16, val: u8) {
    unsafe {
        let (port_addr, port_data) = if reg >= 0x100 {
            (OPL3_EXP_ADDR, OPL3_EXP_DATA)
        } else {
            (OPL3_BASE_ADDR, OPL3_BASE_DATA)
        };
        outb(port_addr, (reg & 0xFF) as u8);
        for _ in 0..6 { io_wait(); }
        outb(port_data, val);
        for _ in 0..35 { io_wait(); }
    }
}

// Detect OPL3 FM Synthesizer
fn detect_opl3() -> bool {
    // Reset timer 1 and 2
    opl3_write(0x04, 0x60);
    // Reset IRQ
    opl3_write(0x04, 0x80);
    
    let status1 = unsafe { inb(OPL3_BASE_ADDR) };

    // Set timer 1 to 0xFF
    opl3_write(0x02, 0xFF);
    // Start timer 1
    opl3_write(0x04, 0x21);

    for _ in 0..10_000 { core::hint::spin_loop(); }

    let status2 = unsafe { inb(OPL3_BASE_ADDR) };

    // Reset timers again
    opl3_write(0x04, 0x60);
    opl3_write(0x04, 0x80);

    // If timer fired (status2 bit 7 or bit 6), FM synth chip exists!
    (status1 & 0xE0) == 0x00 && (status2 & 0xE0) != 0x00
}

// Initialize OPL3 in 2-operator FM mode
pub fn opl3_init() {
    // Enable OPL3 mode (write 1 to reg 0x105)
    opl3_write(0x105, 0x01);

    // Clear all registers
    for reg in 0..0x200 {
        opl3_write(reg, 0x00);
    }
    // Enable waveform control
    opl3_write(0x01, 0x20);
}

// Play FM synth note on Channel 0
pub fn opl3_play_note(freq_hz: u32, duration_ms: u32) {
    // OPL3 F-Number calculation: F-Number = freq * 2^(20 - block) / 49716
    let block = 4u32; // Octave 4
    let fnum = ((freq_hz as u64 * (1 << (20 - block))) / 49716) as u32;

    // Operator 1 (Modulator): Slot 0 (Offset 0x00)
    opl3_write(0x20, 0x01); // Multiplier = 1
    opl3_write(0x40, 0x10); // Moderate volume
    opl3_write(0x60, 0xF2); // Fast attack, medium decay
    opl3_write(0x80, 0x77); // Medium sustain, medium release
    opl3_write(0xE0, 0x00); // Sine wave

    // Operator 2 (Carrier): Slot 3 (Offset 0x03)
    opl3_write(0x23, 0x01); // Multiplier = 1
    opl3_write(0x43, 0x00); // Maximum volume (0 = loudest)
    opl3_write(0x63, 0xF2); // Fast attack, medium decay
    opl3_write(0x83, 0x77); // Medium sustain, medium release
    opl3_write(0xE3, 0x00); // Sine wave

    // Connect operators in FM mode, output to both left & right
    opl3_write(0xC0, 0x30);

    // Set frequency low
    opl3_write(0xA0, (fnum & 0xFF) as u8);
    // Set frequency high + Key-On bit (bit 5) + Octave
    opl3_write(0xB0, 0x20 | (((block as u8) & 0x07) << 2) | (((fnum >> 8) & 0x03) as u8));

    // Hold note duration
    for _ in 0..(duration_ms * 12_000) {
        core::hint::spin_loop();
    }

    // Key-Off: clear bit 5 in 0xB0
    opl3_write(0xB0, (((block as u8) & 0x07) << 2) | (((fnum >> 8) & 0x03) as u8));

    for _ in 0..50_000 {
        core::hint::spin_loop();
    }
}

// Play classic 4-note ascending Sound Blaster fanfare (C4, E4, G4, C5)
pub fn play_sound_blast() {
    let notes = [
        (261u32, 100u32), // C4
        (329u32, 100u32), // E4
        (392u32, 120u32), // G4
        (523u32, 250u32), // C5
    ];

    let has_opl3 = { SB16.lock().opl3_present };

    if has_opl3 {
        opl3_init();
        for &(freq, dur) in &notes {
            opl3_play_note(freq, dur);
        }
    } else {
        // Fallback to PC Speaker
        crate::drivers::audio::speaker::play_melody(&notes);
    }
}

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let mut sb = SB16.lock();

    // Probe base 0x220, then 0x240 fallback
    let candidate_bases = [0x220, 0x240];
    let mut detected = false;

    for &base in &candidate_bases {
        if reset_dsp(base) {
            sb.base_port = base;
            sb.present = true;
            detected = true;

            // Query DSP version
            if dsp_write(base, CMD_GET_VERSION) {
                if let (Some(maj), Some(min)) = (dsp_read(base), dsp_read(base)) {
                    sb.version_major = maj;
                    sb.version_minor = min;
                }
            }

            // Turn on DSP speaker
            dsp_write(base, CMD_SPEAKER_ON);

            // Configure Mixer: Set Master, Voice, FM, CD volume to ~85%
            mixer_write(base, 0x22, 0xEE); // Master volume (left/right)
            mixer_write(base, 0x04, 0xEE); // Voice/DAC volume
            mixer_write(base, 0x26, 0xEE); // FM Synth volume
            mixer_write(base, 0x28, 0xEE); // CD audio volume

            let _ = write!(
                serial,
                "SB16: Sound Blaster detected at I/O {:#x} (DSP v{}.{:02})\n",
                base, sb.version_major, sb.version_minor
            );
            break;
        }
    }

    // Check for OPL3 FM Synthesizer at 0x388
    let opl3_found = detect_opl3();
    sb.opl3_present = opl3_found;
    if opl3_found {
        opl3_init();
        let _ = write!(serial, "OPL3: Yamaha YMF262 FM Synthesizer detected at I/O 0x388-0x38B\n");
    }

    if !detected && !opl3_found {
        let _ = write!(serial, "SB16: No Sound Blaster hardware responding on 0x220/0x240 (PC Speaker fallback active).\n");
    }
}

pub fn is_present() -> bool {
    let sb = SB16.lock();
    sb.present || sb.opl3_present
}

pub fn get_status() -> String {
    let sb = SB16.lock();
    if sb.present || sb.opl3_present {
        format!(
            "Sound Blaster 16 & OPL3 Audio:\n  Status      : Online\n  DSP Base    : {:#x}\n  DSP Version : v{}.{:02} ({})\n  FM Synth    : {}\n  Mixer State : Master=88%, PCM=88%, FM=88%",
            sb.base_port,
            sb.version_major,
            sb.version_minor,
            if sb.version_major >= 4 { "Sound Blaster 16 / AWE" } else { "Sound Blaster Pro / 2.0" },
            if sb.opl3_present { "Yamaha OPL3 (YMF262) Dual-Operator FM" } else { "PC Speaker Emulation" }
        )
    } else {
        format!("Sound Blaster 16: Not detected on 0x220/0x240 (PC Speaker available at 0x61)")
    }
}

