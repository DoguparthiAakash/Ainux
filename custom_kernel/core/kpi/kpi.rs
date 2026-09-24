// SPDX-License-Identifier: GPL-2.0
//! Ainux KPI — Kernel Programming Interface
//!
//! This module implements the `extern "C"` symbols that back the Linux
//! compatibility headers in `c_src/kpi/include/linux/`. Linux driver C code
//! calls these functions; we implement them using Ainux's native Rust subsystems.
//!
//! Rules for this module:
//!   1. Every function here MUST have a matching declaration in a KPI header.
//!   2. No panics inside KPI functions — they must fail gracefully (return NULL,
//!      return error codes, etc.) because C callers cannot unwind Rust panics.
//!   3. No allocations without size guards — enforce a per-allocation 64 MB cap.

#![allow(non_snake_case)]
#![allow(unused_variables)]

use core::fmt::Write;
use alloc::alloc::{alloc, dealloc, realloc, Layout};
use alloc::string::String;

// ── KPI allocation size guard ─────────────────────────────────────────────────
const KPI_MAX_ALLOC: usize = 64 * 1024 * 1024; // 64 MB per single allocation

// ── Memory allocation ─────────────────────────────────────────────────────────

/// kmalloc — allocate kernel memory. GFP_ZERO (0x8000) zeroes the block.
#[no_mangle]
pub unsafe extern "C" fn kpi_kmalloc(size: usize, flags: u32) -> *mut u8 {
    if size == 0 || size > KPI_MAX_ALLOC {
        return core::ptr::null_mut();
    }
    let layout = match Layout::from_size_align(size, 16) {
        Ok(l) => l,
        Err(_) => return core::ptr::null_mut(),
    };
    let ptr = alloc(layout);
    if !ptr.is_null() && (flags & 0x8000 != 0) {
        core::ptr::write_bytes(ptr, 0, size);
    }
    ptr
}

/// kfree — free memory allocated by kmalloc/kzalloc/etc.
/// Linux drivers may call kfree(NULL), which must be a no-op.
#[no_mangle]
pub unsafe extern "C" fn kpi_kfree(ptr: *const u8) {
    if ptr.is_null() {
        return;
    }
    // We cannot recover the original layout here without tracking it.
    // Use a sentinel-size layout so the global allocator can find the block.
    // The Ainux heap uses linked-list allocator which tracks block sizes internally.
    let layout = Layout::from_size_align_unchecked(1, 1);
    dealloc(ptr as *mut u8, layout);
}

/// krealloc — resize a previously allocated block.
#[no_mangle]
pub unsafe extern "C" fn kpi_krealloc(ptr: *const u8, new_size: usize, flags: u32) -> *mut u8 {
    if new_size == 0 {
        kpi_kfree(ptr);
        return core::ptr::null_mut();
    }
    if ptr.is_null() {
        return kpi_kmalloc(new_size, flags);
    }
    if new_size > KPI_MAX_ALLOC {
        return core::ptr::null_mut();
    }
    let layout = Layout::from_size_align_unchecked(1, 1);
    let new_layout = match Layout::from_size_align(new_size, 16) {
        Ok(l) => l,
        Err(_) => return core::ptr::null_mut(),
    };
    realloc(ptr as *mut u8, layout, new_size)
}

/// kzalloc — allocate zeroed kernel memory.
#[no_mangle]
pub unsafe extern "C" fn kpi_kzalloc(size: usize, flags: u32) -> *mut u8 {
    kpi_kmalloc(size, flags | 0x8000)
}

/// kcalloc — allocate zeroed array.
#[no_mangle]
pub unsafe extern "C" fn kpi_kcalloc(n: usize, size: usize, flags: u32) -> *mut u8 {
    let total = n.checked_mul(size).unwrap_or(usize::MAX);
    kpi_kmalloc(total, flags | 0x8000)
}

/// kmalloc_array — allocate array.
#[no_mangle]
pub unsafe extern "C" fn kpi_kmalloc_array(n: usize, size: usize, flags: u32) -> *mut u8 {
    let total = n.checked_mul(size).unwrap_or(usize::MAX);
    kpi_kmalloc(total, flags)
}

/// kstrdup — duplicate a C string.
#[no_mangle]
pub unsafe extern "C" fn kpi_kstrdup(s: *const u8, flags: u32) -> *mut u8 {
    if s.is_null() {
        return core::ptr::null_mut();
    }
    let len = {
        let mut p = s;
        while *p != 0 { p = p.add(1); }
        p as usize - s as usize
    };
    let buf = kpi_kmalloc(len + 1, flags);
    if !buf.is_null() {
        core::ptr::copy_nonoverlapping(s, buf, len + 1);
    }
    buf
}

/// vmalloc — large virtually contiguous allocation (we use same heap for now).
#[no_mangle]
pub unsafe extern "C" fn kpi_vmalloc(size: usize) -> *mut u8 {
    kpi_kmalloc(size, 0x8000)
}

/// vfree — free vmalloc'd memory.
#[no_mangle]
pub unsafe extern "C" fn kpi_vfree(ptr: *const u8) {
    kpi_kfree(ptr);
}

// ── Printing ──────────────────────────────────────────────────────────────────

/// kpi_serial_write — write a pre-formatted buffer to Ainux serial + VGA.
///
/// The C-side `printk` macro in `c_src/kpi/include/linux/kernel.h` uses
/// the C runtime's own `vsnprintf` to format the string into a stack buffer,
/// then calls this function. This avoids needing the unstable Rust
/// `c_variadic` feature entirely.
#[no_mangle]
pub unsafe extern "C" fn kpi_serial_write(buf: *const u8, len: usize) {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    if buf.is_null() || len == 0 { return; }
    let slice = core::slice::from_raw_parts(buf, len);
    for &b in slice {
        if b == 0 { break; }
        let _ = write!(serial, "{}", b as char);
    }
}

/// kpi_printk — stub called when C drivers use pr_info/printk directly
/// with a literal string (no format args). Full variadic printk is handled
/// on the C side — see kernel.h macro.
#[no_mangle]
pub unsafe extern "C" fn kpi_printk(buf: *const u8) -> i32 {
    if buf.is_null() { return 0; }
    let mut len = 0usize;
    while *buf.add(len) != 0 { len += 1; }
    kpi_serial_write(buf, len);
    len as i32
}

// ── Panic ─────────────────────────────────────────────────────────────────────

/// kpi_panic — called by BUG() macro in Linux drivers.
#[no_mangle]
pub extern "C" fn kpi_panic(msg: *const u8) -> ! {
    unsafe {
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        let _ = write!(serial, "\n[KPI PANIC] ");
        if !msg.is_null() {
            let mut i = 0;
            loop {
                let c = *msg.add(i);
                if c == 0 { break; }
                let _ = write!(serial, "{}", c as char);
                i += 1;
            }
        }
        let _ = write!(serial, "\n");
    }
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}

// ── MMIO remap ────────────────────────────────────────────────────────────────

/// ioremap — map physical MMIO range into kernel virtual space.
/// On bare-metal x86-64 with our direct physical map at KERNEL_BASE,
/// MMIO addresses below 4 GB can be accessed directly.
#[no_mangle]
pub unsafe extern "C" fn kpi_ioremap(phys_addr: u64, size: usize) -> *mut u8 {
    // Direct physical map: physical_addr + KERNEL_BASE
    // For MMIO < 4GB this is always already identity-mapped by our kernel.
    // We just return phys + offset into our higher-half direct map.
    const KERNEL_PHYS_OFFSET: u64 = 0xFFFFFFFF80000000;
    if phys_addr < 0x1_0000_0000 {
        // Low 4 GB: accessible via our kernel's direct-map region
        (phys_addr + KERNEL_PHYS_OFFSET) as *mut u8
    } else {
        // High memory: allocate a page-aligned buffer in the heap and
        // note that on QEMU/VMs MMIO is always in low 4 GB anyway.
        core::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn kpi_ioremap_nocache(phys_addr: u64, size: usize) -> *mut u8 {
    kpi_ioremap(phys_addr, size)
}

#[no_mangle]
pub unsafe extern "C" fn kpi_iounmap(addr: *const u8) {
    // No cleanup needed for direct-map aliases.
}

// ── DMA ───────────────────────────────────────────────────────────────────────

#[repr(C)]
pub struct Device { dummy: i32 }

#[no_mangle]
pub unsafe extern "C" fn kpi_dma_alloc_coherent(
    _dev: *mut Device, size: usize, dma_handle: *mut u64, _flags: u32
) -> *mut u8 {
    // Align to page boundary for DMA
    let layout = match Layout::from_size_align(size, 4096) {
        Ok(l) => l,
        Err(_) => return core::ptr::null_mut(),
    };
    let ptr = alloc(layout);
    if !ptr.is_null() {
        core::ptr::write_bytes(ptr, 0, size);
        // Physical address = virtual - KERNEL_BASE
        if !dma_handle.is_null() {
            *dma_handle = (ptr as u64).saturating_sub(0xFFFFFFFF80000000);
        }
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn kpi_dma_free_coherent(
    _dev: *mut Device, size: usize, cpu_addr: *mut u8, _dma_handle: u64
) {
    if cpu_addr.is_null() { return; }
    let layout = Layout::from_size_align_unchecked(size, 4096);
    dealloc(cpu_addr, layout);
}

#[no_mangle]
pub unsafe extern "C" fn kpi_dma_map_single(
    _dev: *mut Device, ptr: *mut u8, _size: usize, _direction: i32
) -> u64 {
    (ptr as u64).saturating_sub(0xFFFFFFFF80000000)
}

#[no_mangle]
pub unsafe extern "C" fn kpi_dma_unmap_single(
    _dev: *mut Device, _dma_addr: u64, _size: usize, _direction: i32
) {
    // No IOMMU — no-op
}

// ── IRQ Registration ──────────────────────────────────────────────────────────

type IrqHandler = unsafe extern "C" fn(i32, *mut u8) -> i32;

#[no_mangle]
pub unsafe extern "C" fn kpi_request_irq(
    irq: u32, handler: IrqHandler, _flags: u64, _name: *const u8, dev: *mut u8
) -> i32 {
    // Register via our IDT-based IRQ system
    // Our IDT handler table is in cpu::idt. We store the C handler pointer.
    crate::drivers::irq::register_kpi_handler(irq as u8, handler, dev);
    0 // success
}

#[no_mangle]
pub unsafe extern "C" fn kpi_free_irq(irq: u32, _dev: *mut u8) {
    crate::drivers::irq::unregister_kpi_handler(irq as u8);
}

#[no_mangle]
pub unsafe extern "C" fn kpi_enable_irq(irq: u32) {
    crate::drivers::irq::enable_irq(irq as u8);
}

#[no_mangle]
pub unsafe extern "C" fn kpi_disable_irq(irq: u32) {
    crate::drivers::irq::disable_irq(irq as u8);
}

#[no_mangle]
pub unsafe extern "C" fn kpi_synchronize_irq(_irq: u32) {
    // Bare metal single-core: IRQs are disabled during handlers, so this is a no-op.
}

// ── PCI wrappers ──────────────────────────────────────────────────────────────

#[repr(C)]
pub struct PciDev {
    vendor:             u16,
    device:             u16,
    subsystem_vendor:   u16,
    subsystem_device:   u16,
    revision:           u8,
    irq:                u8,
    bus:                u8,
    devfn:              u8,
    resource:           [PciResource; 7],
    driver_data:        *mut u8,
}

#[repr(C)]
pub struct PciResource {
    start: u64,
    end:   u64,
    flags: u64,
}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_read_config_dword(dev: *mut PciDev, offset: i32, val: *mut u32) -> i32 {
    let d = &*dev;
    let v = crate::drivers::pci::config_read32(d.bus, d.devfn >> 3, d.devfn & 7, offset as u8);
    *val = v;
    0
}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_read_config_word(dev: *mut PciDev, offset: i32, val: *mut u16) -> i32 {
    let d = &*dev;
    let v = crate::drivers::pci::config_read16(d.bus, d.devfn >> 3, d.devfn & 7, offset as u8);
    *val = v;
    0
}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_read_config_byte(dev: *mut PciDev, offset: i32, val: *mut u8) -> i32 {
    let d = &*dev;
    let v = crate::drivers::pci::config_read8(d.bus, d.devfn >> 3, d.devfn & 7, offset as u8);
    *val = v;
    0
}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_write_config_dword(dev: *mut PciDev, offset: i32, val: u32) -> i32 {
    let d = &*dev;
    crate::drivers::pci::config_write32(d.bus, d.devfn >> 3, d.devfn & 7, offset as u8, val);
    0
}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_write_config_word(dev: *mut PciDev, offset: i32, val: u16) -> i32 {
    let d = &*dev;
    crate::drivers::pci::config_write16(d.bus, d.devfn >> 3, d.devfn & 7, offset as u8, val);
    0
}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_write_config_byte(dev: *mut PciDev, offset: i32, val: u8) -> i32 {
    let d = &*dev;
    crate::drivers::pci::config_write8(d.bus, d.devfn >> 3, d.devfn & 7, offset as u8, val);
    0
}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_enable_device(dev: *mut PciDev) -> i32 {
    let d = &*dev;
    // Enable Memory Space + Bus Master in PCI Command register
    let mut cmd = crate::drivers::pci::config_read16(d.bus, d.devfn >> 3, d.devfn & 7, 0x04);
    cmd |= 0x0006; // PCI_COMMAND_MEMORY | PCI_COMMAND_MASTER
    crate::drivers::pci::config_write16(d.bus, d.devfn >> 3, d.devfn & 7, 0x04, cmd);
    0
}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_disable_device(_dev: *mut PciDev) {}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_request_regions(_dev: *mut PciDev, _name: *const u8) -> i32 { 0 }

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_release_regions(_dev: *mut PciDev) {}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_set_master(dev: *mut PciDev) {
    let d = &*dev;
    let mut cmd = crate::drivers::pci::config_read16(d.bus, d.devfn >> 3, d.devfn & 7, 0x04);
    cmd |= 0x0004; // PCI_COMMAND_MASTER
    crate::drivers::pci::config_write16(d.bus, d.devfn >> 3, d.devfn & 7, 0x04, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_clear_master(_dev: *mut PciDev) {}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_alloc_irq_vectors(
    _dev: *mut PciDev, _min: u32, _max: u32, _flags: u32
) -> i32 { 1 }

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_free_irq_vectors(_dev: *mut PciDev) {}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_irq_vector(dev: *mut PciDev, nr: u32) -> i32 {
    (*dev).irq as i32 + nr as i32
}

#[no_mangle]
pub unsafe extern "C" fn kpi_pci_iomap(dev: *mut PciDev, bar: i32, _maxlen: u64) -> *mut u8 {
    let d = &*dev;
    if bar < 0 || bar >= 6 { return core::ptr::null_mut(); }
    let start = d.resource[bar as usize].start;
    kpi_ioremap(start, 0) // size doesn't matter for direct-map
}

// ── Formatting helpers (no_std, no format! for primitives) ───────────────────

fn format_i32(v: i32) -> String {
    alloc::format!("{}", v)
}

fn format_u64(v: u64) -> String {
    alloc::format!("{}", v)
}

fn format_hex(v: u64, upper: bool) -> String {
    if upper {
        alloc::format!("{:X}", v)
    } else {
        alloc::format!("{:x}", v)
    }
}
