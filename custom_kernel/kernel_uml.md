# Ainux Kernel: Industrial Architectural Blueprint

This document provides a multi-dimensional view of the Ainux Kernel's design, capturing the essence of its sovereign and industrialized architecture.

## 1. Overall System Architecture
The big picture: How Ainux sits between the hardware and the user.

```mermaid
graph TD
    subgraph "User Space (Ring 3)"
        Shell["Ainux Shell (A-SH)"]
        Apps["Native Apps (httpd, sshd, fetch)"]
        UI["TUI / Graphics Engine (NuxV)"]
    end

    subgraph "System Call Interface"
        Sys["Syscall Dispatcher (ID 0-11)"]
    end

    subgraph "Kernel Core (Ring 0)"
        direction TB
        Sched["SMP Scheduler (RR/Fair)"]
        VFS["Virtual File System (VFS)"]
        Net["Network Stack (smoltcp)"]
        Mem["Memory Manager (Heap/Paging)"]
    end

    subgraph "Hardware Abstraction Layer (HAL)"
        DRV_V["Video (VBE/BGA)"]
        DRV_N["Net (RTL8139/Atheros)"]
        DRV_D["Disk (ATA/AHCI)"]
        DRV_K["Keyboard (PS2/USB)"]
    end

    subgraph "Hardware"
        HW_CPU["CPU (x86_64 / SMP)"]
        HW_RAM["RAM (Physical Memory)"]
        HW_NIC["Network Cards"]
        HW_STO["Storage Devices"]
    end

    Shell --> Sys
    Apps --> Sys
    Sys --> Sched
    Sys --> VFS
    Sys --> Net
    VFS --> DRV_D
    Net --> DRV_N
    Sched --> HW_CPU
    Mem --> HW_RAM
    DRV_V --> HW_CPU
```

---

## 2. Functional Architecture: The IPC & Pipe Flow
Detailed view of how data moves through the new Pipe infrastructure.

```mermaid
sequenceDiagram
    participant P1 as Process A (Writer)
    participant K as Kernel (VFS/Pipe)
    participant P2 as Process B (Reader)

    Note over P1, P2: Multi-stage Pipe Chain (ls | grep)

    P1->>K: sys_write(FD_OUT, buffer)
    K->>K: Mutex::lock(PipeBuffer)
    K->>K: VecDeque::push_back(data)
    K->>K: Wakeup(ReaderThread)
    K-->>P1: Ok(written_size)

    P2->>K: sys_read(FD_IN, buffer)
    K->>K: Mutex::lock(PipeBuffer)
    K->>K: VecDeque::pop_front(data)
    K-->>P2: Ok(read_size)
    Note right of P2: Process B processes filtered stream
```

---

## 3. Network Feature Architecture
Dynamic port binding and service hosting.

```mermaid
graph LR
    subgraph "Network Subsystem"
        Stack["smoltcp Stack"]
        Socket["Socket Manager"]
        Binding["Dynamic Port Binder"]
    end

    subgraph "Services"
        HTTP["HTTPD (Port 8080)"]
        SSH["SSHD (Port 22)"]
    end

    HTTP --> Binding
    SSH --> Binding
    Binding --> Socket
    Socket --> Stack
    Stack --> NIC["RTL8139 Driver"]
```

---

## 4. Feature Matrix (Capability View)

| Category | Feature | Status | Implementation Detail |
| :--- | :--- | :--- | :--- |
| **Core** | SMP Support | ✅ Active | Multiprocessor bootstrap (AP Trampoline) |
| **Core** | Paging | ✅ Active | 4-level paging (ML4T) |
| **Filesystem** | VFS | ✅ Active | Unified inode/handle abstraction |
| **Filesystem** | Ext4 | ✅ Active | Read/Write support via debugfs injection |
| **Filesystem** | Pipes | ✅ Active | Synchronized circular buffers (`|`) |
| **Network** | TCP/IPv4 | ✅ Active | smoltcp integration |
| **Network** | Dynamic Ports | ✅ Active | Multi-service socket management |
| **UI** | Graphics | ✅ Active | VBE/BGA high-res support |
| **Security** | Signals | 🚧 Planned | `sys_kill`, `SIGINT` handling |
| **Advanced** | ASOA | 🚧 Planned | Sovereign State Restoration (ASOA) |

---

## 5. Directory Mapping
| Module | Path | Responsibility |
| :--- | :--- | :--- |
| `Process` | `src/process/` | Scheduler, Task Switching, Threading |
| `Filesystem`| `src/fs/` | VFS, Ext4, Btrfs, Pipes |
| `Network` | `src/net/` | smoltcp wrappers, Socket handles |
| `CPU` | `src/cpu/` | GDT, IDT, Syscalls, Interrupts |
| `Drivers` | `src/drivers/` | Hardware interaction (Video, Net, ATA) |
| `Apps` | `src/apps/` | Native kernel-mode binaries (httpd, sshd) |
