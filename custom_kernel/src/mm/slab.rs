use core::ptr::null_mut;
use spin::Mutex;
use alloc::vec::Vec;
use crate::mm::pmm::PAGE_SIZE;

/// A Slab represents a single page of memory divided into fixed-size slots.
pub struct Slab {
    pub page_addr: usize,
    pub free_list: *mut u8,
    pub used_slots: usize,
    pub total_slots: usize,
}

unsafe impl Send for Slab {}

impl Slab {
    pub fn new(page_addr: usize, slot_size: usize) -> Self {
        let total_slots = PAGE_SIZE / slot_size;
        let mut slab = Self {
            page_addr,
            free_list: page_addr as *mut u8,
            used_slots: 0,
            total_slots,
        };

        // Initialize free list link-style within the page
        unsafe {
            for i in 0..total_slots - 1 {
                let current = (page_addr + i * slot_size) as *mut *mut u8;
                let next = (page_addr + (i + 1) * slot_size) as *mut u8;
                *current = next;
            }
            // Last one is null
            let last = (page_addr + (total_slots - 1) * slot_size) as *mut *mut u8;
            *last = null_mut();
        }

        slab
    }

    pub fn alloc(&mut self) -> *mut u8 {
        if self.free_list.is_null() {
            return null_mut();
        }
        let ptr = self.free_list;
        unsafe {
            self.free_list = *(ptr as *mut *mut u8);
        }
        self.used_slots += 1;
        ptr
    }

    pub fn free(&mut self, ptr: *mut u8, slot_size: usize) {
        unsafe {
            *(ptr as *mut *mut u8) = self.free_list;
            self.free_list = ptr;
        }
        self.used_slots -= 1;
    }

    pub fn is_full(&self) -> bool {
        self.used_slots == self.total_slots
    }

    pub fn is_empty(&self) -> bool {
        self.used_slots == 0
    }
}

/// A Cache manages a collection of Slabs for a specific object size.
pub struct SlabCache {
    pub object_size: usize,
    pub partial_slabs: Vec<Slab>,
    pub full_slabs: Vec<Slab>,
    pub empty_slabs: Vec<Slab>,
}

impl SlabCache {
    pub fn new(size: usize) -> Self {
        Self {
            object_size: size,
            partial_slabs: Vec::new(),
            full_slabs: Vec::new(),
            empty_slabs: Vec::new(),
        }
    }

    fn grow(&mut self) -> Result<(), ()> {
        let frame = crate::mm::pmm::PMM.lock()
            .as_mut()
            .and_then(|pmm| pmm.alloc_frame())
            .ok_or(())?;
        
        // Map it into kernel space (HHDM)
        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let virt = frame as usize + hhdm as usize;
        
        let slab = Slab::new(virt, self.object_size);
        self.empty_slabs.push(slab);
        Ok(())
    }

    pub fn alloc(&mut self) -> *mut u8 {
        if self.partial_slabs.is_empty() {
            if self.empty_slabs.is_empty() {
                if self.grow().is_err() { return null_mut(); }
            }
            // Move from empty to partial
            if let Some(slab) = self.empty_slabs.pop() {
                self.partial_slabs.push(slab);
            }
        }

        // Try partial first
        if let Some(slab) = self.partial_slabs.last_mut() {
            let ptr = slab.alloc();
            if slab.is_full() {
                let full = self.partial_slabs.pop().unwrap();
                self.full_slabs.push(full);
            }
            return ptr;
        }

        null_mut()
    }
}

pub static SLAB_MANAGER: Mutex<Option<Vec<SlabCache>>> = Mutex::new(None);

pub fn init() {
    let mut manager = Vec::new();
    // Standard powers of 2 for generic kmalloc
    let sizes = [16, 32, 64, 128, 256, 512, 1024, 2048];
    for size in sizes {
        manager.push(SlabCache::new(size));
    }
    *SLAB_MANAGER.lock() = Some(manager);
}

pub fn kmalloc(size: usize) -> *mut u8 {
    let mut guard = SLAB_MANAGER.lock();
    if let Some(caches) = guard.as_mut() {
        for cache in caches {
            if cache.object_size >= size {
                return cache.alloc();
            }
        }
    }
    null_mut()
}
