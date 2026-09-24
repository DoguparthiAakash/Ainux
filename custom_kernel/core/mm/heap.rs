use linked_list_allocator::LockedHeap;
use crate::mm::vmm;
use core::alloc::{GlobalAlloc, Layout};

pub struct HybridAllocator {
    heap: LockedHeap,
}

unsafe impl GlobalAlloc for HybridAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Fallback: Robust Linked List Heap handles everything
        let ptr = self.heap.alloc(layout);
        if ptr.is_null() {
            let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
            use core::fmt::Write;
            let _ = write!(serial, "\nHeap OOM: size={}, align={}\n", layout.size(), layout.align());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let addr = ptr as usize;
        
        // If the address is below our managed kernel heap, it must be a slab pointer
        if addr < HEAP_START {
            let size = layout.size();
            crate::mm::slab::free_custom(ptr, size);
            return;
        }

        self.heap.dealloc(ptr, layout)
    }
}

// Needed for inner access
impl HybridAllocator {
    pub const fn empty() -> Self {
        Self {
            heap: LockedHeap::empty(),
        }
    }
    
    pub fn init(&self, start: *mut u8, size: usize) {
        unsafe {
            self.heap.lock().init(start, size);
        }
    }
}

#[global_allocator]
static ALLOCATOR: HybridAllocator = HybridAllocator::empty();

pub const HEAP_START: usize = 0xFFFF_A000_0000_0000;

pub fn init() {
    // Default: 64MB heap. init_custom will auto-scale based on available physical RAM.
    init_custom(64 * 1024 * 1024);
}

pub fn init_custom(heap_size: usize) {
    let total_mem = crate::mm::pmm::TOTAL_MEMORY.load(core::sync::atomic::Ordering::Relaxed) as usize;

    // Auto-scale the heap based on available system RAM:
    //   <= 32MB  -> use 25% for heap
    //   <= 128MB -> use 30% for heap
    //   <= 512MB -> use 40% for heap
    //   > 512MB  -> use 50% for heap (generous; kernel allocator and user space both benefit)
    let ideal_heap = if total_mem > 0 && total_mem <= 32 * 1024 * 1024 {
        total_mem / 4
    } else if total_mem > 0 && total_mem <= 128 * 1024 * 1024 {
        total_mem * 30 / 100
    } else if total_mem > 0 && total_mem <= 512 * 1024 * 1024 {
        total_mem * 40 / 100
    } else if total_mem > 0 {
        total_mem / 2
    } else {
        heap_size // caller-provided fallback
    };

    // Never allocate more than what is physically free.
    let (allocated, usable) = crate::mm::pmm::PMM.lock().as_ref().unwrap().get_stats_fast();
    let free_frames = usable.saturating_sub(allocated);
    let max_possible_heap = free_frames * 4096;
    // Keep at least 512 frames (2MB) free for the OS after the heap is created.
    let headroom = 512 * 4096;
    let capped = if max_possible_heap > headroom { max_possible_heap - headroom } else { 4096 };

    let heap_size = ideal_heap.min(capped).max(4096); // always at least one page

    let pages = (heap_size + 4095) / 4096;
    let mut current_addr = HEAP_START;
    
    // Debug helper (manual serial output)
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "Heap: Reserving {} pages ({:?} bytes) at {:#x}\n", pages, heap_size, current_addr);

    for i in 0..pages {
        let frame = {
            let mut pmm_lock = crate::mm::pmm::PMM.lock();
            if let Some(ref mut pmm) = *pmm_lock {
                pmm.alloc_frame()
            } else {
                None
            }
        };
        
        if let Some(frame) = frame {
            if frame == 0 {
                let _ = write!(serial, "\nHeap: CRITICAL ERROR - PMM returned Frame 0 for index {}\n", i);
                loop { crate::hlt(); }
            }
            unsafe {
                // Only log every 2048th page (8MB) to reduce spam
                if i % 2048 == 0 {
                    let _ = write!(serial, "Heap: Mapping page {}/{} (Frame {:#x})\n", i, pages, frame);
                }
                match vmm::map_page(current_addr as u64, frame, 0x03) {
                    Ok(_) => {},
                    Err(e) => {
                        let _ = write!(serial, "\nHeap: FAILED to map page {} at {:#x} (Err: {:?})\n", i, current_addr, e);
                        loop { crate::hlt(); }
                    }
                }
            }
            current_addr += 4096;
        } else {
            let _ = write!(serial, "\nHeap: PHYSICAL OOM during init at page {}/{}\n", i, pages);
            loop { crate::hlt(); }
        }
    }
    
    let _ = write!(serial, "Heap: Mapping done. Initializing allocator...\n");
    
    // Initialize the allocator
    ALLOCATOR.init(HEAP_START as *mut u8, heap_size);
    
    let _ = write!(serial, "Heap: Allocator init done.\n");
}

// #[alloc_error_handler]
// fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
//     panic!("allocation error: {:?}", layout)
// }

pub fn verify_heap() {
    // Basic runtime self-test
    // Try to allocate a small box and free it.
    // If heap is corrupted, this might panic or hang (if deadlock).
    use alloc::boxed::Box;
    let b = Box::new(0xDEADBEEFu32);
    if *b != 0xDEADBEEF {
        panic!("Heap Corrupted: Value mismatch");
    }
    // Drop b -> free
}

#[no_mangle]
pub extern "C" fn kmalloc(size: usize, align: usize) -> *mut u8 {
    use core::alloc::{GlobalAlloc, Layout};
    let layout = Layout::from_size_align(size, align).unwrap();
    unsafe { ALLOCATOR.alloc(layout) }
}

#[no_mangle]
pub extern "C" fn kfree(ptr: *mut u8, size: usize, align: usize) {
    use core::alloc::{GlobalAlloc, Layout};
    let layout = Layout::from_size_align(size, align).unwrap();
    unsafe { ALLOCATOR.dealloc(ptr, layout) }
}
