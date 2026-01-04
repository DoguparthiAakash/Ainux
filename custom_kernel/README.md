# Ainux Custom Kernel

A modular, Rust-based kernel implementing a hybrid architecture.

## Prerequisites
*   **Rust**: Nightly toolchain (`rustup default nightly`).
*   **Target**: `rustup target add x86_64-unknown-none`
*   **QEMU**: `qemu-system-x86_64` for emulation.
*   **Xorriso**: For building the ISO image.
*   **Limine**: Bootloader (included in repository).

## Building and Running
The easiest way to run the kernel is using the provided script:

```bash
./run.sh
```

This script will:
1.  Compile the kernel (`cargo build --release`).
2.  Create a bootable ISO (`ainux.iso`).
3.  Create a 32MB EXT4 disk image (`disk.img`) if missing.
4.  Launch QEMU with the ISO and Disk attached.

## Output
*   **Framebuffer**: Graphical output (kernel logs).
*   **Serial (stdio)**: Debug logs are printed to your terminal.

## Features
*   **PMM & VMM**: Full paging support.
*   **Multitasking**: Preemptive/Cooperative scheduler.
*   **Filesystem**: EXT4 (ReadOnly Superblock parser) & ATA Driver.
*   **Userspace**: Ring 3 switching and Syscalls.
