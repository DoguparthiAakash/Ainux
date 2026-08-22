use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

lazy_static::lazy_static! {
    pub static ref SHM_REGISTRY: Mutex<BTreeMap<String, SharedMemoryRegion>> = Mutex::new(BTreeMap::new());
}

#[derive(Clone)]
pub struct SharedMemoryRegion {
    pub name: String,
    pub size: usize,
    pub data: Vec<u8>, // Simulating physical contiguous pages for now
}

impl SharedMemoryRegion {
    pub fn new(name: &str, size: usize) -> Self {
        Self {
            name: String::from(name),
            size,
            data: alloc::vec![0; size],
        }
    }
}

/// Creates or opens a POSIX shared memory object.
pub fn shm_open(name: &str, size: usize) -> Option<usize> {
    let mut registry = SHM_REGISTRY.lock();
    if !registry.contains_key(name) {
        let region = SharedMemoryRegion::new(name, size);
        registry.insert(String::from(name), region);
    }
    // Return a mock file descriptor / handle
    Some(registry.keys().position(|k| k == name).unwrap() + 1000) // Offset by 1000 for SHM FDs
}

/// Maps a shared memory object into the virtual address space.
pub fn shm_mmap(name: &str) -> Option<*mut u8> {
    let mut registry = SHM_REGISTRY.lock();
    if let Some(region) = registry.get_mut(name) {
        Some(region.data.as_mut_ptr())
    } else {
        None
    }
}
