use core::arch::{asm, naked_asm};
use crate::cpu::pic::notify_eoi;
use spin::Mutex;
use core::option::Option;

const BUFFER_SIZE: usize = 128;

struct RingBuffer {
    data: [char; BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
    count: usize,
}

impl RingBuffer {
    const fn new() -> Self {
        Self {
            data: ['\0'; BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
            count: 0,
        }
    }

    fn push(&mut self, c: char) {
        if self.count < BUFFER_SIZE {
            self.data[self.write_pos] = c;
            self.write_pos = (self.write_pos + 1) % BUFFER_SIZE;
            self.count += 1;
        }
    }

    fn pop(&mut self) -> Option<char> {
        if self.count > 0 {
            let c = self.data[self.read_pos];
            self.read_pos = (self.read_pos + 1) % BUFFER_SIZE;
            self.count -= 1;
            Some(c)
        } else {
            None
        }
    }
}

static KEY_BUFFER: Mutex<RingBuffer> = Mutex::new(RingBuffer::new());

static mut SHIFT: bool = false;
static mut CAPS: bool = false;
static mut NUMLOCK: bool = true;
static mut EXTENDED: bool = false;

static mut CTRL: bool = false;

unsafe fn wait_write() {
    while (inb(0x64) & 2) != 0 {}
}

unsafe fn wait_read() {
    while (inb(0x64) & 1) == 0 {}
}

unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

unsafe fn inb(port: u16) -> u8 {
    let mut val: u8;
    asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

pub fn init() {
    unsafe {
        // Explicitly ENABLE scanning for the keyboard
        // Wait for buffer to be empty
        wait_write();
        outb(0x60, 0xF4); // Enable Scanning
        
        // Wait for acknowledgment (0xFA) to clear the buffer
        // Note: bit 0 of 0x64 must be 1 for a successful read.
        wait_read();
        let _ack = inb(0x60); 
        
        crate::cpu::pic::unmask_irq(1);
    }
}

pub fn pop_char() -> Option<char> {
    KEY_BUFFER.lock().pop()
}

pub fn is_ctrl_active() -> bool {
    unsafe { CTRL }
}

pub unsafe extern "C" fn keyboard_handler_addr() -> u64 {
    keyboard_handler as u64
}

#[unsafe(naked)]
extern "C" fn keyboard_handler() {
    naked_asm!(
        "push rax",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "call rust_keyboard_handler",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rax",
        "iretq"
    );
}

#[no_mangle]
extern "C" fn rust_keyboard_handler() {
    unsafe {
        let status = inb(0x64);
        if (status & 1) == 0 {
            notify_eoi(1);
            return;
        }

        let scancode = inb(0x60);
        
        // If bit 5 of status is set, it's mouse data, not keyboard
        if (status & 0x20) != 0 {
            // Serial Debug: Print 'M' for mouse data found in KB handler
            asm!("out dx, al", in("dx") 0x3F8, in("al") b'm' as u8, options(nomem, nostack, preserves_flags));
            notify_eoi(1);
            return;
        }

        // Serial Debug: Print 'K' for every keyboard event
        asm!("out dx, al", in("dx") 0x3F8, in("al") b'K' as u8, options(nomem, nostack, preserves_flags));

        notify_eoi(1);
        if scancode == 0xE0 {
            EXTENDED = true;
            return;
        }

        let released = scancode & 0x80 != 0;
        let code = scancode & 0x7F;

        if released {
            match code {
                0x2A | 0x36 => SHIFT = false,
                0x1D => CTRL = false,
                _ => {}
            }
            return;
        }

        match code {
            0x2A | 0x36 => { SHIFT = true; return; }
            0x1D => { CTRL = true; return; }
            0x3A => { CAPS = !CAPS; return; }
            0x45 => { NUMLOCK = !NUMLOCK; return; }
            _ => {}
        }

        let ch = if EXTENDED {
            EXTENDED = false;
            match code {
                0x48 => Some('\u{2191}'), // ↑
                0x50 => Some('\u{2193}'), // ↓
                0x4B => Some('\u{2190}'), // ←
                0x4D => Some('\u{2192}'), // →
                0x47 => Some('\u{2196}'), // Home
                0x4F => Some('\u{2198}'), // End
                0x49 => Some('\u{21DE}'), // PgUp
                0x51 => Some('\u{21DF}'), // PgDn
                0x52 => Some('\u{2197}'), // Insert
                0x53 => Some('\x7F'), // Delete (DEL)
                _ => None,
            }
        } else {
            let mut c = decode_scancode(code);
            // Handle Ctrl+Char mapping
            if CTRL {
                if let Some(ch) = c {
                    if ch >= 'a' && ch <= 'z' {
                        c = Some((ch as u8 - b'a' + 1) as char);
                    } else if ch >= 'A' && ch <= 'Z' {
                         c = Some((ch as u8 - b'A' + 1) as char);
                    }
                }
            }
            c
        };

        if let Some(c) = ch {
            KEY_BUFFER.lock().push(c);
        }
    }
}

fn decode_scancode(code: u8) -> Option<char> {
    unsafe {
        let base = match code {
            // Numbers
            0x02 => '1', 0x03 => '2', 0x04 => '3', 0x05 => '4',
            0x06 => '5', 0x07 => '6', 0x08 => '7', 0x09 => '8',
            0x0A => '9', 0x0B => '0',

            // Letters
            0x10 => 'q', 0x11 => 'w', 0x12 => 'e', 0x13 => 'r',
            0x14 => 't', 0x15 => 'y', 0x16 => 'u', 0x17 => 'i',
            0x18 => 'o', 0x19 => 'p',

            0x1E => 'a', 0x1F => 's', 0x20 => 'd', 0x21 => 'f',
            0x22 => 'g', 0x23 => 'h', 0x24 => 'j', 0x25 => 'k',
            0x26 => 'l',

            0x2C => 'z', 0x2D => 'x', 0x2E => 'c', 0x2F => 'v',
            0x30 => 'b', 0x31 => 'n', 0x32 => 'm',

            // Symbols
            0x0C => '-', 0x0D => '=',
            0x1A => '[', 0x1B => ']',
            0x27 => ';', 0x28 => '\'',
            0x29 => '`', 0x2B => '\\',
            0x33 => ',', 0x34 => '.', 0x35 => '/',

            // Whitespace
            0x39 => ' ',
            0x1C => '\n',
            0x0F => '\t',
            0x0E => '\x08',

            // Numpad
            0x47 => if NUMLOCK { '7' } else { return None },
            0x48 => if NUMLOCK { '8' } else { return None },
            0x49 => if NUMLOCK { '9' } else { return None },
            0x4B => if NUMLOCK { '4' } else { return None },
            0x4C => if NUMLOCK { '5' } else { return None },
            0x4D => if NUMLOCK { '6' } else { return None },
            0x4F => if NUMLOCK { '1' } else { return None },
            0x50 => if NUMLOCK { '2' } else { return None },
            0x51 => if NUMLOCK { '3' } else { return None },
            0x52 => if NUMLOCK { '0' } else { return None },
            0x53 => if NUMLOCK { '.' } else { return None },

            _ => return None,
        };

        let mut c = base;

        if c.is_ascii_alphabetic() {
            if SHIFT ^ CAPS {
                c = c.to_ascii_uppercase();
            }
        } else if SHIFT {
            c = match c {
                '1' => '!', '2' => '@', '3' => '#', '4' => '$',
                '5' => '%', '6' => '^', '7' => '&', '8' => '*',
                '9' => '(', '0' => ')',
                '-' => '_', '=' => '+',
                '[' => '{', ']' => '}',
                ';' => ':', '\'' => '"',
                ',' => '<', '.' => '>', '/' => '?',
                '`' => '~', '\\' => '|',
                _ => c,
            };
        }

        Some(c)
    }
}
