// =============================================================================
// Ainux Kernel Log — Lock-Free Ring Buffer for ISR-Safe Logging
// Replaces serial I/O in hot paths with O(1) memory writes.
// Readable via shell `dmesg` command.
// =============================================================================

use core::sync::atomic::{AtomicUsize, Ordering};
use core::fmt;

/// 4KB ring buffer for kernel log messages
const KLOG_SIZE: usize = 4096;

/// Static ring buffer — no allocation, ISR-safe
static mut KLOG_BUF: [u8; KLOG_SIZE] = [0; KLOG_SIZE];
static KLOG_HEAD: AtomicUsize = AtomicUsize::new(0);
static KLOG_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Non-blocking kernel log writer
pub struct KernelLog;

impl KernelLog {
    /// Write a single byte to the ring buffer (ISR-safe, no locks)
    #[inline(always)]
    pub fn write_byte(b: u8) {
        let head = KLOG_HEAD.fetch_add(1, Ordering::Relaxed) % KLOG_SIZE;
        unsafe { KLOG_BUF[head] = b; }
        // Saturate count at KLOG_SIZE
        let _ = KLOG_COUNT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |c| {
            Some(c.min(KLOG_SIZE - 1) + 1)
        });
    }

    /// Write a string to the ring buffer
    pub fn write_str(s: &str) {
        for b in s.bytes() {
            Self::write_byte(b);
        }
    }

    /// Read all available log data into a buffer. Returns bytes written.
    pub fn read(buf: &mut [u8]) -> usize {
        let count = KLOG_COUNT.load(Ordering::Relaxed).min(KLOG_SIZE);
        let head = KLOG_HEAD.load(Ordering::Relaxed) % KLOG_SIZE;
        let start = if count >= KLOG_SIZE {
            head  // wrapped — oldest data is at head
        } else {
            (head + KLOG_SIZE - count) % KLOG_SIZE
        };

        let to_read = count.min(buf.len());
        for i in 0..to_read {
            let idx = (start + i) % KLOG_SIZE;
            buf[i] = unsafe { KLOG_BUF[idx] };
        }
        to_read
    }

    /// Get a snapshot of the log as a string (for shell display).
    /// Returns up to max_len bytes.
    pub fn snapshot(buf: &mut [u8]) -> &str {
        let n = Self::read(buf);
        core::str::from_utf8(&buf[..n]).unwrap_or("<binary>")
    }
}

impl fmt::Write for KernelLog {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        KernelLog::write_str(s);
        Ok(())
    }
}

/// Macro for kernel logging — writes to ring buffer, zero serial I/O
#[macro_export]
macro_rules! klog {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($crate::drivers::klog::KernelLog, $($arg)*);
    }};
}

/// Macro for kernel logging with serial echo (boot-time only)
#[macro_export]
macro_rules! klog_serial {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($crate::drivers::klog::KernelLog, $($arg)*);
        let mut _serial = $crate::drivers::serial::SerialPort::new(0x3F8);
        let _ = write!(_serial, $($arg)*);
    }};
}
