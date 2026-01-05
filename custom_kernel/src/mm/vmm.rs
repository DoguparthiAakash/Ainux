use core::arch::asm;
use crate::mm::pmm::{PAGE_SIZE, PMM, HHDM_OFFSET};
pub use crate::mm::pmm::HHDM_OFFSET as VMM_HHDM_OFFSET;
use core::sync::atomic::Ordering;
use core::slice;

// Page Table Entry Flags
pub const PRESENT: u64 = 1 << 0;
pub const WRITABLE: u64 = 1 << 1;
pub const USER: u64 = 1 << 2;
pub const HUGE_PAGE: u64 = 1 << 7;
pub const NX: u64 = 1 << 63;

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [u64; 512],
}

impl PageTable {
    pub fn new() -> Self {
        Self {
            entries: [0; 512],
        }
    }

    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = 0;
        }
    }
}

/// Helpers to manipulate addresses
fn p4_index(addr: u64) -> usize { ((addr >> 39) & 0x1ff) as usize }
fn p3_index(addr: u64) -> usize { ((addr >> 30) & 0x1ff) as usize }
fn p2_index(addr: u64) -> usize { ((addr >> 21) & 0x1ff) as usize }
fn p1_index(addr: u64) -> usize { ((addr >> 12) & 0x1ff) as usize }

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

pub fn init() {
    // Phase 2 VMM stub
}

unsafe fn active_pml4() -> &'static mut PageTable {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    // Deadlock fix: Don't lock PMM here.

    let cr3 = read_cr3();
    let phys_pml4 = cr3 & 0x000FFFFFFFFFF000;
    let virt_pml4 = phys_pml4 + hhdm_offset;
    
    // Debug
    // let mut serial = crate::SerialPort::new(0x3F8);
    // use core::fmt::Write;
    // let _ = write!(serial, "VMM: active_pml4 cr3={:#x}\n", cr3);

    &mut *(virt_pml4 as *mut PageTable)
}

unsafe fn get_next_table(entry: &mut u64) -> Option<&'static mut PageTable> {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    
    // Always clear NX bit to allow execution (Compromise for now)
    // If Limine set NX, we unset it so we can execute code we map.
    if *entry & crate::mm::vmm::NX != 0 {
         *entry &= !crate::mm::vmm::NX;
    }
    
    // Enforce USER bit to allow Ring 3 access to this path
    if *entry & crate::mm::vmm::USER == 0 {
        *entry |= crate::mm::vmm::USER;
    }
    
    if *entry & PRESENT == 0 {
        // Allocate a new table
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

pub unsafe fn map_page(virt: u64, phys: u64, flags: u64) -> Result<(), &'static str> {
    let pml4 = active_pml4();
    
    let p4 = get_next_table(&mut pml4.entries[p4_index(virt)]).ok_or("Failed to alloc P3")?;
    let p3 = get_next_table(&mut p4.entries[p3_index(virt)]).ok_or("Failed to alloc P2")?;
    let p2 = get_next_table(&mut p3.entries[p2_index(virt)]).ok_or("Failed to alloc P1")?;
    
    let pt_entry = &mut p2.entries[p1_index(virt)];
    
    if *pt_entry & PRESENT != 0 {
        return Err("Page already mapped");
    }
    
    *pt_entry = phys | flags;
    flush_tlb(virt);
    
    Ok(())
}

pub unsafe fn unmap_page(virt: u64) {
    let pml4 = active_pml4();
    
    let idx4 = p4_index(virt);
    if pml4.entries[idx4] & PRESENT == 0 { return; }
    let p4 = get_next_table(&mut pml4.entries[idx4]).unwrap();

    let idx3 = p3_index(virt);
    if p4.entries[idx3] & PRESENT == 0 { return; }
    let p3 = get_next_table(&mut p4.entries[idx3]).unwrap();

    let idx2 = p2_index(virt);
    if p3.entries[idx2] & PRESENT == 0 { return; }
    let p2 = get_next_table(&mut p3.entries[idx2]).unwrap();
    
    let idx1 = p1_index(virt);
    p2.entries[idx1] = 0;
    flush_tlb(virt);
}

// Map a page in a specific PML4 (physical address) via HHDM
pub unsafe fn map_page_in_pml4(pml4_phys: u64, vaddr: u64, paddr: u64, flags: u64) {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);
    let pml4 = slice::from_raw_parts_mut((pml4_phys + hhdm_offset) as *mut u64, 512);
    
    let p4_idx = ((vaddr >> 39) & 0x1FF) as usize;
    let p3_idx = ((vaddr >> 30) & 0x1FF) as usize;
    let p2_idx = ((vaddr >> 21) & 0x1FF) as usize;
    let p1_idx = ((vaddr >> 12) & 0x1FF) as usize;
    
    // Helper to get next table or allocate
    // Manually implemented to avoid borrow checker issues with recursion/helpers
    if pml4[p4_idx] & 1 == 0 {
        let mut pmm_lock = PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
             let frame_addr = pmm.alloc_frame().unwrap();
             core::ptr::write_bytes((frame_addr + hhdm_offset) as *mut u8, 0, 4096);
             pml4[p4_idx] = frame_addr | 0x7; // Present, RW, User
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
    
    pt[p1_idx] = paddr | flags | 1; // Present + flags
}
