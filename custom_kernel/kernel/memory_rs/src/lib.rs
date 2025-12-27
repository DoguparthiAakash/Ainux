#![no_std]

extern crate alloc;

use linked_list_allocator::LockedHeap;
use core::alloc::Layout;
use core::ffi::c_void;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Import kernel PMM functions
extern "C" {
    fn pmm_alloc_pages(count: u64) -> *mut u8;
    fn kprint(msg: *const u8);
}

// Minimal panic handler
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        kprint("RUST PANIC!\n\0".as_ptr());
    }
    loop {}
}

const HEAP_SIZE: usize = 32 * 1024 * 1024; // 32 MB

#[no_mangle]
pub extern "C" fn heap_init() {
    unsafe {
        let pages_needed = (HEAP_SIZE as u64 + 4096 - 1) / 4096;
        let heap_start = pmm_alloc_pages(pages_needed);
        
        if heap_start.is_null() {
            kprint("Rust Heap: Failed to allocate physical memory\n\0".as_ptr());
            return;
        }

        // Map to higher half as per kernel convention
        // heap_start is physical address. existing heap.c adds 0xFFFF800000000000
        let heap_start_virt = heap_start as usize + 0xFFFF800000000000;
        let heap_start_ptr = heap_start_virt as *mut u8;

        ALLOCATOR.lock().init(heap_start_ptr, HEAP_SIZE);
        kprint("Rust Heap: Initialized\n\0".as_ptr());
    }
}

#[no_mangle]
pub extern "C" fn kmalloc(size: usize) -> *mut c_void {
    // Layout for the payload
    // Align to 8 bytes generic
    // We need extra space for usize (size)
    let align = 8;
    let size_field_size = core::mem::size_of::<usize>();
    
    // We allocation: [size (8 bytes)][...payload...]
    let total_size = size + size_field_size;
    let layout = Layout::from_size_align(total_size, align).unwrap();
    
    unsafe {
        let ptr = alloc::alloc::alloc(layout);
        if ptr.is_null() {
            return core::ptr::null_mut();
        }
        
        // Write size
        *(ptr as *mut usize) = size;
        
        // Return pointer to payload
        ptr.add(size_field_size) as *mut c_void
    }
}

#[no_mangle]
pub extern "C" fn kfree(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    
    unsafe {
        let size_field_size = core::mem::size_of::<usize>();
        let real_ptr = (ptr as *mut u8).sub(size_field_size);
        let size = *(real_ptr as *mut usize);
        
        let align = 8;
        let total_size = size + size_field_size;
        let layout = Layout::from_size_align(total_size, align).unwrap();
        
        alloc::alloc::dealloc(real_ptr, layout);
    }
}
