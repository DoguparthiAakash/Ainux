extern crate alloc;
// use alloc::vec::Vec;
// use alloc::string::String;
use crate::drivers::{keyboard, video, rtc};
use core::fmt::Write;

// We need a way to get input from the keyboard driver.
// Currently keyboard driver pushes to a buffer but doesn't expose a "read_line" or stream.
// We will poll `keyboard::pop_key()` (needs implementation) or similar.

// Assuming a `poll_key` or event queue exists.
// We'll stub it here and update keyboard driver next.

pub fn run() {
    let mut serial = crate::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "Shell: Run entered.\n");

    video::put_str("\nWelcome to Ainux Shell!\nType 'help' for commands.\n");
    let _ = write!(serial, "Shell: Welcome printed.\n");
    
    // Fixed buffer for input (No Heap)
    let mut buffer: [char; 64] = ['\0'; 64];
    let mut len = 0;
    
    loop {
        video::put_str("ainux> ");
        
        // Read Line Loop
        loop {
            if let Some(c) = keyboard::pop_char() {
                if c == '\n' {
                    video::put_char('\n');
                    break;
                } else if c == '\x08' { // Backspace
                     if len > 0 {
                         len -= 1;
                         buffer[len] = '\0';
                         video::put_str("\x08 \x08");
                     }
                } else {
                     if len < 64 {
                         buffer[len] = c;
                         len += 1;
                         video::put_char(c);
                     }
                }
            } else {
                // Yield CPU to allow interrupts to fire
                unsafe { core::arch::asm!("pause", options(nomem, nostack, preserves_flags)); }
            }
        }
        
        // Parse Command (Manual match since no String/slice comparison easily without alloc? 
        // We can match slice buffer[0..len])
        // Need to convert [char] to str or match manually.
        // Simplest: match first char? Or implement simple string match.
        
        // Convert buffer to temporary str if possible?
        // We can't easily create &str from [char].
        // We should treat buffer as bytes? Keyboard returns char.
        // Let's iterate.
        
        let cmd_len = len;
        
        // Reset buffer len for next loop early
        len = 0;
        
        if cmd_len > 0 {
             // Simple command parser
             // Check "help"
             if cmd_len == 4 
                && buffer[0] == 'h' && buffer[1] == 'e' && buffer[2] == 'l' && buffer[3] == 'p' {
                video::put_str("Available commands:\n");
                video::put_str("  help    - Show this menu\n");
                video::put_str("  whoami  - Show current user\n");
                video::put_str("  time    - Show current time\n");
                video::put_str("  clear   - Clear screen\n");
                video::put_str("  shutdown- Halt CPU\n");
             } 
             else if cmd_len == 6
                && buffer[0] == 'w' && buffer[1] == 'h' && buffer[2] == 'o' && buffer[3] == 'a' && buffer[4] == 'm' && buffer[5] == 'i' {
                 video::put_str("root\n");
             }
             else if cmd_len == 5
                && buffer[0] == 'c' && buffer[1] == 'l' && buffer[2] == 'e' && buffer[3] == 'a' && buffer[4] == 'r' {
                 video::clear();
             }
             else if cmd_len == 4
                && buffer[0] == 't' && buffer[1] == 'i' && buffer[2] == 'm' && buffer[3] == 'e' {
                  // Time
                  let t = rtc::read_time();
                  // Manual format
                  print_digit(t.hours as u8);
                  video::put_char(':');
                  print_digit(t.minutes as u8);
                  video::put_char(':');
                  print_digit(t.seconds as u8);
                  video::put_str(" (UTC)\n");
             }
             else if cmd_len == 8
                && buffer[0] == 's' && buffer[1] == 'h' && buffer[2] == 'u' && buffer[3] == 't' && buffer[4] == 'd' && buffer[5] == 'o' && buffer[6] == 'w' && buffer[7] == 'n' {
                 video::put_str("Shutting down...\n");
                 loop { unsafe { core::arch::asm!("hlt"); } }
             }
             else {
                 video::put_str("Unknown command.\n");
             }
        }
        
        // Zero out buffer (optional, len handles it)
        for i in 0..64 { buffer[i] = '\0'; }
    }
}

fn print_digit(val: u8) {
    let tens = val / 10;
    let ones = val % 10;
    video::put_char((b'0' + tens) as char);
    video::put_char((b'0' + ones) as char);

}
