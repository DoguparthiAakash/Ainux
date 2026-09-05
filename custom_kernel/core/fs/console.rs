use alloc::string::String;
use alloc::sync::Arc;
use crate::fs::vfs::{FileHandle, VfsResult, VfsError};
use crate::drivers::video;

#[derive(Debug)]
pub struct ConsoleHandle;

impl FileHandle for ConsoleHandle {
    fn read(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        // Mock standard input from serial/keyboard for currently blocking version
        Ok(0)
    }

    fn write(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        if let Ok(s) = core::str::from_utf8(buf) {
            video::put_str(s);
            Ok(buf.len())
        } else {
            Err(VfsError::IOError)
        }
    }

    fn truncate(&self) -> VfsResult<()> { Ok(()) }
    fn close(&self) -> VfsResult<()> { Ok(()) }
}
