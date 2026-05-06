# Ainux Kernel AI Context & System Architecture

> **Notice to AI Assistants:** This document contains the sovereign architectural blueprint of the Ainux Custom Kernel. Use this file to understand the system layout, core data structures, control flows, and development guidelines.

## 1. Project Overview

Ainux is a modern, high-performance hybrid kernel built primarily in **Rust (nightly, `#![no_std]`)**, targeting the `x86_64-unknown-none` bare-metal architecture. It utilizes a modular monolithic design with support for Symmetric Multi-Processing (SMP), Virtual File Systems (VFS), a native shell (`ainux-sh`), and an integrated TCP/IPv4 networking stack via `smoltcp`.

### Core Technologies
- **Language:** Rust (2021 Edition, `#![no_std]`, `#![no_main]`), Assembly (NASM), C (for Userspace)
- **Toolchain:** `rustup target add x86_64-unknown-none`
- **Bootloader:** GRUB (Multiboot2)
- **Emulation:** QEMU with KVM

---

## 2. Directory Structure & Module Mapping

When asked to modify a feature, refer to this mapping:

- `src/main.rs`: Kernel entry point (`_start`), early initialization, architecture bootstrap.
- `src/process/`: Multitasking, SMP Scheduler (Round-Robin/Fair), task switching, threading.
- `src/mm/`: Physical Memory Management (PMM) via Zoned Buddy Allocator, Virtual Memory Management (VMM), Page Tables (ML4T).
- `src/fs/`: Virtual File System (VFS), Pipes, and concrete implementations (Ext4, Btrfs, initramfs).
- `src/net/`: Networking stack (`smoltcp`), Socket handling, dynamic port binding.
- `src/cpu/`: CPU intrinsics, GDT, IDT, Syscall dispatcher, APIC/LAPIC initialization.
- `src/drivers/`: Hardware Abstraction Layer (HAL).
  - `video`: VBE/BGA Graphics.
  - `net`: RTL8139, Atheros Ethernet.
  - `storage`: ATA PIO/DMA, AHCI.
- `src/apps/`: Native kernel-mode ring 3 binaries (`httpd`, `sshd`, `fetch`).
- `src/shell/`: `ainux-sh` interface, builtin commands (`ls`, `cd`, `cat`, `ps`, `top`).

---

## 3. UML Architectural Designs

### 3.1. Overall Kernel Architecture

```mermaid
graph TD
    subgraph "Ring 3 (Userspace)"
        Shell["Ainux Shell (A-SH)"]
        Apps["Native Apps & Daemons"]
        UI["NuxV Graphics Engine"]
    end

    subgraph "Syscall Interface"
        IDT["Interrupt Descriptor Table"]
        Sys["Syscall Dispatcher"]
    end

    subgraph "Ring 0 (Kernel Core)"
        Sched["SMP Task Scheduler"]
        VFS["Virtual File System"]
        Net["smoltcp Network Stack"]
        Mem["Zoned Buddy Allocator & Paging"]
    end

    subgraph "Hardware Abstraction Layer"
        DRV_V["Video Driver (VBE)"]
        DRV_N["NIC Driver (RTL8139)"]
        DRV_D["Disk Driver (ATA/AHCI)"]
    end

    subgraph "Hardware Base"
        CPU["x86_64 AP/BSP Cores"]
        RAM["Physical Memory"]
        HW_NIC["Network Cards"]
        HW_STO["Storage Devices"]
    end

    Shell --> IDT
    Apps --> IDT
    UI --> IDT
    IDT --> Sys
    Sys --> Sched
    Sys --> VFS
    Sys --> Net
    Sys --> Mem
    VFS --> DRV_D
    Net --> DRV_N
    UI --> DRV_V
    Sched --> CPU
    Mem --> RAM
    DRV_D --> HW_STO
    DRV_N --> HW_NIC
```

### 3.2. Boot Flow & SMP Initialization

```mermaid
sequenceDiagram
    participant BIOS as BIOS/UEFI
    participant GRUB as GRUB (Multiboot2)
    participant BSP as Bootstrap Processor (BSP)
    participant APs as Application Processors (APs)

    BIOS->>GRUB: Load Bootloader
    GRUB->>BSP: Enter `_start` (32-bit to 64-bit Long Mode)
    BSP->>BSP: Setup GDT, IDT, and Page Tables
    BSP->>BSP: Initialize Physical Memory (Zoned Buddy)
    BSP->>BSP: Initialize LAPIC & IOAPIC
    BSP->>APs: Send INIT & SIPI IPIs
    APs-->>BSP: Acknowledge & Boot via Trampoline Code
    BSP->>BSP: Initialize VFS & Drivers
    BSP->>BSP: Spawn `init` process (Ainux Shell)
    BSP->>BSP: Enter Scheduler Loop
    APs->>APs: Enter Scheduler Loop
```

### 3.3. Memory Management: Zoned Buddy Allocator

```mermaid
classDiagram
    class ZonedBuddyAllocator {
        +zones: Array~Zone~
        +init(memory_map)
        +alloc_pages(order, zone_type)
        +free_pages(ptr, order)
    }

    class Zone {
        +zone_type: ZoneType
        +start_addr: usize
        +end_addr: usize
        +free_lists: Array~LinkedList~
        +lock: Spinlock
        +allocate(order)
        +free(addr, order)
        -split(order)
        -merge(addr, order)
    }

    class ZoneType {
        <<enumeration>>
        DMA
        NORMAL
        HIGHMEM
    }

    ZonedBuddyAllocator *-- Zone
    Zone o-- ZoneType
```

### 3.4. VFS and Inter-Process Communication (Pipes)

```mermaid
sequenceDiagram
    participant P1 as Process 1 (Writer)
    participant Sys as Syscall Layer
    participant Pipe as Pipe Inode (VFS)
    participant P2 as Process 2 (Reader)

    Note over P1, P2: Execution of command `ls | grep`

    P1->>Sys: sys_write(FD_OUT, data)
    Sys->>Pipe: write_to_buffer()
    Pipe->>Pipe: Acquire Spinlock
    Pipe->>Pipe: Enqueue Data
    Pipe->>Pipe: Wake Reader Thread
    Pipe-->>Sys: Return written bytes
    Sys-->>P1: Ok()

    P2->>Sys: sys_read(FD_IN, buffer)
    Sys->>Pipe: read_from_buffer()
    Pipe->>Pipe: Acquire Spinlock
    Pipe->>Pipe: Dequeue Data
    Pipe-->>Sys: Return read bytes
    Sys-->>P2: Ok()
```

---

## 4. Development Guidelines & Rules for AI Agents

When acting on this codebase, adhere to the following rules:

1. **No Standard Library**: The kernel is `#![no_std]`. Use `core::` instead of `std::`. For heap allocation, use `alloc::vec::Vec`, `alloc::string::String`, etc. (Requires `extern crate alloc`).
2. **Synchronization**: Since the kernel operates in an SMP (multicore) environment, NEVER use regular mutable static variables. Always use `spin::Mutex` or `spin::RwLock` to protect shared resources. Avoid deadlocks by minimizing lock scope.
3. **Interrupts**: When holding a spinlock in kernel space, it is often necessary to disable interrupts (`x86_64::instructions::interrupts::without_interrupts`) to prevent interrupt handlers from deadlocking the CPU.
4. **Error Handling**: Use `Result` heavily. Kernel panics should be reserved only for unrecoverable hardware or boot state failures.
5. **Memory Safety**: Limit the use of `unsafe` blocks. Only use them for direct hardware MMIO, raw pointers from FFI/Assembly, or low-level allocator manipulation. Document why `unsafe` is used.
6. **Testing**: QEMU is the primary test environment. To build and run, execute `./run.sh`.

## 5. Network Stack Integration
The network relies on `smoltcp`. Hardware packet reception is usually interrupt-driven (or polled). The `net` module abstracts the RTL8139 PCIe device and passes raw Ethernet frames to `smoltcp::iface::Interface`.

```mermaid
graph LR
    NIC[RTL8139 Driver] -->|Raw Frames| RX_Buf[RX Ring Buffer]
    RX_Buf --> NetManager[Net Subsystem / Poller]
    NetManager --> smoltcp[smoltcp Interface]
    smoltcp -->|TCP/UDP Payload| Sockets[Socket Handles]
    Sockets --> Syscalls[sys_read / sys_write]
```

## 6. How to Extend the Kernel
- **Adding a Syscall:** 
  1. Define the syscall ID in `src/cpu/syscalls.rs`.
  2. Add the handler function matching the ID.
  3. Map the interrupt/syscall entry in ASM or IDT.
- **Adding a Command:** Add it to the `src/shell/shell.rs` command match block.

*End of AI Context Document.*
