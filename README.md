<p align="center">
  <img src="logo.png" width="200" alt="Ainux Logo">
</p>

# Ainux Custom Kernel

A modern, Rust-based kernel implementing a hybrid architecture built for high performance and sovereign operation.

## Prerequisites
*   **Rust**: Nightly toolchain (`rustup default nightly`).
*   **Target**: `rustup target add x86_64-unknown-none`
*   **NASM / GCC / Python**: For assembly, userspace C compilation, and LLVM hybrid scripts.
*   **QEMU**: `qemu-system-x86_64` for emulation.
*   **GRUB & Xorriso**: `grub-mkrescue` for ISO creation.

## Building and Running
The easiest way to build and run the kernel using QEMU is the provided script:

```bash
./run.sh
```

This script will:
1.  Assemble the boot, trampoline, and utility code using `nasm`.
2.  Compile the kernel (`cargo build --release --target x86_64-unknown-none`).
3.  Create a bootable ISO (`ainux.iso`) using GRUB.
4.  Create a 32MB EXT4 disk image (`disk2.img`) if missing.
5.  Launch QEMU with KVM (if available), SMP (4 cores), RTL8139 networking, and the ISO+Disk attached.

## Output
*   **Framebuffer**: Graphical output (kernel logs and native shell interface).
*   **Serial (stdio)**: Debug and system logs are printed directly to your host terminal.

## Core Features
*   **Symmetric Multi-Processing (SMP)**: Full support for bootstrapping APs (Application Processors) with robust locking mechanisms and Local APIC setup.
*   **Memory Management**: PMM/VMM with dynamic heap sizing and page table structures.
*   **Filesystem**: EXT4 (ReadOnly) and VFS for file operations.
*   **Networking**: Native `smoltcp` stack integration with RTL8139 driver, providing IPv4/TCP support and a remote SSH Daemon (`sshd`).
*   **Native Shell (`ainux-sh`)**: Fully featured interactive shell with filesystem traversal (`ls`, `cd`, `cat`, etc.), process management (`ps`, `kill`, `top`), and core utilities.
*   **Hardware Fetch (`neofetch`)**: Dynamic system info tool polling raw PCI data to detect specific GPUs, CPU brands, Topology, and Memory parameters with ASCII-art integration.
*   **Userspace**: Ring 3 switching, syscalls, and native application execution (`exec`).
