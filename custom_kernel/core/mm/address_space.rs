use alloc::sync::Arc;
use crate::mm::memory_region::{MemoryRegion, RegionMap, RegionPermissions, RegionType, RegionFlags};
use crate::mm::vmm;

/// Generic Virtual Address Space
/// Owns the region metadata and interacts with the architecture-specific VMM backend.
#[derive(Debug)]
pub struct AddressSpace {
    /// True if this is the kernel address space.
    pub is_kernel: bool,
    /// The map of all memory regions.
    regions: RegionMap,
    /// The physical address of the page table root (PML4 on x86_64).
    pub pml4_phys: u64,
}

impl Drop for AddressSpace {
    fn drop(&mut self) {
        if !self.is_kernel && self.pml4_phys != 0 {
            unsafe {
                // Free the architecture-specific page table root
                vmm::destroy_address_space(self.pml4_phys);
            }
        }
    }
}

impl AddressSpace {
    /// Create a new empty AddressSpace for a user process.
    pub fn new_user() -> Self {
        Self {
            is_kernel: false,
            regions: RegionMap::new(),
            pml4_phys: vmm::create_address_space(),
        }
    }

    /// Create the kernel AddressSpace.
    pub fn new_kernel() -> Self {
        Self {
            is_kernel: true,
            regions: RegionMap::new(),
            pml4_phys: unsafe { vmm::KERNEL_PML4.load(core::sync::atomic::Ordering::Relaxed) },
        }
    }

    /// Maps a new region into the address space.
    /// Returns the start address of the region on success.
    pub fn map_region(
        &mut self,
        start: u64,
        length: u64,
        permissions: RegionPermissions,
        mapping_type: RegionType,
        flags: RegionFlags,
    ) -> Result<u64, &'static str> {
        let region = MemoryRegion::new(start, length, permissions, mapping_type, flags);
        
        // Ensure no overlap
        self.regions.insert_region(region.clone())?;

        // Inform the VMM backend to map the pages.
        // We only map if it's NOT demand-paged, or we can eagerly map anonymous memory.
        // For Phase 2, we will just call the backend `map_page` for the entire region.
        let end = start.checked_add(length).unwrap();
        let mut addr = start;
        while addr < end {
            unsafe {
                // Determine VMM flags from RegionPermissions
                let mut vmm_flags = vmm::PRESENT;
                if permissions.contains(RegionPermissions::WRITE) {
                    vmm_flags |= vmm::WRITABLE;
                }
                if permissions.contains(RegionPermissions::USER) {
                    vmm_flags |= vmm::USER;
                }
                
                // Currently, we don't have a direct frame allocated for Anonymous mappings
                // unless we do it here, or demand page it. If we are doing eager mapping:
                // We assume `vmm::map_page_allocate` exists, or we demand page later.
                // For this stub to compile and be robust, we rely on the VMM backend.
                // We'll leave the actual VMM backend integration to `vmm::map_region_backend_in_pml4`.
                vmm::map_region_backend_in_pml4(self.pml4_phys, addr, crate::mm::pmm::PAGE_SIZE as u64, vmm_flags)?;
            }
            addr += crate::mm::pmm::PAGE_SIZE as u64;
        }

        Ok(start)
    }

    /// Unmaps a region starting exactly at `start`.
    pub fn unmap_region(&mut self, start: u64) -> Result<(), &'static str> {
        if let Some(region) = self.regions.remove_region(start) {
            let mut addr = region.start;
            while addr < region.end {
                unsafe {
                    vmm::unmap_page_in_pml4(self.pml4_phys, addr);
                }
                addr += crate::mm::pmm::PAGE_SIZE as u64;
            }
            Ok(())
        } else {
            Err("Region not found")
        }
    }

    /// Finds a region containing the given address.
    pub fn find_region(&self, address: u64) -> Option<&MemoryRegion> {
        self.regions.find_region(address)
    }

    /// Changes the protection flags for a region containing `address`.
    pub fn protect_region(&mut self, address: u64, new_permissions: RegionPermissions) -> Result<(), &'static str> {
        // We need mutable access to the region.
        // BTreeMap doesn't let us mutate keys, but we can mutate values.
        // For now, remove and re-insert is easiest and safest.
        let region = self.find_region(address).cloned().ok_or("Region not found")?;
        
        let start = region.start;
        let mut new_region = region;
        new_region.permissions = new_permissions;
        
        // Update backend
        let mut addr = new_region.start;
        while addr < new_region.end {
            unsafe {
                let mut vmm_flags = vmm::PRESENT;
                if new_permissions.contains(RegionPermissions::WRITE) {
                    vmm_flags |= vmm::WRITABLE;
                }
                if new_permissions.contains(RegionPermissions::USER) {
                    vmm_flags |= vmm::USER;
                }
                vmm::protect_page_backend_in_pml4(self.pml4_phys, addr, vmm_flags)?;
            }
            addr += crate::mm::pmm::PAGE_SIZE as u64;
        }

        self.regions.remove_region(start);
        self.regions.insert_region(new_region)?;
        Ok(())
    }

    /// Clone this address space for COW.
    pub fn clone_cow(&self) -> Result<Self, &'static str> {
        Err("COW is not fully implemented yet")
    }

    /// Handle a page fault.
    pub fn handle_page_fault(&mut self, fault_addr: u64, error_code: u64) -> bool {
        // 1. Look up region
        if let Some(region) = self.find_region(fault_addr) {
            // Check permissions
            let is_write = (error_code & 2) != 0;
            let is_user = (error_code & 4) != 0;

            if is_user && !region.permissions.contains(RegionPermissions::USER) {
                return false; // Protection fault (user accessed kernel memory)
            }

            if is_write && !region.permissions.contains(RegionPermissions::WRITE) {
                // If it's a write on a read-only page, check for COW.
                if region.flags.contains(RegionFlags::COW) {
                    // Perform COW copy (deferred to future implementation)
                    // return true once resolved
                    return false;
                }
                return false; // Protection fault (write to read-only memory)
            }

            // Demand paging
            // If the page is not present (error_code & 1 == 0), allocate and map
            if (error_code & 1) == 0 {
                // Allocate physical page and map it via backend
                unsafe {
                    let mut vmm_flags = vmm::PRESENT;
                    if region.permissions.contains(RegionPermissions::WRITE) {
                        vmm_flags |= vmm::WRITABLE;
                    }
                    if region.permissions.contains(RegionPermissions::USER) {
                        vmm_flags |= vmm::USER;
                    }
                    // For Phase 2, we just try to allocate and map one page
                    if vmm::map_page_allocate_in_pml4(self.pml4_phys, fault_addr & !0xFFF, vmm_flags).is_ok() {
                        return true;
                    }
                }
            }
        }
        false
    }
}
