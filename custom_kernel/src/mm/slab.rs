// src/mm/slab.rs
// Ainux Sovereign Slab Allocator
// Carves physical pages into small, efficient object slots.

use spin::Mutex;
use super::pmm::{PMM, PAGE_SIZE};

#[derive(Clone, Copy, Debug)]
struct SlabHeader {
    next_free: Option<*mut u8>,
    next_slab: Option<*mut u8>, // Link to next slab page
    free_count: usize,
}

pub struct SlabCache {
    object_size: usize,
    slabs: Mutex<Option<*mut u8>>, // Head of the slab page list
}

unsafe impl Send for SlabCache {}
unsafe impl Sync for SlabCache {}

pub fn init() {
}

impl SlabCache {
    pub const fn new(size: usize) -> Self {
        Self {
            object_size: size,
            slabs: Mutex::new(None),
        }
    }

    pub fn alloc(&self) -> Option<*mut u8> {
        let mut slabs_head = self.slabs.lock();
        
        // 1. Try to find a slab with free space
        let mut curr = *slabs_head;
        while let Some(slab_ptr) = curr {
            let header = unsafe { &mut *(slab_ptr as *mut SlabHeader) };
            if let Some(ptr) = header.next_free {
                // Pop from the free list
                let next = unsafe { *(ptr as *const Option<*mut u8>) };
                header.next_free = next;
                header.free_count -= 1;
                return Some(ptr);
            }
            curr = header.next_slab;
        }

        // 2. No free space, allocate a new page from PMM
        let mut pmm_lock = PMM.lock();
        if let Some(pmm) = pmm_lock.as_mut() {
            if let Some(phys) = pmm.alloc_frame() {
                let hhdm = pmm.hhdm_offset();
                let virt = (phys + hhdm) as *mut u8;
                
                // Initialize the new slab
                self.init_slab(virt);
                
                // Link it into the slab list
                let header = unsafe { &mut *(virt as *mut SlabHeader) };
                header.next_slab = *slabs_head;
                *slabs_head = Some(virt);
                
                // Re-allocate from the freshly initialized slab
                let ptr = header.next_free.unwrap();
                let next = unsafe { *(ptr as *const Option<*mut u8>) };
                header.next_free = next;
                header.free_count -= 1;
                return Some(ptr);
            }
        }
        None
    }

    fn init_slab(&self, ptr: *mut u8) {
        let header_size = core::mem::size_of::<SlabHeader>();
        let usable_space = PAGE_SIZE - header_size;
        let count = usable_space / self.object_size;
        
        unsafe {
            let mut current = ptr.add(header_size);
            let header = &mut *(ptr as *mut SlabHeader);
            header.next_free = Some(current);
            header.next_slab = None;
            header.free_count = count;
            
            // Create a linked list of free slots inside the page
            for i in 0..count {
                let next_ptr = if i == count - 1 {
                    None
                } else {
                    Some(current.add(self.object_size))
                };
                
                *(current as *mut Option<*mut u8>) = next_ptr;
                if let Some(next) = next_ptr {
                    current = next;
                }
            }
        }
    }

    pub fn free(&self, ptr: *mut u8) {
        // Simple return to free list (we assume the pointer is valid)
        let page_base = (ptr as usize & !(PAGE_SIZE - 1)) as *mut u8;
        let header = unsafe { &mut *(page_base as *mut SlabHeader) };
        
        unsafe {
            *(ptr as *mut Option<*mut u8>) = header.next_free;
            header.next_free = Some(ptr);
            header.free_count += 1;
        }
    }
}

// Pre-defined caches for common kernel sizes
pub static SLAB_64: SlabCache = SlabCache::new(64);
pub static SLAB_128: SlabCache = SlabCache::new(128);
pub static SLAB_512: SlabCache = SlabCache::new(512);

pub fn alloc_custom(size: usize) -> Option<*mut u8> {
    if size <= 64 { SLAB_64.alloc() }
    else if size <= 128 { SLAB_128.alloc() }
    else if size <= 512 { SLAB_512.alloc() }
    else { None }
}

pub fn free_custom(ptr: *mut u8, size: usize) {
    if size <= 64 { SLAB_64.free(ptr) }
    else if size <= 128 { SLAB_128.free(ptr) }
    else if size <= 512 { SLAB_512.free(ptr) }
}
