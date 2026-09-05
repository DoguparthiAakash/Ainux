use core::fmt;
use spin::Mutex;
use core::arch::asm;

pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
             // Wait for THRE (Transmitter Holding Register Empty)
             // Bit 5 of LSR (Line Status Register, Port + 5)
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
