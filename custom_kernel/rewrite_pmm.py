import os

path = "e:/Ainux/Ainux/custom_kernel/src/mm/pmm.rs"
with open(path, "r") as f:
    content = f.read()

# Replace the entire first section up to init_from_mmap
new_content = """// =============================================================================
// Ainux Physical Memory Manager — Limine Edition
// Uses Limine memory map parsing and Higher Half Direct Map (HHDM).
// =============================================================================

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;
use limine::memory_map::EntryType;

// ---- Constants ----

pub const PAGE_SIZE: usize = 4096;

/// HHDM offset.
pub static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);

/// Total detected system memory in bytes
pub static TOTAL_MEMORY: AtomicU64 = AtomicU64::new(0);

/// PMM singleton
pub static PMM: Mutex<Option<BitmapPmm>> = Mutex::new(None);

// ---- Bitmap Physical Memory Manager ----

pub struct BitmapPmm {
    bitmap: &'static mut [u64],
    total_frames: usize,    // Total Physical Limit (Address Space)
    usable_frames: usize,   // Total Physical RAM (Sum of available regions)
    allocated_frames: AtomicUsize, // Frames handed out via alloc_frame
    last_idx: usize,
    hhdm_offset: u64,
}

unsafe impl Send for BitmapPmm {}
unsafe impl Sync for BitmapPmm {}

impl BitmapPmm {
    pub fn init() {
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        use core::fmt::Write;
        
        let hhdm_offset = if let Some(hhdm) = crate::HHDM_REQUEST.get_response().get() {
            hhdm.offset()
        } else {
            0
        };
        HHDM_OFFSET.store(hhdm_offset, Ordering::Relaxed);
        let _ = write!(serial, "PMM: HHDM Offset = {:#x}\\n", hhdm_offset);

        if let Some(fb_resp) = crate::FRAMEBUFFER_REQUEST.get_response().get() {
            if let Some(fb) = fb_resp.framebuffers().next() {
                *crate::drivers::video::FRAMEBUFFER_ADDR.lock() = fb.addr() as u64;
                *crate::drivers::video::FRAMEBUFFER_WIDTH.lock() = fb.width() as usize;
                *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock() = fb.height() as usize;
                *crate::drivers::video::FRAMEBUFFER_PITCH.lock() = fb.pitch() as usize;
                *crate::drivers::video::FRAMEBUFFER_BPP.lock() = fb.bpp() as u8;
                *crate::drivers::video::FRAMEBUFFER_TYPE.lock() = 0; 
                let _ = write!(serial, "PMM: Framebuffer injected.\\n");
            }
        }

        if let Some(mmap_resp) = crate::MEMMAP_REQUEST.get_response().get() {
            Self::init_from_limine(mmap_resp, hhdm_offset, &mut serial);
        } else {
            let _ = write!(serial, "PMM: No memory map from Limine!\\n");
            Self::init_fallback();
        }
    }

    fn init_from_limine(mmap: &limine::response::MemmapResponse, hhdm_offset: u64, serial: &mut crate::drivers::serial::SerialPort) {
        use core::fmt::Write;

        let mut max_addr: u64 = 0;
        let mut total_available: u64 = 0;
        let entries = mmap.entries();
        
        for entry in entries.iter() {
            let base = entry.base;
            let len = entry.length;
            let end = base + len;
            
            if entry.entry_type == EntryType::USABLE {
                total_available += len;
            }
            if end > max_addr {
                max_addr = end;
            }
        }

        TOTAL_MEMORY.store(total_available, Ordering::Relaxed);
        let _ = write!(serial, "PMM: Max physical address: {:#x}\\n", max_addr);
        let _ = write!(serial, "PMM: Total available memory: {} MB\\n", total_available / (1024 * 1024));

        let total_frames = (max_addr / PAGE_SIZE as u64) as usize;
        let bitmap_size_u64 = (total_frames + 63) / 64;
        let bitmap_size_bytes = bitmap_size_u64 * 8;

        let mut bitmap_phys: u64 = 0;
        
        for entry in entries.iter() {
            if entry.entry_type == EntryType::USABLE && entry.length >= bitmap_size_bytes as u64 {
                let candidate = if entry.base < 0x1000000 { 0x1000000 } else { entry.base };
                let aligned = (candidate + 0xFFF) & !0xFFF;
                if aligned + bitmap_size_bytes as u64 <= entry.base + entry.length {
                    bitmap_phys = aligned;
                    break;
                }
            }
        }

        if bitmap_phys == 0 {
            Self::init_fallback();
            return;
        }

        let bitmap_virt = bitmap_phys + hhdm_offset;
        let bitmap_ptr = bitmap_virt as *mut u64;
        let bitmap = unsafe { core::slice::from_raw_parts_mut(bitmap_ptr, bitmap_size_u64) };

        for qword in bitmap.iter_mut() {
            *qword = !0; // Mark all used initially
        }

        let mut pmm = BitmapPmm {
            bitmap,
            total_frames,
            usable_frames: (total_available / 4096) as usize,
            allocated_frames: AtomicUsize::new(0),
            last_idx: 0,
            hhdm_offset,
        };

        for entry in entries.iter() {
            if entry.entry_type == EntryType::USABLE {
                pmm.free_region(entry.base, entry.length as usize);
                crate::mm::buddy::ZONED_PMM.lock().add_free_region(entry.base, entry.length as usize);
            }
        }

        pmm.mark_region_used(0, 0x100000); // 1MB legacy
        pmm.mark_region_used(bitmap_phys, bitmap_size_bytes);

        let (used, _) = pmm.get_stats();
        pmm.allocated_frames.store(used, Ordering::Relaxed);

        let _ = write!(serial, "PMM: Initialized. Used frames: {}\\n", used);
        *PMM.lock() = Some(pmm);
    }
"""

start_idx = content.find("    fn init_basic")
end_part = content[start_idx:]

with open(path, "w") as f:
    f.write(new_content + end_part)
