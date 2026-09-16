use core::cmp::{Ord, Ordering};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

bitflags::bitflags! {
    /// Permissions for a Memory Region
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RegionPermissions: u8 {
        const READ    = 1 << 0;
        const WRITE   = 1 << 1;
        const EXECUTE = 1 << 2;
        const USER    = 1 << 3;
    }
}

bitflags::bitflags! {
    /// Extended flags for memory behavior
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RegionFlags: u16 {
        const COW       = 1 << 0;
        const GROW_DOWN = 1 << 1;
        const GROW_UP   = 1 << 2;
        const GUARD     = 1 << 3;
        const WIRED     = 1 << 4; // Locked in memory, cannot be paged out
    }
}

/// The type of backing storage for this memory region
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionType {
    /// Zero-filled memory, not backed by a file
    Anonymous,
    /// Backed by a file (inode reference omitted until VFS is implemented)
    FileBacked,
    /// Shared memory object
    Shared,
    /// Device memory (MMIO)
    Device,
}

/// Represents a contiguous range of virtual memory with specific attributes
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub length: u64,
    pub permissions: RegionPermissions,
    pub mapping_type: RegionType,
    pub flags: RegionFlags,
    pub sharing_state: u32, // Placeholder for actual sharing metadata
    pub cow_state: u32,     // Placeholder for actual COW metadata
}

impl MemoryRegion {
    pub fn new(
        start: u64,
        length: u64,
        permissions: RegionPermissions,
        mapping_type: RegionType,
        flags: RegionFlags,
    ) -> Self {
        let end = start.checked_add(length).expect("MemoryRegion end overflow");
        Self {
            start,
            end,
            length,
            permissions,
            mapping_type,
            flags,
            sharing_state: 0,
            cow_state: 0,
        }
    }

    /// Check if this region contains a specific address
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.end
    }

    /// Check if this region overlaps with a given range [start, end)
    pub fn overlaps(&self, start: u64, end: u64) -> bool {
        self.start < end && start < self.end
    }
}

/// A cache-friendly tree map for memory regions.
/// Currently implemented via BTreeMap.
#[derive(Debug, Clone)]
pub struct RegionMap {
    /// Maps region `start` address to the `MemoryRegion`.
    regions: BTreeMap<u64, MemoryRegion>,
}

impl RegionMap {
    pub fn new() -> Self {
        Self {
            regions: BTreeMap::new(),
        }
    }

    /// Attempt to insert a new region. Rejects overlapping regions.
    pub fn insert_region(&mut self, region: MemoryRegion) -> Result<(), &'static str> {
        if region.length == 0 {
            return Err("Zero-length region");
        }

        // Check for overlaps. We need to find if any existing region overlaps with the new one.
        // A full BTreeMap iteration is O(N). In a robust implementation, we'd use a BTreeMap
        // combined with an interval check, but for now we'll do an intersection check.
        if self.find_intersection(region.start, region.end).is_some() {
            return Err("Overlapping region");
        }

        self.regions.insert(region.start, region);
        Ok(())
    }

    /// Find a region containing the specified address
    pub fn find_region(&self, address: u64) -> Option<&MemoryRegion> {
        // range(..=address) gives us an iterator up to the address.
        // The last element is the region with the highest start address <= `address`.
        if let Some((_, region)) = self.regions.range(..=address).next_back() {
            if region.contains(address) {
                return Some(region);
            }
        }
        None
    }

    /// Find any region that intersects with [start, end)
    pub fn find_intersection(&self, start: u64, end: u64) -> Option<&MemoryRegion> {
        // Any region starting before `end` might overlap.
        for (_, region) in self.regions.range(..end) {
            if region.overlaps(start, end) {
                return Some(region);
            }
        }
        None
    }

    /// Remove a region by its exact start address
    pub fn remove_region(&mut self, start: u64) -> Option<MemoryRegion> {
        self.regions.remove(&start)
    }
}
