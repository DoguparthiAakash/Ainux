use linked_list_allocator::LockedHeap;
use crate::mm::vmm;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 32 * 1024 * 1024; // 32 MiB

pub fn init() {
    // Map the heap pages
    // map HEAP_SIZE bytes starting at HEAP_START
    // allocate frames from PMM
    
    let pages = (HEAP_SIZE + 4095) / 4096;
    let mut current_addr = HEAP_START;
    
    // Debug helper (manual serial output)
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
    let _ = write!(serial, "Heap: Need to map {} pages at {:#x}. HHDM: {:#x}\n", pages, current_addr, hhdm);

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
            unsafe {
                // Only log every 1024th page (4MB) to avoid spam
                if i % 1024 == 0 {
                    let _ = write!(serial, "Heap: Mapping page {}/{} (Frame {:#x})\n", i, pages, frame);
                }
                match vmm::map_page(current_addr as u64, frame, 0x03) {
                    Ok(_) => {},
                    Err(e) => {
                        let _ = write!(serial, "Heap: Failed to map page {}: {}\n", i, e);
                    }
                }
            }
            current_addr += 4096;
        } else {
            panic!("Heap OOM during init");
        }
    }
    
    let _ = write!(serial, "Heap: Mapping done. Initializing allocator...\n");
    
    // Initialize the allocator
    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }
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

pub fn kmalloc(size: usize, align: usize) -> *mut u8 {
    use core::alloc::{GlobalAlloc, Layout};
    let layout = Layout::from_size_align(size, align).unwrap();
    unsafe { ALLOCATOR.alloc(layout) }
}

pub fn kfree(ptr: *mut u8, size: usize, align: usize) {
    use core::alloc::{GlobalAlloc, Layout};
    let layout = Layout::from_size_align(size, align).unwrap();
    unsafe { ALLOCATOR.dealloc(ptr, layout) }
}
