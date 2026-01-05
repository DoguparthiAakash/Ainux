use alloc::vec::Vec;
use alloc::sync::Arc;
use crate::fs::vfs::FileHandle;

pub const MAX_FDS: usize = 256;

#[derive(Clone, Debug)]
pub struct FileDescriptor {
    pub handle: Arc<dyn FileHandle>,
    pub offset: u64,
}

#[derive(Clone, Debug)]
pub struct FileDescriptorTable {
    files: Vec<Option<FileDescriptor>>,
}

impl FileDescriptorTable {
    pub fn new() -> Self {
        let mut files = Vec::with_capacity(MAX_FDS);
        // Initialize with standard streams (0, 1, 2)
        // For now, they are None or Todo: SerialFileHandle?
        for _ in 0..MAX_FDS {
             files.push(None);
        }
        
        Self { files }
    }
    
    
    pub fn alloc_fd(&mut self, handle: Arc<dyn FileHandle>) -> Option<usize> {
        for (i, slot) in self.files.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(FileDescriptor { handle, offset: 0 });
                return Some(i);
            }
        }
        None
    }
    
    pub fn get_entry(&self, fd: usize) -> Option<FileDescriptor> {
        if fd >= self.files.len() {
             return None;
        }
        self.files[fd].clone()
    }

    pub fn update_offset(&mut self, fd: usize, new_offset: u64) {
         if fd < self.files.len() {
             if let Some(desc) = &mut self.files[fd] {
                 desc.offset = new_offset;
             }
         }
    }
    
    pub fn free_fd(&mut self, fd: usize) {
        if fd < self.files.len() {
            self.files[fd] = None;
        }
    }
}
