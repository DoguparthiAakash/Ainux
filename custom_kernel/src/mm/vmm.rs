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

// ---- VMM Init ----

pub fn init() {
    // VMM init is a stub — the boot.asm page tables are sufficient for early boot.
    // The real work happens when we map heap pages, user pages, etc.
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

    if pml4[p4_idx] & 1 == 0 {
        let mut pmm_lock = PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
            let frame_addr = pmm.alloc_frame().unwrap();
            core::ptr::write_bytes((frame_addr + hhdm_offset) as *mut u8, 0, 4096);
            pml4[p4_idx] = frame_addr | 0x7;
        }
    }

    let pdpt_phys = pml4[p4_idx] & 0x000FFFFFFFFFF000;
    let pdpt = slice::from_raw_parts_mut((pdpt_phys + hhdm_offset) as *mut u64, 512);

    if pdpt[p3_idx] & 1 == 0 {
        let mut pmm_lock = PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
            let frame_addr = pmm.alloc_frame().unwrap();
            core::ptr::write_bytes((frame_addr + hhdm_offset) as *mut u8, 0, 4096);
            pdpt[p3_idx] = frame_addr | 0x7;
        }
    }

    let pd_phys = pdpt[p3_idx] & 0x000FFFFFFFFFF000;
    let pd = slice::from_raw_parts_mut((pd_phys + hhdm_offset) as *mut u64, 512);

    if pd[p2_idx] & 1 == 0 {
        let mut pmm_lock = PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
            let frame_addr = pmm.alloc_frame().unwrap();
            core::ptr::write_bytes((frame_addr + hhdm_offset) as *mut u8, 0, 4096);
            pd[p2_idx] = frame_addr | 0x7;
        }
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
                
                // Also copy the first entry for identity mapping (TEMPORARY: until bootstrap finished)
                pml4.entries[0] = active.entries[0];
            }
            return pml4_phys;
        }
    }
    0
}

