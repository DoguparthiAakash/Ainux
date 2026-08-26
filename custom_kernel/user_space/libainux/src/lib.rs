#![no_std]
#![feature(alloc_error_handler)]

pub mod syscalls;
pub mod env;

use core::panic::PanicInfo;
use core::fmt;

pub struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        syscalls::sys_write(1, s.as_bytes());
        Ok(())
    }
}

pub fn print_args(args: core::fmt::Arguments) {
    use core::fmt::Write;
    let _ = Writer.write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print_args(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

// Simple bump allocator over mmap for simple tasks if needed.
// For now, no allocator by default to keep it simple, 
// unless we really need `alloc`. Let's stick to no-alloc first.

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    syscalls::sys_exit(1);
}

// We provide _start here so programs don't have to duplicate it.
// Programs must define `#[no_mangle] pub extern "C" fn main() -> isize`
unsafe extern "C" {
    fn main() -> isize;
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    env::init(argc, argv);
    let ret = unsafe { main() };
    syscalls::sys_exit(ret);
}
