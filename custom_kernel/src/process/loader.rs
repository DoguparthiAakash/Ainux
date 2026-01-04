use super::task::{Task, TaskState};
use super::scheduler; // We need access to create tasks.
use crate::fs::elf::{ElfHeader, ProgramHeader, PT_LOAD, PF_W, PF_X};
use crate::mm::vmm;
use crate::mm::pmm;
use alloc::vec::Vec;
use core::slice;

// Error types
#[derive(Debug)]
pub enum LoadError {
    InvalidElf,
    UnsupportedArch,
    MemoryError,
}

// Helper for PMM allocation
fn alloc_frame_safe() -> Result<u64, LoadError> {
    let mut pmm_lock = pmm::PMM.lock();
    if let Some(ref mut pmm) = *pmm_lock {
        pmm.alloc_frame().map(|f| f as u64).ok_or(LoadError::MemoryError)
    } else {
        Err(LoadError::MemoryError)
    }
}

// Load an ELF binary into a new process
pub fn load_elf(data: &[u8]) -> Result<usize, LoadError> {
    const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
    
    if data.len() < core::mem::size_of::<ElfHeader>() {
        return Err(LoadError::InvalidElf);
    }
    
    let header = unsafe { &*(data.as_ptr() as *const ElfHeader) };
    
    if header.magic != ELF_MAGIC {
        return Err(LoadError::InvalidElf);
    }
    
    if header.machine != 0x3E { // x86_64
        return Err(LoadError::UnsupportedArch);
    }
    
    // Create Valid User Page Table
    // 1. Allocate a PMM frame for PML4
    let pml4_frame = alloc_frame_safe()?;
    let pml4_addr = pml4_frame * 4096; // Physical
    
    // 2. We need to initialize this PML4. 
    // It must map the KERNEL (upper half) identically to current kernel PML4.
    unsafe {
        // Assuming current CR3 is kernel's
        let current_cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) current_cr3);
        
        let hhdm_offset = vmm::VMM_HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        
        // Map as temporary to copy
        // Current PML4 virtual address
        let src_pml4_phys = current_cr3 & 0x000FFFFFFFFFF000;
        let src_ptr = (src_pml4_phys + hhdm_offset) as *const u64; 
        
        let dest_ptr = (pml4_addr + hhdm_offset) as *mut u64; 
        
        // COPY Kernel half (entries 256-511)
        // Zero user half (0-255)
        core::ptr::copy_nonoverlapping(src_ptr.add(256), dest_ptr.add(256), 256);
        core::ptr::write_bytes(dest_ptr, 0, 256);
    }

    // Iterate Program Headers
    let ph_offset = header.phoff as usize;
    let ph_count = header.phnum as usize;
    let ph_entry_size = header.phentsize as usize;
    
    for i in 0..ph_count {
        let offset = ph_offset + i * ph_entry_size;
        if offset + core::mem::size_of::<ProgramHeader>() > data.len() {
            continue;
        }
        
        let ph = unsafe { &*(data.as_ptr().add(offset) as *const ProgramHeader) };
        
        if ph.p_type == PT_LOAD {
            // Load Segment
            
            // We need to allocate pages for [p_vaddr, p_vaddr + p_memsz]
            let start_vaddr = ph.p_vaddr;
            let end_vaddr = ph.p_vaddr + ph.p_memsz;
            
            let start_page = start_vaddr & !0xFFF;
            let end_page = (end_vaddr + 0xFFF) & !0xFFF;
            let page_count = (end_page - start_page) / 4096;

            for j in 0..page_count {
                let vaddr = start_page + j * 4096;
                // Alloc frame
                let frame = alloc_frame_safe()?;
                let frame_phys = frame * 4096;
                
                // Map to User Table
                unsafe {
                    vmm::map_page_in_pml4(pml4_addr, vaddr, frame_phys, 0x07);
                }
                
                // Copy Data
                let segment_offset = vaddr.saturating_sub(ph.p_vaddr); 
                
                if segment_offset < ph.p_filesz {
                    let copy_len = core::cmp::min(4096, ph.p_filesz - segment_offset);
                    
                    // Destination in HHDM
                    let dest_phys = frame_phys;
                    let hhdm_offset = vmm::VMM_HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
                    let dest_virt = dest_phys + hhdm_offset;
                    
                    let dest_ptr = dest_virt as *mut u8;
                    let src_ptr = unsafe { data.as_ptr().add(offset + segment_offset as usize) };
                    
                    unsafe {
                        core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, copy_len as usize);
                        
                        if copy_len < 4096 {
                             core::ptr::write_bytes(dest_ptr.add(copy_len as usize), 0, 4096 - copy_len as usize);
                        }
                    }
                } else {
                    // BSS Page
                    unsafe {
                        let dest_phys = frame_phys;
                        let hhdm_offset = vmm::VMM_HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
                        let dest_virt = dest_phys + hhdm_offset;
                        core::ptr::write_bytes(dest_virt as *mut u8, 0, 4096);
                    }
                }
            }
        }
    }
    
    // Allocate Stack (fixed at 0x4000_0000)
    let stack_top = 0x40000000;
    let stack_bottom = stack_top - 4096 * 4; // 16KB stack
    
    for i in 0..4 {
         let vaddr = stack_bottom + i * 4096;
         let frame = alloc_frame_safe()?;
         unsafe { vmm::map_page_in_pml4(pml4_addr, vaddr, frame * 4096, 0x07); }
    }
    
    // Create Task
    scheduler::spawn_user(header.entry, stack_top, pml4_addr);

    Ok(0)
}
