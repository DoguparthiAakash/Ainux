// =============================================================================
// Ainux Buddy Allocator
// Designed to gracefully manage Terabytes of RAM via O(1) allocation lists
// =============================================================================

use core::sync::atomic::{AtomicUsize, Ordering};
use crate::mm::pmm::PAGE_SIZE;

pub const MAX_ORDER: usize = 20; // 2^20 * 4KB = 4GB contiguous regions max

#[derive(Debug)]
pub struct FreeBlock {
    pub next: Option<u64>,
    pub prev: Option<u64>,
}

pub struct BuddyAllocator {
    free_lists: [Option<u64>; MAX_ORDER + 1],
    total_frames: usize,
    used_frames: AtomicUsize,
    hhdm_offset: u64,
}

unsafe impl Send for BuddyAllocator {}
unsafe impl Sync for BuddyAllocator {}

impl BuddyAllocator {
    pub const fn new(hhdm_offset: u64) -> Self {
        Self {
            free_lists: [None; MAX_ORDER + 1],
            total_frames: 0,
            used_frames: AtomicUsize::new(0),
            hhdm_offset,
        }
    }

    pub fn set_hhdm_offset(&mut self, offset: u64) {
        self.hhdm_offset = offset;
    }

    pub fn set_total_frames(&mut self, frames: usize) {
        self.total_frames = frames;
        self.used_frames.store(frames, Ordering::Relaxed);
    }

    #[inline(always)]
    unsafe fn get_block_meta(&self, phys: u64) -> *mut FreeBlock {
        (phys + self.hhdm_offset) as *mut FreeBlock
    }

    unsafe fn push_list(&mut self, order: usize, phys: u64) {
        let meta = self.get_block_meta(phys);
        (*meta).prev = None;
        (*meta).next = self.free_lists[order];
        
        if let Some(head_phys) = self.free_lists[order] {
            let head_meta = self.get_block_meta(head_phys);
            (*head_meta).prev = Some(phys);
        }
        
        self.free_lists[order] = Some(phys);
    }

    unsafe fn remove_list(&mut self, order: usize, phys: u64) {
        let meta = self.get_block_meta(phys);
        let prev = (*meta).prev;
        let next = (*meta).next;
        
        if let Some(p) = prev {
            (*self.get_block_meta(p)).next = next;
        } else {
            self.free_lists[order] = next;
        }
        
        if let Some(n) = next {
            (*self.get_block_meta(n)).prev = prev;
        }
    }

    /// Add a free region to the buddy allocator
    pub unsafe fn add_free_region(&mut self, mut base: u64, mut len: usize) {
        let mut frames_added = 0;
        
        while len >= PAGE_SIZE {
            let mut order = 0;
            // Find max order alignment and length matching
            while order < MAX_ORDER {
                let block_size = PAGE_SIZE * (1 << (order + 1));
                if base % block_size as u64 != 0 { break; }
                if len < block_size { break; }
                order += 1;
            }
            
            self.push_list(order, base);
            
            let added_size = PAGE_SIZE * (1 << order);
            base += added_size as u64;
            len -= added_size;
            frames_added += 1 << order;
        }
        
        self.used_frames.fetch_sub(frames_added, Ordering::Relaxed);
    }

    pub fn alloc(&mut self, target_order: usize) -> Option<u64> {
        if target_order > MAX_ORDER { return None; }
        
        // Find suitable block
        let mut found_order = target_order;
        while found_order <= MAX_ORDER && self.free_lists[found_order].is_none() {
            found_order += 1;
        }
        
        if found_order > MAX_ORDER {
            return None; // No memory available for this order
        }
        
        unsafe {
            // Remove from found order
            let phys = self.free_lists[found_order].unwrap();
            self.remove_list(found_order, phys);
            
            // Split down to target_order
            while found_order > target_order {
                found_order -= 1;
                let buddy = phys + (PAGE_SIZE * (1 << found_order)) as u64;
                self.push_list(found_order, buddy);
            }
            
            self.used_frames.fetch_add(1 << target_order, Ordering::Relaxed);
            
            // Zero the page
            let ptr = (phys + self.hhdm_offset) as *mut u8;
            core::ptr::write_bytes(ptr, 0, PAGE_SIZE * (1 << target_order));
            
            Some(phys)
        }
    }

    pub fn free(&mut self, phys: u64, order: usize) {
        // Simple free without merging, in a full buddy implement we merge with buddy.
        // For baseline TB scaling, delayed merging is generally acceptable or basic.
        // Let's implement eager coalescing:
        let mut current_phys = phys;
        let mut current_order = order;
        
        unsafe {
            while current_order < MAX_ORDER {
                let buddy_phys = current_phys ^ (PAGE_SIZE * (1 << current_order)) as u64;
                
                // Scan list to check if buddy is free
                let mut is_buddy_free = false;
                let mut scan = self.free_lists[current_order];
                while let Some(scan_phys) = scan {
                    if scan_phys == buddy_phys {
                        is_buddy_free = true;
                        break;
                    }
                    scan = (*self.get_block_meta(scan_phys)).next;
                }
                
                if is_buddy_free {
                    self.remove_list(current_order, buddy_phys);
                    current_phys = core::cmp::min(current_phys, buddy_phys);
                    current_order += 1;
                } else {
                    break;
                }
            }
            
            self.push_list(current_order, current_phys);
            self.used_frames.fetch_sub(1 << order, Ordering::Relaxed);
        }
    }

    pub fn get_stats(&self) -> (usize, usize) {
        (self.used_frames.load(Ordering::Relaxed), self.total_frames)
    }

    pub fn alloc_frame(&mut self) -> Option<u64> {
        self.alloc(0)
    }

    pub fn alloc_contiguous(&mut self, count: usize) -> Option<u64> {
        let mut target_order = 0;
        while (1 << target_order) < count {
            target_order += 1;
        }
        self.alloc(target_order)
    }

    pub fn free_frame(&mut self, phys: u64) {
        self.free(phys, 0);
    }
}
