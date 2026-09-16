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
        for _ in 0..MAX_FDS {
             files.push(None);
        }
        Self { files }
    }
    
    /// Allocate the next free fd starting from 3 (preserving stdin/stdout/stderr slots).
    pub fn alloc_fd(&mut self, handle: Arc<dyn FileHandle>) -> Option<usize> {
        for (i, slot) in self.files.iter_mut().enumerate().skip(3) {
            if slot.is_none() {
                *slot = Some(FileDescriptor { handle, offset: 0 });
                return Some(i);
            }
        }
        None
    }

    /// Allocate fd 0 specifically (for first pipe-read end before any dup2 from terminald).
    pub fn alloc_fd_at(&mut self, fd: usize, handle: Arc<dyn FileHandle>) -> Option<usize> {
        if fd >= MAX_FDS { return None; }
        self.files[fd] = Some(FileDescriptor { handle, offset: 0 });
        Some(fd)
    }
    
    pub fn get_entry(&self, fd: usize) -> Option<FileDescriptor> {
        if fd >= self.files.len() {
             return None;
        }
        self.files[fd].clone()
    }

    /// Returns true if the given fd slot has a real file handle.
    pub fn get_fd(&self, fd: usize) -> Option<&FileDescriptor> {
        if fd >= self.files.len() { return None; }
        self.files[fd].as_ref()
    }

    pub fn get_handle(&self, fd: usize) -> Result<Arc<dyn FileHandle>, ()> {
        self.get_entry(fd).map(|e| e.handle).ok_or(())
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
    
    pub fn dup(&mut self, oldfd: usize) -> Option<usize> {
        if let Some(entry) = self.get_entry(oldfd) {
            for (i, slot) in self.files.iter_mut().enumerate().skip(3) {
                if slot.is_none() {
                    *slot = Some(entry);
                    return Some(i);
                }
            }
        }
        None
    }
    
    pub fn dup2(&mut self, oldfd: usize, newfd: usize) -> Option<usize> {
        if newfd >= MAX_FDS {
            return None;
        }
        if oldfd == newfd {
            return Some(newfd);
        }
        if let Some(entry) = self.get_entry(oldfd) {
            self.files[newfd] = Some(entry);
            Some(newfd)
        } else {
            None
        }
    }
}
