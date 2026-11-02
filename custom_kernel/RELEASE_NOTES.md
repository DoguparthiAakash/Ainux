# Mithl OS — Developer Testing Release (v0.3-alpha)

This is a non-commercial, experimental developer preview of **Mithl OS**, built upon the custom sovereign **[Ainux Kernel](https://github.com/DoguparthiAakash/Ainux)** written in bare-metal Rust (`x86_64-unknown-none`). 

This release is intended for testing, kernel evaluation, and community collaboration. If you are interested in contributing, testing drivers, or porting software to Mithl OS, please reach out via email: **[doguparthiaakash@gmail.com](mailto:doguparthiaakash@gmail.com)**.

---

## 🚀 Key Highlights & New Features

### 🖥️ Desktop Environment & Window Compositor
- **Windows-Style Desktop Interface**: Complete graphical desktop environment featuring a modern acrylic taskbar, Start Menu, draggable desktop icons, and interactive right-click context menu.
- **Windows File Explorer**:
  - Command & Navigation Ribbon: Back (`<`), Forward (`>`), Up (`^`), Refresh (`R`).
  - Breadcrumb Address Bar (`This PC > ...`) and real-time Search Bar.
  - Quick Access Sidebar: Instant jump to root `/`, `/home`, `/bin`, `/dev`, `/etc`, and `/tmp`.
  - Detailed List View: File name, file type, file size columns with color-coded classification (`[DIR]`, `[DOC]`, `[EXE]`, `[FIL]`).
  - History navigation stack and directory traversal (double-click / Enter / Backspace).
- **Fixed Window Resizing**:
  - Replaced coordinate-scaled raster blitting with direct 1:1 pixel viewport blitting.
  - Window borders drag-resize cleanly without content magnification or blur.
  - Dynamic `ResizeBuffer` IPC: window manager dispatches resize events and apps dynamically allocate buffers to fit viewport dimensions.

### 🔊 Audio & Hardware Support
- **AC97 Audio Engine**: Hardware-timed DMA playback with double-buffering. Plays polyphonic system startup chimes directly on boot.
- **PC Speaker Fallback**: Low-level programmable interval timer (PIT 8254) tone generation.
- **PS/2 Mouse & Keyboard**: High-precision cursor movement with boundary clamping, left/right click, and multi-key scancode decoder.
- **VESA / GOP Display**: 800x600 24-bit / 32-bit linear framebuffer rendering.

### 📦 Pre-Packaged Applications
- **File Explorer**: Windows Explorer inspired hierarchical file browser.
- **MithlPad (Notepad)**: Text editor with keyboard typing and buffer rendering.
- **Terminal Emulator**: Command shell integrated with kernel VFS and IPC.
- **Task Manager & System Monitor**: Live process inspect, memory allocation, and CPU load tracker.
- **Settings App**: Display resolution, theme, wallpaper, and system information controls.
- **Paint App**: Canvas drawing utility with color palettes.
- **Entertainment Suite**: Minesweeper, Tetris, Pong, 2048, Snake, Chess, Sudoku.

---

## 💿 Available Distribution Images

| Image File | Target Platform | Description |
|---|---|---|
| **`mithl.iso`** | UEFI / BIOS & Universal | Bootable GRUB Multiboot hybrid ISO for bare-metal or hypervisors. |
| **`mithl.vdi`** | Oracle VirtualBox | Native VirtualBox virtual disk image with Ext4 filesystem. |
| **`mithl.vmdk`** | VMware Workstation / Player / ESXi | Native VMware virtual machine disk image. |

---

## 🧪 Tested Hypervisors & Compatibility

| Platform | Status | Graphics | Audio | Mouse / Keyboard |
|---|:---:|:---:|:---:|:---:|
| **QEMU (v7.0+)** | ✅ Verified | VESA std (800x600) | AC97 / PC Speaker | PS/2 Smooth |
| **Oracle VirtualBox (v6.1+)** | ✅ Verified | VBoxVGA / VMSVGA | AC97 (ICH AC97) | PS/2 Mouse |
| **VMware Workstation / Player** | ✅ Verified | Auto / VESA | Auto Sound | PS/2 Mouse |

---

## ⚡ Getting Started / How to Run

### Option 1: QEMU (Recommended)
#### On Windows (PowerShell):
```powershell
.\start.ps1
```
#### On Linux / WSL:
```bash
./start.sh
```

### Option 2: Oracle VirtualBox
1. Open VirtualBox and click **New**.
2. **Type**: `Other`, **Version**: `Other/Unknown (64-bit)`.
3. Set **RAM** to at least `2048 MB (2 GB)` and **Processors** to `2` or `4`.
4. In **Storage**, attach `mithl.iso` to the Optical Drive (or attach `mithl.vdi` as primary Hard Disk).
5. In **Audio**, select **ICH AC97** audio controller.
6. Click **Start**.

### Option 3: VMware Workstation / Player
1. Create a **New Virtual Machine** (`Custom` -> `Other 64-bit`).
2. Assign `2 GB` memory and `2` CPU cores.
3. Attach `mithl.iso` as CD/DVD drive (or use `mithl.vmdk` as virtual disk).
4. Power on virtual machine.

---

## ⚠️ Known Limitations & Work in Progress

- **Networking**: RTL8139 network stack and DHCP loopback active; browser app currently operates in offline preview mode.
- **Ext4 Write Back**: Ext4 read/lookup functional; root disk synchronization in progress.
- **SMP Scaling**: Core APIC initialization stable on up to 4 virtual CPU cores.

---

## 🤝 Contributing & Contact

Mithl OS is an open-source research operating system. We welcome OS enthusiasts, kernel developers, Rust programmers, and testers.

- **Author & Maintainer**: Doguparthi Aakash
- **Ainux Kernel Repository**: [https://github.com/DoguparthiAakash/Ainux](https://github.com/DoguparthiAakash/Ainux)
- **Contact Email**: [doguparthiaakash@gmail.com](mailto:doguparthiaakash@gmail.com)
