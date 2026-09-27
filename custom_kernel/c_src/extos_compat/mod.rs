/// ExtOS Compatibility Layer — XNU-style bridge
///
/// This module sits between:
///   - The Mach Microkernel  (task/port/IPC layer)
///   - The IOKit Driver Tree (hardware abstraction)
///   - The POSIX ExtOS Layer   (file descriptors, sockets, signals)
///
/// Mirrors how macOS XNU connects its three kernels.

use alloc::sync::Arc;
use core::fmt::Write;

// ─── ExtOS Kernel Name Ports ────────────────────────────────────────────────────
// Each major subsystem registers a well-known Mach port so that user-space
// daemons and other subsystems can send it messages.

/// Port ID for the ExtOS filesystem/VFS subsystem.
pub static ExtOS_VFS_PORT: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(usize::MAX);

/// Port ID for the ExtOS network subsystem (routes socket calls).
pub static ExtOS_NET_PORT: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(usize::MAX);

/// Port ID for the ExtOS process (signals/wait) subsystem.
pub static ExtOS_PROC_PORT: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(usize::MAX);

// ─── Init ─────────────────────────────────────────────────────────────────────

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "ExtOS Compat: Initializing XNU-style hybrid bridge...\n");

    // 1. Create well-known Mach ports for each ExtOS subsystem.
    //    Kernel PID 0 (root task) owns these ports.
    let vfs_port  = crate::microkernel::ipc::create_port(0);
    let net_port  = crate::microkernel::ipc::create_port(0);
    let proc_port = crate::microkernel::ipc::create_port(0);

    ExtOS_VFS_PORT .store(vfs_port,  core::sync::atomic::Ordering::SeqCst);
    ExtOS_NET_PORT .store(net_port,  core::sync::atomic::Ordering::SeqCst);
    ExtOS_PROC_PORT.store(proc_port, core::sync::atomic::Ordering::SeqCst);

    let _ = write!(serial,
        "ExtOS Compat: VFS port={}, Net port={}, Proc port={}\n",
        vfs_port, net_port, proc_port
    );

    // 2. Connect IOKit network drivers → ExtOS net port.
    //    When the RTL8139 / E1000 driver receives a packet it will enqueue a
    //    Mach message on ExtOS_NET_PORT for smoltcp to process.
    connect_iokit_net_driver();

    // 3. Expose POSIX IPC port for userspace POSIX servers (fsd, posixd).
    //    Syscall handlers in arch/x86_64/cpu/syscall.rs read this port to
    //    route mkdir/unlink/stat to the right subsystem.
    crate::ipc::port::POSIX_SUBSYSTEM_PORT
        .store(vfs_port, core::sync::atomic::Ordering::SeqCst);

    let _ = write!(serial, "ExtOS Compat: Hybrid bridge active.\n");
}

// ─── IOKit → ExtOS Net bridge ───────────────────────────────────────────────────

fn connect_iokit_net_driver() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);

    // Walk the IOKit registry. If a network device is attached, record it so
    // the ExtOS net stack knows a real NIC exists.
    let registry = crate::drivers::iokit::registry::REGISTRY.lock();
    if let Some(root) = &registry.root {
        let children = root.children.lock();
        for child in children.iter() {
            let name = child.service.get_name();
            if name.contains("RTL")
                || name.contains("E1000")
                || name.contains("Atheros")
                || name.contains("mt7601")
            {
                let _ = write!(serial,
                    "ExtOS Compat: IOKit NIC '{}' registered with ExtOS net stack.\n",
                    name
                );
                // The smoltcp stack in net/mod.rs already polls RTL8139/E1000
                // directly; this registration is the architectural handshake.
                NET_DRIVER_REGISTERED.store(true, core::sync::atomic::Ordering::SeqCst);
            }
        }
    }
}

/// Set when at least one NIC has been registered with the ExtOS net stack.
pub static NET_DRIVER_REGISTERED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

// ─── Mach → ExtOS dispatch helper ───────────────────────────────────────────────

/// Called from the Mach trap handler (`handle_mach_trap`) for messages
/// addressed to well-known ExtOS ports.  Returns the reply value.
pub fn dispatch_extos_message(port_id: usize, msg: &crate::microkernel::ipc::MachMessage) -> isize {
    let vfs_port  = ExtOS_VFS_PORT .load(core::sync::atomic::Ordering::SeqCst);
    let net_port  = ExtOS_NET_PORT .load(core::sync::atomic::Ordering::SeqCst);
    let proc_port = ExtOS_PROC_PORT.load(core::sync::atomic::Ordering::SeqCst);

    if port_id == vfs_port {
        dispatch_vfs_message(msg)
    } else if port_id == net_port {
        dispatch_net_message(msg)
    } else if port_id == proc_port {
        dispatch_proc_message(msg)
    } else {
        -1 // Unknown port
    }
}

fn dispatch_vfs_message(msg: &crate::microkernel::ipc::MachMessage) -> isize {
    // msg_id encodes the VFS operation (open=2, read=0, write=1, …)
    // For now acknowledge and return success; real routing done in-kernel.
    let _ = msg;
    0
}

fn dispatch_net_message(msg: &crate::microkernel::ipc::MachMessage) -> isize {
    let _ = msg;
    0
}

fn dispatch_proc_message(msg: &crate::microkernel::ipc::MachMessage) -> isize {
    let _ = msg;
    0
}
