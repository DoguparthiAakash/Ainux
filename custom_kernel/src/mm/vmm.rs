// =============================================================================
// Ainux Virtual Memory Manager — Multiboot / Identity-Mapped Edition
// Works with the boot.asm identity map (first 1GB: virt == phys).
// Higher-half kernel at 0xFFFFFFFF80000000.
// =============================================================================

use core::arch::asm;
use crate::mm::pmm::{PAGE_SIZE, PMM, HHDM_OFFSET};
pub use crate::mm::pmm::HHDM_OFFSET as VMM_HHDM_OFFSET;
use core::sync::atomic::Ordering;
use core::slice;

// Page Table Entry Flags
pub const PRESENT: u64 = 1 << 0;
pub const WRITABLE: u64 = 1 << 1;
pub const USER: u64 = 1 << 2;
pub const WRITE_THROUGH: u64 = 1 << 3;
pub const CACHE_DISABLE: u64 = 1 << 4;
pub const HUGE_PAGE: u64 = 1 << 7;
pub const GLOBAL: u64 = 1 << 8;
pub const NX: u64 = 1 << 63;

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [u64; 512],
}

impl PageTable {
    pub fn new() -> Self {
        Self { entries: [0; 512] }
    }

    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = 0;
        }
    }
}

/// Page table index helpers
#[inline(always)]
fn p4_index(addr: u64) -> usize { ((addr >> 39) & 0x1ff) as usize }
#[inline(always)]
fn p3_index(addr: u64) -> usize { ((addr >> 30) & 0x1ff) as usize }
#[inline(always)]
fn p2_index(addr: u64) -> usize { ((addr >> 21) & 0x1ff) as usize }
#[inline(always)]
fn p1_index(addr: u64) -> usize { ((addr >> 12) & 0x1ff) as usize }

// ---- CR3 / TLB Operations ----

pub unsafe fn read_cr3() -> u64 {
    let cr3: u64;
    asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    cr3
}

pub unsafe fn write_cr3(val: u64) {
    asm!("mov cr3, {}", in(reg) val, options(nomem, nostack, preserves_flags));
}

pub unsafe fn flush_tlb(addr: u64) {
    asm!("invlpg [{}]", in(reg) addr, options(nostack, preserves_flags));
}

pub unsafe fn flush_tlb_local(addr: u64) {
    flush_tlb(addr);
}

pub unsafe fn flush_tlb_global() {
    let cr3 = read_cr3();
    write_cr3(cr3);
}

/// Convert a physical address to a virtual address using HHDM offset.
/// For our identity-mapped boot setup, this is a no-op (offset = 0).
#[inline(always)]
pub fn phys_to_virt(phys: u64) -> u64 {
    phys + HHDM_OFFSET.load(Ordering::Relaxed)
}

/// Convert a virtual address back to physical (assuming HHDM mapping).
#[inline(always)]
pub fn virt_to_phys(virt: u64) -> u64 {
    virt - HHDM_OFFSET.load(Ordering::Relaxed)
}

/// Perform a full page table walk to resolve a virtual address to physical.
/// Works for both 4KB and 2MB pages.
pub unsafe fn virt_to_phys_walk(virt: u64) -> Option<u64> {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let cr3 = read_cr3();
    let phys_pml4 = cr3 & 0x000FFFFFFFFFF000;
    let pml4 = &*((phys_pml4 + hhdm_offset) as *const PageTable);

    let p4_idx = p4_index(virt);
    let p4_entry = pml4.entries[p4_idx];
    if p4_entry & PRESENT == 0 { return None; }

    let p3 = &*(((p4_entry & 0x000FFFFFFFFFF000) + hhdm_offset) as *const PageTable);
    let p3_idx = p3_index(virt);
    let p3_entry = p3.entries[p3_idx];
    if p3_entry & PRESENT == 0 { return None; }
    if p3_entry & HUGE_PAGE != 0 {
        return Some((p3_entry & 0x000FFFFFFC000000) + (virt & 0x3FFFFFFF)); // 1GB page
    }

    let p2 = &*(((p3_entry & 0x000FFFFFFFFFF000) + hhdm_offset) as *const PageTable);
    let p2_idx = p2_index(virt);
    let p2_entry = p2.entries[p2_idx];
    if p2_entry & PRESENT == 0 { return None; }
    if p2_entry & HUGE_PAGE != 0 {
        return Some((p2_entry & 0x000FFFFFFFE00000) + (virt & 0x1FFFFF)); // 2MB page
    }

    let p1 = &*(((p2_entry & 0x000FFFFFFFFFF000) + hhdm_offset) as *const PageTable);
    let p1_idx = p1_index(virt);
    let p1_entry = p1.entries[p1_idx];
    if p1_entry & PRESENT == 0 { return None; }

    Some((p1_entry & 0x000FFFFFFFFFF000) + (virt & 0xFFF))
}

pub static KERNEL_PML4: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

// ---- VMM Init ----

pub fn init() {
    unsafe {
        let cr3 = read_cr3();
        KERNEL_PML4.store(cr3 & 0x000FFFFFFFFFF000, Ordering::Relaxed);

        let pml4 = &mut *( (cr3 & 0x000FFFFFFFFFF000) as *mut PageTable );
        
        // Setup HHDM at 0xffff800000000000 (PML4 index 256)
        // We will map the first 4GB of physical memory using 4 PDs
        let mut pmm_lock = PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
            let pdpt_phys = pmm.alloc_frame().expect("Failed to allocate PDPT for HHDM");
            let pdpt = &mut *(pdpt_phys as *mut PageTable);
            pdpt.clear();
            pml4.entries[256] = pdpt_phys | PRESENT | WRITABLE;
            
            for i in 0..4 {
                let pd_phys = pmm.alloc_frame().expect("Failed to allocate PD for HHDM");
                let pd = &mut *(pd_phys as *mut PageTable);
                pd.clear();
                pdpt.entries[i] = pd_phys | PRESENT | WRITABLE;
                
                for j in 0..512 {
                    let phys = ((i * 512) + j) as u64 * 0x200000;
                    pd.entries[j] = phys | PRESENT | WRITABLE | HUGE_PAGE | GLOBAL;
                }
            }
        }
        drop(pmm_lock);
        
        let new_offset = 0xffff800000000000;
        HHDM_OFFSET.store(new_offset, Ordering::Relaxed);
        crate::mm::buddy::ZONED_PMM.lock().set_hhdm_offset(new_offset);
        
        flush_tlb_global();
    }
}

// ---- Page Table Traversal ----

unsafe fn active_pml4() -> &'static mut PageTable {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let cr3 = read_cr3();
    let phys_pml4 = cr3 & 0x000FFFFFFFFFF000;
    let virt_pml4 = phys_pml4 + hhdm_offset;
    &mut *(virt_pml4 as *mut PageTable)
}

unsafe fn get_next_table(entry: &mut u64, hhdm_offset: u64) -> Option<&'static mut PageTable> {
    if *entry & PRESENT == 0 {
        // Allocate a new page table frame
        let mut pmm_lock = PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
            if let Some(frame) = pmm.alloc_frame() {
                *entry = frame | PRESENT | WRITABLE | USER;
                let table_virt = frame + hhdm_offset;
                let table_ptr = table_virt as *mut PageTable;
                (*table_ptr).clear();
                return Some(&mut *table_ptr);
            }
        }
        return None;
    } else {
        let phys_addr = *entry & 0x000FFFFFFFFFF000;
        let virt_addr = phys_addr + hhdm_offset;
        return Some(&mut *(virt_addr as *mut PageTable));
    }
}

/// Get a next-level table without allocating (read-only traversal).
/// Returns None if the entry is not present.
unsafe fn get_existing_table(entry: u64, hhdm_offset: u64) -> Option<&'static mut PageTable> {
    if entry & PRESENT == 0 {
        return None;
    }
    let phys_addr = entry & 0x000FFFFFFFFFF000;
    let virt_addr = phys_addr + hhdm_offset;
    Some(&mut *(virt_addr as *mut PageTable))
}

// ---- Public Mapping API ----

pub unsafe fn map_page(virt: u64, phys: u64, flags: u64) -> Result<(), &'static str> {
    // Cache HHDM offset once — eliminates 4+ atomic loads per page walk
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let cr3 = read_cr3();
    let phys_pml4 = cr3 & 0x000FFFFFFFFFF000;
    let pml4 = &mut *((phys_pml4 + hhdm_offset) as *mut PageTable);

    let p4 = get_next_table(&mut pml4.entries[p4_index(virt)], hhdm_offset).ok_or("Failed to alloc P3")?;
    let p3 = get_next_table(&mut p4.entries[p3_index(virt)], hhdm_offset).ok_or("Failed to alloc P2")?;
    let p2 = get_next_table(&mut p3.entries[p2_index(virt)], hhdm_offset).ok_or("Failed to alloc P1")?;

    let pt_entry = &mut p2.entries[p1_index(virt)];

    if *pt_entry & PRESENT != 0 {
        // Page already mapped — remap silently instead of erroring
        *pt_entry = phys | flags;
        flush_tlb(virt);
        return Ok(());
    }

    *pt_entry = phys | flags;
    flush_tlb(virt);
    
    // Maturation: Record allocation in current task metrics
    crate::process::scheduler::increment_current_page_count();
    
    Ok(())
}

// ---- Backend API for AddressSpace ----

/// Map a page by allocating a new physical frame in a specific PML4.
pub unsafe fn map_page_allocate_in_pml4(pml4_phys: u64, virt: u64, flags: u64) -> Result<(), &'static str> {
    let mut pmm_lock = PMM.lock();
    let phys = if let Some(ref mut pmm) = *pmm_lock {
        pmm.alloc_frame().ok_or("Out of physical memory")?
    } else {
        return Err("PMM not initialized");
    };
    drop(pmm_lock);
    
    // Zero the frame (via HHDM)
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    core::ptr::write_bytes((phys + hhdm_offset) as *mut u8, 0, crate::mm::pmm::PAGE_SIZE);

    map_page_in_pml4(pml4_phys, virt, phys, flags);
    Ok(())
}

pub unsafe fn map_page_allocate(virt: u64, flags: u64) -> Result<(), &'static str> {
    let pml4_phys = read_cr3() & 0x000FFFFFFFFFF000;
    map_page_allocate_in_pml4(pml4_phys, virt, flags)
}

/// Helper for mapping a region chunk in a specific PML4. 
pub unsafe fn map_region_backend_in_pml4(pml4_phys: u64, virt: u64, _size: u64, flags: u64) -> Result<(), &'static str> {
    map_page_allocate_in_pml4(pml4_phys, virt, flags)
}

/// Unmap a single page (backend alias)
pub unsafe fn unmap_page_backend(virt: u64) {
    unmap_page(virt);
}

pub unsafe fn unmap_page_in_pml4(pml4_phys: u64, virt: u64) {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let pml4 = &mut *((pml4_phys + hhdm_offset) as *mut PageTable);

    let p4 = match get_existing_table(pml4.entries[p4_index(virt)], hhdm_offset) {
        Some(t) => t,
        None => return,
    };

    let p3 = match get_existing_table(p4.entries[p3_index(virt)], hhdm_offset) {
        Some(t) => t,
        None => return,
    };

    let p2 = match get_existing_table(p3.entries[p2_index(virt)], hhdm_offset) {
        Some(t) => t,
        None => return,
    };

    let idx1 = p1_index(virt);
    p2.entries[idx1] = 0;
    
    if pml4_phys == (read_cr3() & 0x000FFFFFFFFFF000) {
        flush_tlb(virt);
    }
}

/// Change protection flags of an existing mapped page in a specific PML4
pub unsafe fn protect_page_backend_in_pml4(pml4_phys: u64, virt: u64, new_flags: u64) -> Result<(), &'static str> {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let pml4 = &mut *((pml4_phys + hhdm_offset) as *mut PageTable);

    let p4 = get_existing_table(pml4.entries[p4_index(virt)], hhdm_offset).ok_or("P4 entry not present")?;
    let p3 = get_existing_table(p4.entries[p3_index(virt)], hhdm_offset).ok_or("P3 entry not present")?;
    let p2 = get_existing_table(p3.entries[p2_index(virt)], hhdm_offset).ok_or("P2 entry not present")?;

    let pt_entry = &mut p2.entries[p1_index(virt)];

    if *pt_entry & PRESENT == 0 {
        return Err("Page not present for protection change");
    }

    let phys = *pt_entry & 0x000FFFFFFFFFF000;
    *pt_entry = phys | new_flags;
    
    if pml4_phys == (read_cr3() & 0x000FFFFFFFFFF000) {
        flush_tlb(virt);
    }
    Ok(())
}

/// Map a 2MB huge page (PD-level mapping, no PT needed).
/// `virt` and `phys` must be 2MB-aligned.
pub unsafe fn map_page_2mb(virt: u64, phys: u64, flags: u64) -> Result<(), &'static str> {
    if virt & 0x1FFFFF != 0 || phys & 0x1FFFFF != 0 {
        return Err("2MB map: addresses not 2MB-aligned");
    }

    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let cr3 = read_cr3();
    let phys_pml4 = cr3 & 0x000FFFFFFFFFF000;
    let pml4 = &mut *((phys_pml4 + hhdm_offset) as *mut PageTable);

    let p4 = get_next_table(&mut pml4.entries[p4_index(virt)], hhdm_offset).ok_or("Failed to alloc P3")?;
    let p3 = get_next_table(&mut p4.entries[p3_index(virt)], hhdm_offset).ok_or("Failed to alloc P2")?;

    // Write directly to PD entry with HUGE_PAGE flag — no PT level
    p3.entries[p2_index(virt)] = phys | flags | HUGE_PAGE | PRESENT;
    flush_tlb(virt);
    Ok(())
}

/// Remaps a physical memory region in the HHDM to be Write-Combining (WC).
/// Assumes PAT1 is configured as WC and mapped via WRITE_THROUGH flag.
pub unsafe fn remap_hhdm_pages_wc(phys_addr: u64, size_bytes: usize) {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let cr3 = read_cr3();
    let phys_pml4 = cr3 & 0x000FFFFFFFFFF000;
    let pml4 = &mut *((phys_pml4 + hhdm_offset) as *mut PageTable);

    let start_align = phys_addr & !(0x1FFFFF); // 2MB align down
    let end_align = (phys_addr + size_bytes as u64 + 0x1FFFFF) & !(0x1FFFFF); // 2MB align up
    
    let mut current = start_align;
    while current < end_align {
        let virt = current + hhdm_offset;
        
        let p4_idx = p4_index(virt);
        let p3_idx = p3_index(virt);
        let p2_idx = p2_index(virt);
        
        if let Some(p4) = get_existing_table(pml4.entries[p4_idx], hhdm_offset) {
            if let Some(p3) = get_existing_table(p4.entries[p3_idx], hhdm_offset) {
                let entry = &mut p3.entries[p2_idx];
                if *entry & HUGE_PAGE != 0 {
                    *entry |= WRITE_THROUGH; // Use PAT1 (WC)
                    flush_tlb(virt);
                }
            }
        }
        
        current += 0x200000;
    }
}

pub unsafe fn unmap_page(virt: u64) {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let cr3 = read_cr3();
    let phys_pml4 = cr3 & 0x000FFFFFFFFFF000;
    let pml4 = &mut *((phys_pml4 + hhdm_offset) as *mut PageTable);

    let p4 = match get_existing_table(pml4.entries[p4_index(virt)], hhdm_offset) {
        Some(t) => t,
        None => return,
    };

    let p3 = match get_existing_table(p4.entries[p3_index(virt)], hhdm_offset) {
        Some(t) => t,
        None => return,
    };

    let p2 = match get_existing_table(p3.entries[p2_index(virt)], hhdm_offset) {
        Some(t) => t,
        None => return,
    };

    let idx1 = p1_index(virt);
    p2.entries[idx1] = 0;
    flush_tlb(virt);
}

/// Map a page in a specific PML4 (given by physical address)
pub unsafe fn map_page_in_pml4(pml4_phys: u64, vaddr: u64, paddr: u64, flags: u64) {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let pml4 = slice::from_raw_parts_mut((pml4_phys + hhdm_offset) as *mut u64, 512);

    let p4_idx = ((vaddr >> 39) & 0x1FF) as usize;
    let p3_idx = ((vaddr >> 30) & 0x1FF) as usize;
    let p2_idx = ((vaddr >> 21) & 0x1FF) as usize;
    let p1_idx = ((vaddr >> 12) & 0x1FF) as usize;

    // Propagate USER bit to all intermediate levels if requested
    let user_bit = flags & 0x4; // bit 2 = USER

    if pml4[p4_idx] & 1 == 0 {
        let mut pmm_lock = PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
            let frame_addr = pmm.alloc_frame().unwrap();
            core::ptr::write_bytes((frame_addr + hhdm_offset) as *mut u8, 0, 4096);
            pml4[p4_idx] = frame_addr | 0x3 | user_bit; // Present | Writable | (User?)
        }
    } else {
        pml4[p4_idx] |= user_bit; // Ensure USER bit is set on existing entry
    }

    let pdpt_phys = pml4[p4_idx] & 0x000FFFFFFFFFF000;
    let pdpt = slice::from_raw_parts_mut((pdpt_phys + hhdm_offset) as *mut u64, 512);

    if pdpt[p3_idx] & 1 == 0 {
        let mut pmm_lock = PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
            let frame_addr = pmm.alloc_frame().unwrap();
            core::ptr::write_bytes((frame_addr + hhdm_offset) as *mut u8, 0, 4096);
            pdpt[p3_idx] = frame_addr | 0x3 | user_bit;
        }
    } else {
        pdpt[p3_idx] |= user_bit;
    }

    let pd_phys = pdpt[p3_idx] & 0x000FFFFFFFFFF000;
    let pd = slice::from_raw_parts_mut((pd_phys + hhdm_offset) as *mut u64, 512);

    if pd[p2_idx] & 1 == 0 {
        let mut pmm_lock = PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
            let frame_addr = pmm.alloc_frame().unwrap();
            core::ptr::write_bytes((frame_addr + hhdm_offset) as *mut u8, 0, 4096);
            pd[p2_idx] = frame_addr | 0x3 | user_bit;
        }
    } else {
        pd[p2_idx] |= user_bit;
    }

    let pt_phys = pd[p2_idx] & 0x000FFFFFFFFFF000;
    let pt = slice::from_raw_parts_mut((pt_phys + hhdm_offset) as *mut u64, 512);

    pt[p1_idx] = paddr | flags | 1;
}

/// Creates a new address space (PML4) by copying kernel mappings from the active one.
pub fn create_address_space() -> u64 {
    let mut pmm_lock = PMM.lock();
    if let Some(ref mut pmm) = *pmm_lock {
        if let Some(pml4_phys) = pmm.alloc_frame() {
            let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
            let pml4_virt = pml4_phys + hhdm_offset;
            unsafe {
                let pml4 = &mut *(pml4_virt as *mut PageTable);
                pml4.clear();
                
                // Copy kernel mappings (top 256 entries)
                let active = active_pml4();
                for i in 256..512 {
                    pml4.entries[i] = active.entries[i];
                }
                
                // Do NOT copy entry 0 — user pages will be mapped fresh
                // pml4.entries[0] = active.entries[0]; // REMOVED: caused USER bit missing
            }
            return pml4_phys;
        }
    }
    0
}
/// Clones the user-space portion of an address space (PML4)
pub unsafe fn clone_address_space(src_pml4_phys: u64) -> u64 {
    let dest_pml4_phys = create_address_space();
    if dest_pml4_phys == 0 {
        return 0;
    }
    
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let src_pml4 = &*((src_pml4_phys + hhdm_offset) as *const PageTable);
    
    // Only clone user space (indices 0..256)
    for i in 0..256 {
        let entry4 = src_pml4.entries[i];
        if entry4 & PRESENT != 0 && entry4 & HUGE_PAGE == 0 {
            let p3_phys = entry4 & 0x000FFFFFFFFFF000;
            let src_p3 = &*((p3_phys + hhdm_offset) as *const PageTable);
            
            for j in 0..512 {
                let entry3 = src_p3.entries[j];
                if entry3 & PRESENT != 0 && entry3 & HUGE_PAGE == 0 {
                    let p2_phys = entry3 & 0x000FFFFFFFFFF000;
                    let src_p2 = &*((p2_phys + hhdm_offset) as *const PageTable);
                    
                    for k in 0..512 {
                        let entry2 = src_p2.entries[k];
                        if entry2 & PRESENT != 0 && entry2 & HUGE_PAGE == 0 {
                            let p1_phys = entry2 & 0x000FFFFFFFFFF000;
                            let src_p1 = &*((p1_phys + hhdm_offset) as *const PageTable);
                            
                            for l in 0..512 {
                                let entry1 = src_p1.entries[l];
                                if entry1 & PRESENT != 0 {
                                    let src_frame_phys = entry1 & 0x000FFFFFFFFFF000;
                                    let flags = entry1 & 0xFFF;
                                    
                                    // Allocate a new frame
                                    let mut pmm_lock = PMM.lock();
                                    let new_frame_phys = if let Some(ref mut pmm) = *pmm_lock {
                                        pmm.alloc_frame()
                                    } else {
                                        None
                                    };
                                    drop(pmm_lock);
                                    
                                    if let Some(new_frame_phys) = new_frame_phys {
                                        // Copy the data
                                        let src_ptr = (src_frame_phys + hhdm_offset) as *const u8;
                                        let dest_ptr = (new_frame_phys + hhdm_offset) as *mut u8;
                                        core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, 4096);
                                        
                                        // Map it in the new address space
                                        let vaddr = ((i as u64) << 39) | ((j as u64) << 30) | ((k as u64) << 21) | ((l as u64) << 12);
                                        map_page_in_pml4(dest_pml4_phys, vaddr, new_frame_phys, flags);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    dest_pml4_phys
}

/// Destroys an address space by recursively freeing all userspace frames.
pub unsafe fn destroy_address_space(pml4_phys: u64) {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let pml4 = &mut *((pml4_phys + hhdm_offset) as *mut PageTable);

    // Only free the userspace quadrant (0-255)
    for i in 0..256 {
        let entry = pml4.entries[i];
        if entry & PRESENT != 0 && entry & HUGE_PAGE == 0 {
            let p3_phys = entry & 0x000FFFFFFFFFF000;
            let p3 = &mut *((p3_phys + hhdm_offset) as *mut PageTable);
            
            for j in 0..512 {
                let entry3 = p3.entries[j];
                if entry3 & PRESENT != 0 && entry3 & HUGE_PAGE == 0 {
                    let p2_phys = entry3 & 0x000FFFFFFFFFF000;
                    let p2 = &mut *((p2_phys + hhdm_offset) as *mut PageTable);
                    
                    for k in 0..512 {
                        let entry2 = p2.entries[k];
                        if entry2 & PRESENT != 0 && entry2 & HUGE_PAGE == 0 {
                            let p1_phys = entry2 & 0x000FFFFFFFFFF000;
                            let p1 = &mut *((p1_phys + hhdm_offset) as *mut PageTable);
                            
                            // Free all data frames (level 1 entries)
                            for l in 0..512 {
                                let entry1 = p1.entries[l];
                                if entry1 & PRESENT != 0 {
                                    let data_phys = entry1 & 0x000FFFFFFFFFF000;
                                    let mut pmm = PMM.lock();
                                    if let Some(ref mut pmm) = *pmm {
                                        pmm.free_frame(data_phys);
                                    }
                                }
                            }

                            let mut pmm = PMM.lock();
                            if let Some(ref mut pmm) = *pmm {
                                pmm.free_frame(p1_phys);
                            }
                        }
                    }
                    let mut pmm = PMM.lock();
                    if let Some(ref mut pmm) = *pmm {
                        pmm.free_frame(p2_phys);
                    }
                }
            }
            let mut pmm = PMM.lock();
            if let Some(ref mut pmm) = *pmm {
                pmm.free_frame(p3_phys);
            }
        }
    }

    // Finally free the PML4 itself
    let mut pmm = PMM.lock();
    if let Some(ref mut pmm) = *pmm {
        pmm.free_frame(pml4_phys);
    }
}
