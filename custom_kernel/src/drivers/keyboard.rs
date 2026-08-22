use core::arch::{asm, naked_asm};
use crate::cpu::pic::notify_eoi;
use spin::Mutex;
use core::option::Option;
use crate::klog;

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
            // Trace pushed characters for debugging UI stalls
            klog!("KBD+{}\n", c as u32);
        }
    }

    fn pop(&mut self) -> Option<char> {
        if self.count > 0 {
            let c = self.data[self.read_pos];
            self.read_pos = (self.read_pos + 1) % BUFFER_SIZE;
            self.count -= 1;
            // Trace popped characters for debugging UI stalls
            klog!("KBD-{}\n", c as u32);
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
static mut ALT: bool = false;
static mut SUPER: bool = false;

// Custom Keycodes (Unicode Private Use Area)
pub const KEY_F1: char = '\u{E001}';
pub const KEY_F2: char = '\u{E002}';
pub const KEY_F3: char = '\u{E003}';
pub const KEY_F4: char = '\u{E004}';
pub const KEY_F5: char = '\u{E005}';
pub const KEY_F6: char = '\u{E006}';
pub const KEY_F7: char = '\u{E007}';
pub const KEY_F8: char = '\u{E008}';
pub const KEY_F9: char = '\u{E009}';
pub const KEY_F10: char = '\u{E00A}';
pub const KEY_F11: char = '\u{E00B}';
pub const KEY_F12: char = '\u{E00C}';

pub const KEY_UP: char = '\u{2191}';
pub const KEY_DOWN: char = '\u{2193}';
pub const KEY_LEFT: char = '\u{2190}';
pub const KEY_RIGHT: char = '\u{2192}';

pub const KEY_HOME: char = '\u{2196}';
pub const KEY_END: char = '\u{2198}';
pub const KEY_PGUP: char = '\u{21DE}';
pub const KEY_PGDN: char = '\u{21DF}';
pub const KEY_INS: char = '\u{2197}';
pub const KEY_DEL: char = '\x7F';

pub const KEY_PRTSCR: char = '\u{E010}';
pub const KEY_SCROLL: char = '\u{E011}';
pub const KEY_PAUSE: char = '\u{E012}';
pub const KEY_MENU: char = '\u{E013}';

unsafe fn wait_write_timeout() -> bool {
    let mut timeout = 100_000u32;
    while (inb(0x64) & 2) != 0 {
        timeout -= 1;
        if timeout == 0 { return false; }
    }
    true
}

unsafe fn wait_read_timeout() -> bool {
    let mut timeout = 100_000u32;
    while (inb(0x64) & 1) == 0 {
        timeout -= 1;
        if timeout == 0 { return false; }
    }
    true
}

// Keep non-timeout versions for use from IRQ handler (they should be fast)
unsafe fn wait_write() {
    wait_write_timeout();
}

unsafe fn wait_read() {
    wait_read_timeout();
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
        // Flush any stale bytes from the PS/2 output buffer
        let mut flush = 0;
        while (inb(0x64) & 1) != 0 && flush < 16 {
            let _ = inb(0x60);
            flush += 1;
        }

        // Send 0xF4 (Enable Scanning) with write-buffer timeout
        if wait_write_timeout() {
            outb(0x60, 0xF4);
            // Wait for 0xFA ACK — but don't hang if VirtualBox doesn't send it
            if wait_read_timeout() {
                let _ack = inb(0x60);
            }
        }

        // Always unmask IRQ1 regardless of ACK result
        crate::cpu::pic::unmask_irq(1);
    }
}

pub fn pop_char() -> Option<char> {
    let mut result = None;
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
        result = KEY_BUFFER.lock().pop();
        core::arch::asm!("sti", options(nomem, nostack));
    }
    result
}

pub fn has_char() -> bool {
    KEY_BUFFER.lock().count > 0
}

pub fn get_char() -> char {
    loop {
        if let Some(c) = pop_char() {
            return c;
        }
        core::hint::spin_loop();
    }
}

// Helpers for apps
pub fn is_ctrl_active() -> bool { unsafe { CTRL } }
pub fn is_alt_active() -> bool { unsafe { ALT } }
pub fn is_super_active() -> bool { unsafe { SUPER } }
pub fn is_shift_active() -> bool { unsafe { SHIFT } }

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
                0x1D => if !EXTENDED { CTRL = false; },
                0x38 => if !EXTENDED { ALT = false; },
                _ => {}
            }
            if EXTENDED {
                match code {
                    0x1D => CTRL = false, // R-Ctrl
                    0x38 => ALT = false,  // R-Alt (AltGr)
                    0x5B | 0x5C => SUPER = false,
                    _ => {}
                }
                EXTENDED = false;
            }
            return;
        }

        // Modifier presses
        match code {
            0x2A | 0x36 => { SHIFT = true; return; }
            0x1D => if !EXTENDED { CTRL = true; return; }
            0x38 => if !EXTENDED { ALT = true; return; }
            0x3A => { CAPS = !CAPS; return; }
            0x45 => { NUMLOCK = !NUMLOCK; return; }
            _ => {}
        }
        
        if EXTENDED {
            match code {
                0x1D => { CTRL = true; EXTENDED = false; return; } // R-Ctrl
                0x38 => { ALT = true; EXTENDED = false; return; }  // R-Alt
                0x5B | 0x5C => { SUPER = true; EXTENDED = false; return; }
                0x5D => { EXTENDED = false; KEY_BUFFER.lock().push(KEY_MENU); return; }
                _ => {}
            }
        }

            let ch = if EXTENDED {
                EXTENDED = false;
                match code {
                    0x48 => Some(KEY_UP),
                    0x50 => Some(KEY_DOWN),
                    0x4B => Some(KEY_LEFT),
                    0x4D => Some(KEY_RIGHT),
                    0x47 => Some(KEY_HOME),
                    0x4F => Some(KEY_END),
                    0x49 => Some(KEY_PGUP),
                    0x51 => Some(KEY_PGDN),
                    0x52 => Some(KEY_INS),
                    0x53 => Some(KEY_DEL),
                    _ => None,
                }
            } else {
                let mut c = decode_scancode(code);
                if CTRL {
                    if let Some(ch) = c {
                        match ch {
                            'c' | 'C' => {
                                crate::process::scheduler::post_signal(crate::process::scheduler::get_current_pid(), crate::process::task::SIGINT);
                                c = Some('\x03'); // Still push char for polling apps
                            },
                            'z' | 'Z' => {
                                crate::process::scheduler::post_signal(crate::process::scheduler::get_current_pid(), crate::process::task::SIGTSTP);
                                c = Some('\x1A');
                            },
                            _ => {
                                if ch >= 'a' && ch <= 'z' {
                                    c = Some((ch as u8 - b'a' + 1) as char);
                                } else if ch >= 'A' && ch <= 'Z' {
                                     c = Some((ch as u8 - b'A' + 1) as char);
                                }
                            }
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
            0x01 => Some('\x1B'), // Escape
            
            // F-Keys
            0x3B => Some(KEY_F1), 0x3C => Some(KEY_F2), 0x3D => Some(KEY_F3), 0x3E => Some(KEY_F4),
            0x3F => Some(KEY_F5), 0x40 => Some(KEY_F6), 0x41 => Some(KEY_F7), 0x42 => Some(KEY_F8),
            0x43 => Some(KEY_F9), 0x44 => Some(KEY_F10),
            0x57 => Some(KEY_F11), 0x58 => Some(KEY_F12),
            // Number Row
            0x02 => Some('1'), 0x03 => Some('2'), 0x04 => Some('3'), 0x05 => Some('4'),
            0x06 => Some('5'), 0x07 => Some('6'), 0x08 => Some('7'), 0x09 => Some('8'),
            0x0A => Some('9'), 0x0B => Some('0'),
            0x0C => Some('-'), 0x0D => Some('='), 0x0E => Some('\x08'), // Backspace

            // QWERT...
            0x0F => Some('\t'),
            0x10 => Some('q'), 0x11 => Some('w'), 0x12 => Some('e'), 0x13 => Some('r'),
            0x14 => Some('t'), 0x15 => Some('y'), 0x16 => Some('u'), 0x17 => Some('i'),
            0x18 => Some('o'), 0x19 => Some('p'), 0x1A => Some('['), 0x1B => Some(']'),
            0x1C => Some('\n'),

            // ASDF...
            0x1E => Some('a'), 0x1F => Some('s'), 0x20 => Some('d'), 0x21 => Some('f'),
            0x22 => Some('g'), 0x23 => Some('h'), 0x24 => Some('j'), 0x25 => Some('k'),
            0x26 => Some('l'), 0x27 => Some(';'), 0x28 => Some('\''), 0x29 => Some('`'),
            0x2B => Some('\\'),

            // ZXCV...
            0x2C => Some('z'), 0x2D => Some('x'), 0x2E => Some('c'), 0x2F => Some('v'),
            0x30 => Some('b'), 0x31 => Some('n'), 0x32 => Some('m'), 0x33 => Some(','),
            0x34 => Some('.'), 0x35 => Some('/'),
            
            // Numpad & Others
            0x39 => Some(' '),
            0x46 => Some(KEY_SCROLL),
            0x47 => if NUMLOCK { Some('7') } else { Some(KEY_HOME) },
            0x48 => if NUMLOCK { Some('8') } else { Some(KEY_UP) },
            0x49 => if NUMLOCK { Some('9') } else { Some(KEY_PGUP) },
            0x4A => Some('-'),
            0x4B => if NUMLOCK { Some('4') } else { Some(KEY_LEFT) },
            0x4C => Some('5'),
            0x4D => if NUMLOCK { Some('6') } else { Some(KEY_RIGHT) },
            0x4E => Some('+'),
            0x4F => if NUMLOCK { Some('1') } else { Some(KEY_END) },
            0x50 => if NUMLOCK { Some('2') } else { Some(KEY_DOWN) },
            0x51 => if NUMLOCK { Some('3') } else { Some(KEY_PGDN) },
            0x52 => if NUMLOCK { Some('0') } else { Some(KEY_INS) },
            0x53 => Some('.'), // Numpad Del
            0x60 => Some('₹'), // Custom mapped keysym 8377
            0x61 => Some('૫'), // Custom mapped keysym 2730
            
            _ => None,
        };

        if let Some(mut c) = base {
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
        } else {
            None
        }
    }
}
