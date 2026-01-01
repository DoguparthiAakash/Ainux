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

extern "C" {
    fn serial_putc(c: u8);
}

// Minimal panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        let msg = "RUST PANIC AT: \0";
        for &b in msg.as_bytes() { if b!=0 { serial_putc(b); } }

        if let Some(location) = info.location() {
             let file = location.file();
             for &b in file.as_bytes() { serial_putc(b); }
             
             let line_msg = "\nLINE: \0";
             for &b in line_msg.as_bytes() { if b!=0 { serial_putc(b); } }
             
             let mut line = location.line();
             if line == 0 { serial_putc(b'0'); }
             else {
                 let mut rev_digits = [0u8; 20];
                 let mut count = 0;
                 while line > 0 {
                     rev_digits[count] = (line % 10) as u8 + b'0';
                     line /= 10;
                     count += 1;
                 }
                 while count > 0 {
                     count -= 1;
                     serial_putc(rev_digits[count]);
                 }
             }
             serial_putc(b'\n');
        } else {
             let unk = "UNKNOWN LOCATION\n\0";
             for &b in unk.as_bytes() { if b!=0 { serial_putc(b); } }
        }
    }
    loop {}
}

const HEAP_SIZE: usize = 16 * 1024 * 1024; 

#[no_mangle]
pub extern "C" fn heap_init(offset: u64) {
    unsafe {
        let pages_needed = (HEAP_SIZE as u64 + 4096 - 1) / 4096;
        let heap_start = pmm_alloc_pages(pages_needed);
        
        if heap_start.is_null() {
            kprint("Rust Heap: Failed to allocate physical memory\n\0".as_ptr());
            return;
        }

        let heap_start_virt = heap_start as usize + offset as usize;
        let heap_start_ptr = heap_start_virt as *mut u8;

        ALLOCATOR.lock().init(heap_start_ptr, HEAP_SIZE);
        kprint("Rust Heap: Initialized (Dynamic HHDM, 16MB)\n\0".as_ptr());
    }
}

#[no_mangle]
pub extern "C" fn kmalloc(size: usize) -> *mut c_void {
    // unsafe { kprint("[KA]\n\0".as_ptr()); } // Trace Alloc
    
    let align = 8;
    let size_field_size = core::mem::size_of::<usize>();
    let total_size = size + size_field_size;
    let layout = Layout::from_size_align(total_size, align).unwrap();
    
    unsafe {
        let ptr = alloc::alloc::alloc(layout);
        if ptr.is_null() {
            return core::ptr::null_mut();
        }
        *(ptr as *mut usize) = size;
        ptr.add(size_field_size) as *mut c_void
    }
}

#[no_mangle]
pub extern "C" fn kfree(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    
    // unsafe { kprint("[KF]\n\0".as_ptr()); } // Trace Free

    unsafe {
        let size_field_size = core::mem::size_of::<usize>();
        let real_ptr = (ptr as *mut u8).sub(size_field_size);
        let size = *(real_ptr as *mut usize);
        
        if size > 128*1024*1024 || size == 0 {
             kprint("Rust Heap: Corrupted Size in kfree!\n\0".as_ptr());
             return; 
        }
        
        let align = 8;
        let total_size = size + size_field_size;
        
        let layout_res = Layout::from_size_align(total_size, align);
        if layout_res.is_err() {
             kprint("Rust Heap: Header Corruption (Layout Error)!\n\0".as_ptr());
             return;
        }
        let layout = layout_res.unwrap();
        
        alloc::alloc::dealloc(real_ptr, layout);
    }
}
