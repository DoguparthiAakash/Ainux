/// BSD-Rust subsystem — POSIX personality running over the Mach microkernel.
///
/// This is the Rust analogue of XNU's BSD layer.  It owns:
///   - POSIX process semantics (fork, exec, wait, signals)
///   - The VFS mount table (delegated to fs/vfs.rs)
///   - The socket/networking API (delegated to net/mod.rs)
///
/// It does *not* implement drivers; those live in drivers/ and are accessed
/// via the IOKit registry.

use core::fmt::Write;

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "BSD Subsystem: Initializing POSIX personality...\n");

    // Signal table is per-task; initialized on task creation.
    // Nothing global to set up here beyond announcing readiness.

    let _ = write!(serial, "BSD Subsystem: Ready. POSIX personality active.\n");
}

/// Return the BSD subsystem version string (analogous to kern.osrelease).
pub fn os_release() -> &'static str {
    "Mithl OS BSD 1.0 (XNU-Hybrid)"
}
