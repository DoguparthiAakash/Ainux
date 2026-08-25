use core::slice;

/// Address space constants
/// Using 48-bit canonical addressing typical for x86_64
pub const USER_MAX_ADDR: u64 = 0x0000_7FFF_FFFF_FFFF;
pub const KERNEL_MIN_ADDR: u64 = 0xFFFF_8000_0000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserError {
    InvalidPointer,
    BufferTooLarge,
    AccessViolation,
}

/// Validates an arbitrary virtual address range.
/// Rejects non-canonical addresses and integer overflows.
#[inline]
pub fn validate_virtual_range(ptr: u64, len: usize) -> bool {
    if len == 0 {
        return true; // Zero length is trivially valid
    }
    let end = match ptr.checked_add(len as u64) {
        Some(e) => e,
        None => return false, // Overflow
    };
    
    // Both start and (end - 1) must be canonical.
    // Canonical: bits 48-63 must be copies of bit 47.
    // User: < 0x0000_8000_0000_0000
    // Kernel: >= 0xFFFF_8000_0000_0000
    let start_canonical = (ptr <= USER_MAX_ADDR) || (ptr >= KERNEL_MIN_ADDR);
    let end_canonical = ((end - 1) <= USER_MAX_ADDR) || ((end - 1) >= KERNEL_MIN_ADDR);
    
    // Also, a range cannot span the non-canonical gap.
    let spans_gap = ptr <= USER_MAX_ADDR && (end - 1) >= KERNEL_MIN_ADDR;

    start_canonical && end_canonical && !spans_gap
}

/// Validates that a user-provided pointer range lies entirely within userspace.
#[inline]
pub fn validate_user_range(ptr: u64, len: usize) -> bool {
    if len == 0 {
        return true;
    }
    let end = match ptr.checked_add(len as u64) {
        Some(e) => e,
        None => return false, // Overflow
    };

    // Range must be strictly within userspace
    if end > (USER_MAX_ADDR + 1) { // end is exclusive, so max valid end is USER_MAX_ADDR + 1
        return false;
    }

    validate_virtual_range(ptr, len)
}

/// Safely copies data from userspace to a kernel buffer.
pub fn copy_from_user(src: *const u8, dest: &mut [u8]) -> Result<(), UserError> {
    if !validate_user_range(src as u64, dest.len()) {
        return Err(UserError::InvalidPointer);
    }
    unsafe {
        core::ptr::copy_nonoverlapping(src, dest.as_mut_ptr(), dest.len());
    }
    Ok(())
}

/// Safely copies data from kernel buffer to userspace.
pub fn copy_to_user(dest: *mut u8, src: &[u8]) -> Result<(), UserError> {
    if !validate_user_range(dest as u64, src.len()) {
        return Err(UserError::InvalidPointer);
    }
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dest, src.len());
    }
    Ok(())
}
