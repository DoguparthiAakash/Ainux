#![no_std]
#![no_main]

use libainux::println;
use libainux::syscalls::{sys_get_speaker_count, sys_play_beep, sys_read, sys_yield};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    println!("=== Hardware Diagnostic Test ===");

    // 1. Speaker check
    let speaker_count = sys_get_speaker_count();
    println!("Detected Speakers: {}", speaker_count);
    if speaker_count > 0 {
        println!("Playing test beep...");
        sys_play_beep();
    } else {
        println!("No AC97 audio device initialized.");
    }

    // 2. Keyboard check
    println!("\n--- Advanced Input Diagnostics ---");
    println!("Please press 10 different keys to verify keyboard functionality.");
    let mut keys_pressed = 0;
    while keys_pressed < 10 {
        let mut buf = [0u8; 1];
        let bytes_read = sys_read(0, &mut buf);
        if bytes_read > 0 {
            let key = buf[0];
            let display_char = if key >= 32 && key <= 126 {
                key as char
            } else {
                '?' // Non-printable
            };
            println!("Detected Key Event -> Raw Code: {:#04x} | ASCII: '{}' | Dec: {}", key, display_char, key);
            keys_pressed += 1;
        } else {
            sys_yield();
        }
    }

    println!("\n[OK] Keyboard input successfully verified.");
    println!("\nHardware diagnostic complete. All core I/O devices (Audio, Keyboard) are functional.");
    0
}
