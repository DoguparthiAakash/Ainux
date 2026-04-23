// src/fs/pipe.rs
// Unix-style Anonymous Pipes for Ainux

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use crate::fs::vfs::{FileHandle, VfsResult, VfsError};

pub struct Pipe {
    buffer: Mutex<VecDeque<u8>>,
    max_size: usize,
}

impl Pipe {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            buffer: Mutex::new(VecDeque::with_capacity(4096)),
            max_size: 4096,
        })
    }
}

#[derive(Debug)]
pub struct PipeReader {
    pipe: Arc<Pipe>,
}

#[derive(Debug)]
pub struct PipeWriter {
    pipe: Arc<Pipe>,
}

// Manually implement Debug for Pipe because Mutex might not have it in no_std easily 
// or it's cleaner to just show it's a pipe.
impl core::fmt::Debug for Pipe {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Pipe {{ size: {} }}", self.buffer.lock().len())
    }
}

impl FileHandle for PipeReader {
    fn read(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        let mut buffer = self.pipe.buffer.lock();
        if buffer.is_empty() {
            return Ok(0); // In a real system, we'd block here if writer is still open
        }
        
        let mut read = 0;
        for i in 0..buf.len() {
            if let Some(b) = buffer.pop_front() {
                buf[i] = b;
                read += 1;
            } else {
                break;
            }
        }
        Ok(read)
    }

    fn write(&self, _buf: &[u8], _offset: u64) -> VfsResult<usize> {
        Err(VfsError::PermissionDenied)
    }

    fn truncate(&self) -> VfsResult<()> {
        Err(VfsError::PermissionDenied)
    }

    fn close(&self) -> VfsResult<()> { Ok(()) }
}

impl FileHandle for PipeWriter {
    fn read(&self, _buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        Err(VfsError::PermissionDenied)
    }

    fn write(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        let mut buffer = self.pipe.buffer.lock();
        let mut written = 0;
        for b in buf {
            if buffer.len() < self.pipe.max_size {
                buffer.push_back(*b);
                written += 1;
            } else {
                break; // Pipe full
            }
        }
        Ok(written)
    }

    fn close(&self) -> VfsResult<()> { Ok(()) }

    fn truncate(&self) -> VfsResult<()> {
        Err(VfsError::PermissionDenied)
    }
}

pub fn create_pipe() -> (Arc<PipeReader>, Arc<PipeWriter>) {
    let pipe = Pipe::new();
    (
        Arc::new(PipeReader { pipe: pipe.clone() }),
        Arc::new(PipeWriter { pipe }),
    )
}
