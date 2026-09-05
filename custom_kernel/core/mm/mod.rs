pub mod pmm;
pub mod vmm;
pub mod heap;
pub mod buddy;
pub mod zone;
pub mod slab;
pub mod user;
pub mod shm;
pub mod memory_region;
pub mod address_space;

extern "C" {
    pub fn fast_memcpy(dest: *mut u8, src: *const u8, count: usize);
    pub fn fast_memset(dest: *mut u8, val: u8, count: usize);
}
