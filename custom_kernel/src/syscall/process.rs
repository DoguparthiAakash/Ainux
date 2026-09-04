pub fn sys_fork() -> isize {
    // Stub for process creation. A real implementation would:
    // 1. Clone the current process's address space (page tables).
    // 2. Clone file descriptors and other state.
    // 3. Create a new task in the scheduler.
    // 4. Return the child's PID to the parent, and 0 to the child.
    
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_fork stub called\n");
    
    -38 // ENOSYS
}

pub fn sys_execve(_filename: *const u8, _argv: *const *const u8, _envp: *const *const u8) -> isize {
    // Stub for executing a new program. A real implementation would:
    // 1. Read the executable file (ELF) from VFS.
    // 2. Parse ELF headers.
    // 3. Replace the current process's address space with the new one.
    // 4. Set up the stack with arguments and environment.
    // 5. Jump to the entry point (e_entry).
    
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_execve stub called\n");
    
    -38 // ENOSYS
}

pub fn sys_wait4(_pid: i32, _wstatus: *mut i32, _options: i32, _rusage: *mut u8) -> isize {
    // Stub for waiting for a child process to change state.
    
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_wait4 stub called\n");
    
    -38 // ENOSYS
}
