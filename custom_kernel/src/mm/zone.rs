// Zone Allocator - XNU-inspired fast memory allocation
// Provides fixed-size memory pools with per-CPU caching

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;

/// Zone for fixed-size allocations
pub struct Zone {
    element_size: usize,
    elements_per_page: usize,
    free_list: *mut FreeElement,
    allocated_pages: Vec<usize>,
    total_elements: usize,
    free_elements: usize,
}

unsafe impl Send for Zone {}
unsafe impl Sync for Zone {}

#[repr(C)]
struct FreeElement {
    next: *mut FreeElement,
}

/// Per-CPU cache for zone allocations
pub struct PerCpuCache {
    cached_elements: Vec<*mut u8>,
    max_cache_size: usize,
}

unsafe impl Send for PerCpuCache {}
unsafe impl Sync for PerCpuCache {}

/// Zone allocator managing multiple zones
pub struct ZoneAllocator {
    zones: BTreeMap<usize, Mutex<Zone>>,
    per_cpu_caches: Vec<Mutex<PerCpuCache>>,
}

impl Zone {
    pub fn new(element_size: usize) -> Self {
        let aligned_size = (element_size + 7) & !7;
        let elements_per_page = 4096 / aligned_size;
        
        Zone {
            element_size: aligned_size,
            elements_per_page,
            free_list: core::ptr::null_mut(),
            allocated_pages: Vec::new(),
            total_elements: 0,
            free_elements: 0,
        }
    }
    
    fn expand(&mut self) -> Result<(), ()> {
        let page_addr = crate::mm::pmm::PMM.lock()
            .as_mut()
            .and_then(|pmm| pmm.alloc_frame())
            .ok_or(())?;
        
        let hhdm_offset = crate::mm::vmm::VMM_HHDM_OFFSET
            .load(core::sync::atomic::Ordering::Relaxed);
        let virt_addr = page_addr as usize + hhdm_offset as usize;
        
        for i in 0..self.elements_per_page {
            let elem_addr = virt_addr + i * self.element_size;
            let elem = unsafe { &mut *(elem_addr as *mut FreeElement) };
            elem.next = self.free_list;
            self.free_list = elem as *mut FreeElement;
        }
        
        self.allocated_pages.push(page_addr as usize);
        self.total_elements += self.elements_per_page;
        self.free_elements += self.elements_per_page;
        Ok(())
    }
    
    pub fn alloc(&mut self) -> Result<*mut u8, ()> {
        if self.free_list.is_null() {
            self.expand()?;
        }
        
        if !self.free_list.is_null() {
            unsafe {
                let elem = self.free_list;
                self.free_list = (*elem).next;
                self.free_elements -= 1;
                Ok(elem as *mut u8)
            }
        } else {
            Err(())
        }
    }
    
    pub fn free(&mut self, ptr: *mut u8) {
        unsafe {
            let elem = &mut *(ptr as *mut FreeElement);
            elem.next = self.free_list;
            self.free_list = elem as *mut FreeElement;
            self.free_elements += 1;
        }
    }
    
    pub fn stats(&self) -> ZoneStats {
        ZoneStats {
            element_size: self.element_size,
            total_elements: self.total_elements,
            free_elements: self.free_elements,
            allocated_elements: self.total_elements - self.free_elements,
            pages: self.allocated_pages.len(),
        }
    }
}

impl PerCpuCache {
    pub fn new(max_size: usize) -> Self {
        PerCpuCache {
            cached_elements: Vec::with_capacity(max_size),
            max_cache_size: max_size,
        }
    }
    
    pub fn get(&mut self) -> Option<*mut u8> {
        self.cached_elements.pop()
    }
    
    pub fn put(&mut self, ptr: *mut u8) -> bool {
        if self.cached_elements.len() < self.max_cache_size {
            self.cached_elements.push(ptr);
            true
        } else {
            false
        }
    }
}

impl ZoneAllocator {
    pub fn new(num_cpus: usize) -> Self {
        let mut per_cpu_caches = Vec::new();
        for _ in 0..num_cpus {
            per_cpu_caches.push(Mutex::new(PerCpuCache::new(64)));
        }
        
        ZoneAllocator {
            zones: BTreeMap::new(),
            per_cpu_caches,
        }
    }
    
    fn get_zone(&mut self, size: usize) -> &Mutex<Zone> {
        let zone_size = size.next_power_of_two().max(8);
        self.zones.entry(zone_size)
            .or_insert_with(|| Mutex::new(Zone::new(zone_size)))
    }
    
    pub fn alloc(&mut self, size: usize) -> Result<*mut u8, ()> {
        if size == 0 { return Err(()); }
        self.get_zone(size).lock().alloc()
    }
    
    pub fn free(&mut self, ptr: *mut u8, size: usize) {
        let zone_size = size.next_power_of_two().max(8);
        if let Some(zone) = self.zones.get(&zone_size) {
            zone.lock().free(ptr);
        }
    }
    
    pub fn stats(&self) -> Vec<ZoneStats> {
        self.zones.values().map(|z| z.lock().stats()).collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ZoneStats {
    pub element_size: usize,
    pub total_elements: usize,
    pub free_elements: usize,
    pub allocated_elements: usize,
    pub pages: usize,
}

pub static ZONE_ALLOCATOR: Mutex<Option<ZoneAllocator>> = Mutex::new(None);

pub fn init(num_cpus: usize) {
    *ZONE_ALLOCATOR.lock() = Some(ZoneAllocator::new(num_cpus));
}

pub fn zalloc(size: usize) -> Result<*mut u8, ()> {
    // Avoid deadlock: Use try_lock. If locked (e.g. during init), fallback to global heap.
    if let Some(mut guard) = ZONE_ALLOCATOR.try_lock() {
        if let Some(allocator) = guard.as_mut() {
            return allocator.alloc(size);
        }
    }
    Err(())
}

pub fn zfree(ptr: *mut u8, size: usize) {
    if let Some(a) = ZONE_ALLOCATOR.lock().as_mut() {
        a.free(ptr, size);
    }
}

pub fn zone_stats() -> Vec<ZoneStats> {
    ZONE_ALLOCATOR.lock().as_ref().map(|a| a.stats()).unwrap_or_default()
}
