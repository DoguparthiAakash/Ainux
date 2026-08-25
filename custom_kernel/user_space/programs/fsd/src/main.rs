#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    libainux::syscalls::sys_write(1, b"File System Daemon (fsd) started...\n");
    libainux::syscalls::sys_write(1, b"fsd listening for IPC (stubbed)...\n");

    loop {
        // Sleep or wait for IPC
        libainux::syscalls::sys_yield();
    }
}


