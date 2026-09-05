use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use core::fmt::Write;

const LOG_BUFFER_SIZE: usize = 64 * 1024; // 64KB Ring Buffer

pub struct SystemLog {
    buffer: Vec<u8>,
    head: usize,
}

pub static LOG: Mutex<SystemLog> = Mutex::new(SystemLog {
    buffer: Vec::new(),
    head: 0,
});

impl SystemLog {
    pub fn init(&mut self) {
        self.buffer = alloc::vec![0u8; LOG_BUFFER_SIZE];
    }

    pub fn write_str(&mut self, s: &str) {
        for b in s.as_bytes() {
            self.buffer[self.head] = *b;
            self.head = (self.head + 1) % LOG_BUFFER_SIZE;
        }
    }

    pub fn read_all(&self) -> String {
        let mut s = String::new();
        // Since it's a ring buffer, we should technically read from head onwards
        // but for simplicity, we'll just return what's in there.
        // In a real dmesg, we'd start from the oldest entry.
        for i in 0..LOG_BUFFER_SIZE {
            let idx = (self.head + i) % LOG_BUFFER_SIZE;
            let b = self.buffer[idx];
            if b != 0 {
                s.push(b as char);
            }
        }
        s
    }
}

pub fn log_message(msg: &str) {
    let mut log = LOG.lock();
    log.write_str(msg);
    log.write_str("\n");
}

pub fn init() {
    LOG.lock().init();
    log_message("Sovereign System Log (SSL) Initialized.");
}
