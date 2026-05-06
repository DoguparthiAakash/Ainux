# Ainux Kernel Documentation

Welcome to the official documentation for the **Ainux Kernel**, an industrialized, sovereign operating system designed for high-performance networked environments.

## System Overview
Ainux is a monolithic kernel with a modular driver architecture, written primarily in Rust with performance-critical sections in Assembly and Zig. It features a custom SMP scheduler, a robust VFS layer, and a native TCP/IP stack.

## Documentation Contents

### 1. Architecture Specifics
*   [AI System Architecture](../AI_SYSTEM_ARCHITECTURE.md): Comprehensive UML diagrams and system structures tailored for AI IDEs and advanced analysis.
*   [x86_64 Architecture](arch/x86_64.md): Boot process, GDT/IDT, and memory models.

### 2. Core Subsystems
*   [Process Management](process/scheduler.md): The SMP scheduler, task switching, and threading.
*   [Memory Management](memory/management.md): Physical memory allocation, paging, and the kernel heap.

### 3. File Systems & IPC
*   [Virtual File System (VFS)](fs/vfs.md): The unified file interface.
*   [Pipes & Redirection](fs/pipes.md): Inter-process communication via anonymous streams.

### 4. Networking
*   [Networking Stack](net/networking.md): smoltcp integration, socket layer, and port binding.

### 5. Driver Model
*   [Device Drivers](dev/drivers.md): Information on Video (VBE), Network (RTL8139), and Disk (ATA) drivers.

### 6. Administration & Booting
*   [Boot Process](admin/boot.md): GRUB configuration, ISO creation, and early initialization.

---
*Ainux Kernel v0.2.0 - "Sovereign Industrialization"*
