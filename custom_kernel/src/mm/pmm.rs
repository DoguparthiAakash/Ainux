use limine::request::{MemoryMapRequest, HhdmRequest};
use limine::memory_map::EntryType;
use core::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use spin::Mutex;

static MEMMAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

pub static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);

pub const PAGE_SIZE: usize = 4096;

pub struct BitmapPmm {
    bitmap: &'static mut [u64],
    total_frames: usize,
    used_frames: usize,
    last_idx: usize,
    hhdm_offset: u64,
}

pub static PMM: Mutex<Option<BitmapPmm>> = Mutex::new(None);

impl BitmapPmm {
    /// Initialize the physical memory manager
    pub fn init() {
        let memmap = MEMMAP_REQUEST.get_response()
            .expect("Failed to get memory map")
            .entries();
        
        let hhdm_offset = HHDM_REQUEST.get_response()
            .expect("Failed to get HHDM")
            .offset();
        
        HHDM_OFFSET.store(hhdm_offset, Ordering::Relaxed);

        // 1. Calculate max memory
        let mut max_addr = 0;
        for entry in memmap {
            let end = entry.base + entry.length;
            if end > max_addr {
                max_addr = end;
            }
        }

        let total_frames = (max_addr / PAGE_SIZE as u64) as usize;
        let bitmap_size_bytes = (total_frames + 63) / 64 * 8;

        // 2. Find a place for the bitmap
        let mut bitmap_phys = 0;
        for entry in memmap {
            if entry.entry_type == EntryType::USABLE && entry.length >= bitmap_size_bytes as u64 {
                bitmap_phys = entry.base;
                break;
            }
        }
        
        if bitmap_phys == 0 {
            panic!("PMM: No memory for bitmap!");
        }

        // 3. Map the bitmap to virtual address space using HHDM
        let bitmap_virt = bitmap_phys + hhdm_offset;
        let bitmap_ptr = bitmap_virt as *mut u64;
        let bitmap_len = (bitmap_size_bytes as usize) / 8;
        
        // Safety: We found a USABLE region and are using the bootloader-provided HHDM.
        let bitmap = unsafe { core::slice::from_raw_parts_mut(bitmap_ptr, bitmap_len) };

        // 4. Initialize bitmap: Mark EVERYTHING as used first, then free the USABLE regions
        // This is safer than the reverse.
        for qword in bitmap.iter_mut() {
            *qword = !0; // All 1s = All used
        }

        let mut pmm = BitmapPmm {
            bitmap,
            total_frames,
            used_frames: total_frames, 
            last_idx: 0,
            hhdm_offset,
        };

        // 5. Unmark usable regions
        for entry in memmap {
            if entry.entry_type == EntryType::USABLE {
                pmm.free_region(entry.base, entry.length as usize);
            }
        }

        // 6. Mark the bitmap ITSELF as used!
        pmm.mark_region_used(bitmap_phys, bitmap_size_bytes as usize);
        
        // 7. Mark first 16MB as used (BIOS, Kernel, DMA etc safety)
        pmm.mark_region_used(0, 0x1000000);

        *PMM.lock() = Some(pmm);
        
        // Print status (crude logging)
        // Note: we can't easily print here without the serial port from main.
    }

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

    fn set_bit(&mut self, frame: usize) {
        let idx = frame / 64;
        let bit = frame % 64;
        self.bitmap[idx] |= 1 << bit;
        // Optimization: used_frames tracking is tricky with regions, ignoring for now
    }

    fn clear_bit(&mut self, frame: usize) {
        let idx = frame / 64;
        let bit = frame % 64;
        self.bitmap[idx] &= !(1 << bit);
    }
    
    fn test_bit(&self, frame: usize) -> bool {
         let idx = frame / 64;
         let bit = frame % 64;
         (self.bitmap[idx] & (1 << bit)) != 0
    }

    pub fn alloc_frame(&mut self) -> Option<u64> {
        // Simple next-fit search
        for idx in self.last_idx..self.bitmap.len() {
            if self.bitmap[idx] != !0 { // if not full
                // Found a u64 with at least one free bit
                for bit in 0..64 {
                    if (self.bitmap[idx] & (1 << bit)) == 0 {
                        let frame = idx * 64 + bit;
                        self.set_bit(frame);
                        self.last_idx = idx;
                        return Some(frame as u64 * PAGE_SIZE as u64);
                    }
                }
            }
        }
        
        // Wrap around search from 0 if needed (omitted for brevity in MVK, but good practice)
        None
    }
    
    pub fn free_frame(&mut self, phys_addr: u64) {
        let frame = (phys_addr / PAGE_SIZE as u64) as usize;
        if frame < self.total_frames {
            self.clear_bit(frame);
            if frame / 64 < self.last_idx {
                self.last_idx = frame / 64;
            }
        }
    }
    
    /// Returns the HHDM offset (useful for converting Phys -> Virt)
    pub fn hhdm_offset(&self) -> u64 {
        self.hhdm_offset
    }
}
