use core::fmt;
use spin::Mutex;
use core::arch::asm;
use alloc::string::String;
use alloc::format;

pub const COM1_BASE: u16 = 0x3F8;
pub const COM2_BASE: u16 = 0x2F8;
pub const COM3_BASE: u16 = 0x3E8;
pub const COM4_BASE: u16 = 0x2E8;

pub struct SerialPort {
    pub port: u16,
}

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn init(&mut self) -> bool {
        unsafe {
            // Disable interrupts
            asm!("out dx, al", in("dx") self.port + 1, in("al") 0x00u8, options(nomem, nostack, preserves_flags));
            // Enable DLAB (set baud rate divisor)
            asm!("out dx, al", in("dx") self.port + 3, in("al") 0x80u8, options(nomem, nostack, preserves_flags));
            // Set divisor to 3 (lo byte) 38400 baud
            asm!("out dx, al", in("dx") self.port + 0, in("al") 0x03u8, options(nomem, nostack, preserves_flags));
            // (hi byte)
            asm!("out dx, al", in("dx") self.port + 1, in("al") 0x00u8, options(nomem, nostack, preserves_flags));
            // 8 bits, no parity, one stop bit
            asm!("out dx, al", in("dx") self.port + 3, in("al") 0x03u8, options(nomem, nostack, preserves_flags));
            // Enable FIFO, clear them, with 14-byte threshold
            asm!("out dx, al", in("dx") self.port + 2, in("al") 0xC7u8, options(nomem, nostack, preserves_flags));
            // IRQs enabled, RTS/DSR set
            asm!("out dx, al", in("dx") self.port + 4, in("al") 0x0Bu8, options(nomem, nostack, preserves_flags));
            // Test loopback mode: set in loopback (bit 4 of MCR)
            asm!("out dx, al", in("dx") self.port + 4, in("al") 0x1Eu8, options(nomem, nostack, preserves_flags));
            // Send byte 0xAE
            asm!("out dx, al", in("dx") self.port + 0, in("al") 0xAEu8, options(nomem, nostack, preserves_flags));
            let mut read_back: u8;
            asm!("in al, dx", out("al") read_back, in("dx") self.port + 0, options(nomem, nostack, preserves_flags));

            // Set in normal operation mode (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
            asm!("out dx, al", in("dx") self.port + 4, in("al") 0x0Fu8, options(nomem, nostack, preserves_flags));

            read_back == 0xAE
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            // Wait for THRE (Transmitter Holding Register Empty)
            let mut status: u8;
            loop {
                asm!("in al, dx", out("al") status, in("dx") self.port + 5, options(nomem, nostack, preserves_flags));
                if status & 0x20 != 0 { break; }
            }
            asm!("out dx, al", in("dx") self.port, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
    
    pub fn data_ready(&self) -> bool {
        unsafe {
            let status: u8;
            asm!("in al, dx", out("al") status, in("dx") self.port + 5, options(nomem, nostack, preserves_flags));
            if status == 0xFF {
                return false;
            }
            (status & 0x01) != 0
        }
    }
    
    pub fn read_byte(&mut self) -> u8 {
        unsafe {
            let data: u8;
            asm!("in al, dx", out("al") data, in("dx") self.port, options(nomem, nostack, preserves_flags));
            data
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

pub static SERIAL: Mutex<SerialPort> = Mutex::new(SerialPort::new(0x3F8));

pub fn print_hex_8(val: u8) {
    use core::fmt::Write;
    let _ = write!(SERIAL.lock(), "{:02X}", val);
}

pub fn print_hex_32(val: u32) {
    use core::fmt::Write;
    let _ = write!(SERIAL.lock(), "{:08X}", val);
}

pub fn get_com_status() -> String {
    let ports = [
        ("COM1", COM1_BASE),
        ("COM2", COM2_BASE),
        ("COM3", COM3_BASE),
        ("COM4", COM4_BASE),
    ];
    let mut out = String::from("UART 16550A Serial Ports:\n");
    for (name, port) in &ports {
        let mut sp = SerialPort::new(*port);
        let ok = sp.init();
        out.push_str(&format!(
            "  {} ({:#x}) : {}\n",
            name, port, if ok { "Online (115200 Baud / Loopback OK)" } else { "Not Present" }
        ));
    }
    out
}
