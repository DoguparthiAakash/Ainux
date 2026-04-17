// =============================================================================
// Ainux Physical Memory Manager — Multiboot Edition
// Replaces Limine-based PMM with direct Multiboot1 memory map parsing.
// Uses identity-mapped first 1GB from boot.asm for physical memory access.
// =============================================================================

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

// ---- Multiboot1 Structures ----

const MULTIBOOT_MAGIC: u32 = 0x2BADB002;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct MultibootInfo {
    pub flags: u32,
    pub mem_lower: u32,           // KB of lower memory (flag bit 0)
    pub mem_upper: u32,           // KB of upper memory (flag bit 0)
    pub boot_device: u32,
    pub cmdline: u32,
    pub mods_count: u32,
    pub mods_addr: u32,
    pub syms: [u32; 4],
    pub mmap_length: u32,         // Size of memory map (flag bit 6)
    pub mmap_addr: u32,           // Physical address of memory map (flag bit 6)
    pub drives_length: u32,
    pub drives_addr: u32,
    pub config_table: u32,
    pub boot_loader_name: u32,
    pub apm_table: u32,
    // VBE fields follow...
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct MultibootMmapEntry {
    pub size: u32,         // Size of the entry (excluding this field)
    pub addr: u64,         // Base address
    pub len: u64,          // Length in bytes
    pub entry_type: u32,   // 1 = Available, 2 = Reserved, 3 = ACPI Reclaimable, 4 = NVS, 5 = Bad
}

pub const MMAP_TYPE_AVAILABLE: u32 = 1;
pub const MMAP_TYPE_RESERVED: u32 = 2;
pub const MMAP_TYPE_ACPI_RECLAIMABLE: u32 = 3;

// ---- Globals from boot.asm ----

extern "C" {
    static MULTIBOOT_INFO_PTR: u64;
    static MULTIBOOT_MAGIC_VAL: u64;
}

// ---- Constants ----

pub const PAGE_SIZE: usize = 4096;

/// HHDM offset. With our boot.asm identity mapping, physical addresses < 1GB
/// can be accessed directly (offset = 0). We keep this atomic for future
/// extension when we remap with a proper HHDM.
pub static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);

/// Total detected system memory in bytes
pub static TOTAL_MEMORY: AtomicU64 = AtomicU64::new(0);

/// PMM singleton
pub static PMM: Mutex<Option<BitmapPmm>> = Mutex::new(None);

// ---- Bitmap Physical Memory Manager ----

pub struct BitmapPmm {
    bitmap: &'static mut [u64],
    total_frames: usize,
    used_frames: AtomicUsize,
    last_idx: usize,
    hhdm_offset: u64,
}

unsafe impl Send for BitmapPmm {}
unsafe impl Sync for BitmapPmm {}

impl BitmapPmm {
    /// Read the Multiboot info structure and initialize the physical memory manager.
    /// Called once from _start on the BSP.
    pub fn init() {
        let info_phys = unsafe { MULTIBOOT_INFO_PTR };
        let magic = unsafe { MULTIBOOT_MAGIC_VAL } as u32;

        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        use core::fmt::Write;

        let _ = write!(serial, "PMM: Multiboot magic={:#x} info_ptr={:#x}\n", magic, info_phys);

        if magic != MULTIBOOT_MAGIC {
            let _ = write!(serial, "PMM: WARNING — Multiboot magic mismatch! Expected {:#x}\n", MULTIBOOT_MAGIC);
            // Fall back to basic memory detection
            Self::init_fallback();
            return;
        }

        if info_phys == 0 {
            let _ = write!(serial, "PMM: Multiboot info pointer is NULL!\n");
            Self::init_fallback();
            return;
        }

        // Safety: info_phys is in identity-mapped region (< 1GB)
        let info = unsafe { &*(info_phys as *const MultibootInfo) };
        let flags = info.flags;
        let _ = write!(serial, "PMM: Multiboot flags={:#x}\n", flags);

        // Flag bit 6: mmap_* fields are valid
        if flags & (1 << 6) != 0 {
            Self::init_from_mmap(info, &mut serial);
        } else if flags & 1 != 0 {
            // Flag bit 0: mem_lower/mem_upper are valid
            let mem_lower = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*info).mem_lower)) };
            let mem_upper = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*info).mem_upper)) };
            let total_kb = mem_lower as u64 + mem_upper as u64;
            let _ = write!(serial, "PMM: Basic mem info: lower={}KB upper={}KB total={}KB\n",
                mem_lower, mem_upper, total_kb);
            Self::init_basic(total_kb * 1024);
        } else {
            let _ = write!(serial, "PMM: No memory info from GRUB!\n");
            Self::init_fallback();
        }
    }

    fn init_from_mmap(info: &MultibootInfo, serial: &mut crate::drivers::serial::SerialPort) {
        use core::fmt::Write;

        let mmap_addr = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*info).mmap_addr)) } as usize;
        let mmap_len = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*info).mmap_length)) } as usize;

        let _ = write!(serial, "PMM: Parsing memory map at {:#x} len={}\n", mmap_addr, mmap_len);

        // Phase 1: Walk the map to find the maximum physical address and total available memory
        let mut max_addr: u64 = 0;
        let mut total_available: u64 = 0;
        let mut offset = 0;

        while offset < mmap_len {
            let entry_ptr = (mmap_addr + offset) as *const MultibootMmapEntry;
            let e_addr = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).addr)) };
            let e_len = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).len)) };
            let e_type = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).entry_type)) };
            let e_size = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).size)) };
            let entry_end = e_addr + e_len;

            let type_str = match e_type {
                1 => "Available",
                2 => "Reserved",
                3 => "ACPI Reclaim",
                4 => "ACPI NVS",
                5 => "Bad RAM",
                _ => "Unknown",
            };
            let _ = write!(serial, "  [{:#012x} - {:#012x}] {} ({})\n",
                e_addr, entry_end, type_str, e_len);

            if e_type == MMAP_TYPE_AVAILABLE {
                total_available += e_len;
            }
            if entry_end > max_addr {
                max_addr = entry_end;
            }

            // Next entry: size field + 4 bytes for the size field itself
            offset += e_size as usize + 4;
        }

        TOTAL_MEMORY.store(total_available, Ordering::Relaxed);
        let _ = write!(serial, "PMM: Max physical address: {:#x}\n", max_addr);
        let _ = write!(serial, "PMM: Total available memory: {} MB\n", total_available / (1024 * 1024));

        // Cap at 4GB (our identity map covers 4GB via boot.asm PD0-PD3)
        if max_addr > 0x100000000 {
            max_addr = 0x100000000;
        }

        let total_frames = (max_addr / PAGE_SIZE as u64) as usize;
        let bitmap_size_u64 = (total_frames + 63) / 64;
        let bitmap_size_bytes = bitmap_size_u64 * 8;

        let _ = write!(serial, "PMM: Total frames: {} bitmap_size: {} bytes\n", total_frames, bitmap_size_bytes);

        // Phase 2: Find a usable region for the bitmap
        let mut bitmap_phys: u64 = 0;
        offset = 0;
        while offset < mmap_len {
            let entry_ptr = unsafe { (mmap_addr + offset) as *const MultibootMmapEntry };
            let e_addr = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).addr)) };
            let e_len = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).len)) };
            let e_type = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).entry_type)) };
            let e_size = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).size)) };
            if e_type == MMAP_TYPE_AVAILABLE && e_len >= bitmap_size_bytes as u64 {
                // Place bitmap at aligned address, avoiding first 16MB
                let candidate = if e_addr < 0x1000000 {
                    0x1000000_u64
                } else {
                    e_addr
                };
                let aligned = (candidate + 0xFFF) & !0xFFF; // Page align
                if aligned + bitmap_size_bytes as u64 <= e_addr + e_len {
                    bitmap_phys = aligned;
                    break;
                }
            }
            offset += e_size as usize + 4;
        }

        if bitmap_phys == 0 {
            let _ = write!(serial, "PMM: FATAL — no space for bitmap!\n");
            Self::init_fallback();
            return;
        }

        let _ = write!(serial, "PMM: Bitmap at physical {:#x}\n", bitmap_phys);

        // With identity mapping, virtual == physical for < 1GB
        let hhdm_offset: u64 = 0;
        HHDM_OFFSET.store(hhdm_offset, Ordering::Relaxed);

        let bitmap_virt = bitmap_phys + hhdm_offset;
        let bitmap_ptr = bitmap_virt as *mut u64;
        let bitmap = unsafe { core::slice::from_raw_parts_mut(bitmap_ptr, bitmap_size_u64) };

        // Mark everything as used
        for qword in bitmap.iter_mut() {
            *qword = !0;
        }

        let mut pmm = BitmapPmm {
            bitmap,
            total_frames,
            used_frames: AtomicUsize::new(total_frames),
            last_idx: 0,
            hhdm_offset,
        };

        // Phase 3: Free usable regions
        offset = 0;
        while offset < mmap_len {
            let entry_ptr = unsafe { (mmap_addr + offset) as *const MultibootMmapEntry };
            let e_addr = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).addr)) };
            let e_len = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).len)) };
            let e_type = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).entry_type)) };
            let e_size = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_ptr).size)) };
            if e_type == MMAP_TYPE_AVAILABLE {
                let base = e_addr;
                let len = e_len as usize;
                // Only free within our managed range
                if base < max_addr {
                    let actual_len = core::cmp::min(len as u64, max_addr - base) as usize;
                    pmm.free_region(base, actual_len);
                }
            }
            offset += e_size as usize + 4;
        }

        // Phase 4: Mark critical regions as used
        // First 2MB (BIOS, kernel, Multiboot structures, page tables)
        pmm.mark_region_used(0, 0x200000);
        // Bitmap region
        pmm.mark_region_used(bitmap_phys, bitmap_size_bytes);
        // Kernel region (1MB - 4MB approx, conservative)
        pmm.mark_region_used(0x100000, 0x300000);

        let (used, total) = pmm.get_stats_fast();
        let _ = write!(serial, "PMM: Initialized. Used: {}/{} frames ({} MB free)\n",
            used, total, (total - used) * PAGE_SIZE / (1024 * 1024));

        *PMM.lock() = Some(pmm);
    }

    fn init_basic(total_bytes: u64) {
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        use core::fmt::Write;
        let _ = write!(serial, "PMM: Basic init with {} bytes\n", total_bytes);

        let max_addr = core::cmp::min(total_bytes, 0x40000000); // Cap at 1GB
        let total_frames = (max_addr / PAGE_SIZE as u64) as usize;
        let bitmap_size_u64 = (total_frames + 63) / 64;
        let bitmap_size_bytes = bitmap_size_u64 * 8;

        // Place bitmap at 16MB
        let bitmap_phys: u64 = 0x1000000;
        HHDM_OFFSET.store(0, Ordering::Relaxed);
        TOTAL_MEMORY.store(total_bytes, Ordering::Relaxed);

        let bitmap_ptr = bitmap_phys as *mut u64;
        let bitmap = unsafe { core::slice::from_raw_parts_mut(bitmap_ptr, bitmap_size_u64) };

        for qword in bitmap.iter_mut() {
            *qword = !0;
        }

        let mut pmm = BitmapPmm {
            bitmap,
            total_frames,
            used_frames: AtomicUsize::new(total_frames),
            last_idx: 0,
            hhdm_offset: 0,
        };

        // Free everything above 2MB up to max
        if max_addr > 0x200000 {
            pmm.free_region(0x200000, (max_addr - 0x200000) as usize);
        }
        // Re-mark first 2MB + bitmap
        pmm.mark_region_used(0, 0x200000);
        pmm.mark_region_used(bitmap_phys, bitmap_size_bytes);

        let _ = write!(serial, "PMM: Basic init complete. {} frames total\n", total_frames);
        *PMM.lock() = Some(pmm);
    }

    fn init_fallback() {
        // Absolute minimum: assume 128MB of RAM, place bitmap at 16MB
        Self::init_basic(128 * 1024 * 1024);
    }

    // ---- Bitmap Manipulation ----

    fn mark_region_used(&mut self, base: u64, len: usize) {
        let start_frame = (base / PAGE_SIZE as u64) as usize;
        let end_frame = ((base + len as u64 + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as usize;
        for i in start_frame..end_frame {
            if i < self.total_frames {
                self.set_bit(i);
            }
        }
    }

    fn free_region(&mut self, base: u64, len: usize) {
        let start_frame = (base / PAGE_SIZE as u64) as usize;
        let num_frames = len / PAGE_SIZE;
        for i in 0..num_frames {
            let frame = start_frame + i;
            if frame < self.total_frames {
                self.clear_bit(frame);
            }
        }
    }

    #[inline(always)]
    fn set_bit(&mut self, frame: usize) {
        let idx = frame / 64;
        let bit = frame % 64;
        if idx < self.bitmap.len() {
            self.bitmap[idx] |= 1 << bit;
        }
    }

    #[inline(always)]
    fn clear_bit(&mut self, frame: usize) {
        let idx = frame / 64;
        let bit = frame % 64;
        if idx < self.bitmap.len() {
            self.bitmap[idx] &= !(1 << bit);
        }
    }

    #[inline(always)]
    fn test_bit(&self, frame: usize) -> bool {
        let idx = frame / 64;
        let bit = frame % 64;
        if idx < self.bitmap.len() {
            (self.bitmap[idx] & (1 << bit)) != 0
        } else {
            true // Out of range = used
        }
    }

    // ---- Allocation ----

    pub fn alloc_frame(&mut self) -> Option<u64> {
        // Next-fit search with wraparound
        let len = self.bitmap.len();
        for scan in 0..len {
            let idx = (self.last_idx + scan) % len;
            if self.bitmap[idx] != !0 {
                for bit in 0..64 {
                    if (self.bitmap[idx] & (1 << bit)) == 0 {
                        let frame = idx * 64 + bit;
                        if frame < self.total_frames {
                            self.set_bit(frame);
                            self.last_idx = idx;
                            self.used_frames.fetch_add(1, Ordering::Relaxed);
                            return Some(frame as u64 * PAGE_SIZE as u64);
                        }
                    }
                }
            }
        }
        None
    }

    /// Allocate N contiguous physical frames. Returns the base physical address.
    pub fn alloc_contiguous(&mut self, count: usize) -> Option<u64> {
        if count == 0 { return None; }
        if count == 1 { return self.alloc_frame(); }

        // Brute force scan for `count` contiguous free frames
        let mut run_start = 0;
        let mut run_len = 0;

        for frame in 0..self.total_frames {
            if !self.test_bit(frame) {
                if run_len == 0 { run_start = frame; }
                run_len += 1;
                if run_len >= count {
                    // Found! Mark all as used
                    for i in run_start..(run_start + count) {
                        self.set_bit(i);
                    }
                    self.used_frames.fetch_add(count, Ordering::Relaxed);
                    return Some(run_start as u64 * PAGE_SIZE as u64);
                }
            } else {
                run_len = 0;
            }
        }
        None
    }

    pub fn free_frame(&mut self, phys_addr: u64) {
        let frame = (phys_addr / PAGE_SIZE as u64) as usize;
        if frame < self.total_frames && self.test_bit(frame) {
            // Zero the page for security (prevent info leaks)
            let virt = phys_addr + self.hhdm_offset;
            unsafe {
                core::ptr::write_bytes(virt as *mut u8, 0, PAGE_SIZE);
            }
            self.clear_bit(frame);
            self.used_frames.fetch_sub(1, Ordering::Relaxed);
            if frame / 64 < self.last_idx {
                self.last_idx = frame / 64;
            }
        }
    }

    pub fn hhdm_offset(&self) -> u64 {
        self.hhdm_offset
    }

    pub fn get_stats(&self) -> (usize, usize) {
        let mut used = 0;
        for i in 0..self.total_frames {
            if self.test_bit(i) {
                used += 1;
            }
        }
        (used, self.total_frames)
    }

    /// Fast stats using atomic counter instead of scanning bitmap
    pub fn get_stats_fast(&self) -> (usize, usize) {
        (self.used_frames.load(Ordering::Relaxed), self.total_frames)
    }

    pub fn total_frames(&self) -> usize {
        self.total_frames
    }
}
