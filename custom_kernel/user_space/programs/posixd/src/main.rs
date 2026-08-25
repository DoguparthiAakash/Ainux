#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> isize {
    libainux::syscalls::sys_write(1, b"POSIX Subsystem (posixd) started...\n");

    // Stub for IPC receiving loop
    // In a real system, posixd would register itself as the handler
    // and wait for IPC messages from the kernel or other apps.
    
    libainux::syscalls::sys_write(1, b"posixd listening for IPC (stubbed)...\n");

    loop {
        // Sleep or wait for IPC
        libainux::syscalls::sys_yield();
    }
}


