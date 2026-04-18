#![allow(unused_imports)]
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
    InvalidAlo,
    UnsupportedArch,
    MemoryError,
    SegmentError,
}

#[repr(C, packed)]
pub struct AloHeader {
    pub magic: [u8; 4],
    pub entry: u64,
    pub segment_count: u32,
    pub header_size: u32,
}

#[repr(C, packed)]
pub struct AloSegment {
    pub typ: u32,   // 1=CODE, 2=DATA, 3=BSS
    pub flags: u32, // 1=R, 2=W, 4=X
    pub offset: u64,
    pub vaddr: u64,
    pub size: u64,
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
    let pml4_addr = pml4_frame; // Physical
    
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
                let frame_phys = alloc_frame_safe()?;
                
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
                    
                    // DEBUG: Unconditional
                    unsafe {
                       crate::drivers::video::put_str("ELF Load: Phys ");
                       if frame_phys < 0x100000000 {
                           let mb = frame_phys / 1024 / 1024;
                           let d1 = (mb / 10) as u8;
                           let d2 = (mb % 10) as u8;
                           crate::drivers::video::put_char((b'0' + d1) as char);
                           crate::drivers::video::put_char((b'0' + d2) as char);
                           crate::drivers::video::put_str("MB ");
                       }
                       crate::drivers::video::put_str("\n");
                    }
                    
                    if frame_phys > 0x80000000 { // 2GB limit safety
                         crate::drivers::video::put_str("BAD FRAME\n");
                         return Err(LoadError::MemoryError);
                    }
                    
                    unsafe {
                        core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, copy_len as usize);
                        
                        if copy_len < 4096 {
                             core::ptr::write_bytes(dest_ptr.add(copy_len as usize), 0, 4096 - copy_len as usize);
                        }
                    }
                    
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
         let frame_phys = alloc_frame_safe()?;
         unsafe { vmm::map_page_in_pml4(pml4_addr, vaddr, frame_phys, 0x07); }
    }
    
    // Create Task
    scheduler::spawn_user(header.entry, stack_top, pml4_addr);

    // Return PID
    let tasks = scheduler::TASKS.lock();
    for i in 0..scheduler::MAX_TASKS {
        if let Some(ref t) = tasks[i] {
            if t.cr3 == pml4_addr {
                return Ok(t.id);
            }
        }
    }
    
    Ok(0)
}

pub fn load_elf_from_file(path: &str) -> Result<usize, LoadError> {
    let root = crate::fs::vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
        if let Ok(inode) = root_inode.lookup(path) {
            if let Ok(handle) = inode.open(0) {
                let mut buf = alloc::vec![0u8; 1024 * 1024]; 
                if let Ok(n) = handle.read(&mut buf, 0) {
                    buf.truncate(n);
                    return load_elf(&buf);
                }
            }
        }
    }
    Err(LoadError::InvalidElf)
}

pub fn load_alo(data: &[u8]) -> Result<usize, LoadError> {
    if data.len() < core::mem::size_of::<AloHeader>() {
        return Err(LoadError::InvalidAlo);
    }
    
    let header = unsafe { &*(data.as_ptr() as *const AloHeader) };
    if &header.magic != b"ALO\x02" {
        return Err(LoadError::InvalidAlo);
    }
    
    // Create Valid User Page Table
    let pml4_frame = alloc_frame_safe()?;
    let pml4_addr = pml4_frame;
    
    unsafe {
        let current_cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) current_cr3);
        let hhdm_offset = vmm::VMM_HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let src_pml4_phys = current_cr3 & 0x000FFFFFFFFFF000;
        let src_ptr = (src_pml4_phys + hhdm_offset) as *const u64; 
        let dest_ptr = (pml4_addr + hhdm_offset) as *mut u64; 
        core::ptr::copy_nonoverlapping(src_ptr.add(256), dest_ptr.add(256), 256);
        core::ptr::write_bytes(dest_ptr, 0, 256);
    }

    // Load Segments
    let seg_size = core::mem::size_of::<AloSegment>();
    let headers_total_size = header.header_size as usize + (header.segment_count as usize * seg_size);
    
    if data.len() < headers_total_size {
        return Err(LoadError::InvalidAlo);
    }

    for i in 0..header.segment_count {
        let seg_offset = header.header_size as usize + (i as usize * seg_size);
        let seg = unsafe { &*(data.as_ptr().add(seg_offset) as *const AloSegment) };
        
        // 1=CODE, 2=DATA
        if seg.typ == 1 || seg.typ == 2 {
            let start_page = seg.vaddr & !0xFFF;
            let end_page = (seg.vaddr + seg.size + 0xFFF) & !0xFFF;
            let num_pages = (end_page - start_page) / 4096;
            
            // Map Flags:
            // segment flags: 1=R, 2=W, 4=X
            // x86_64 PTE flags: bit 0=Present, 1=Writable, 2=User, 63=NX (No-Execute)
            let mut pte_flags: u64 = 0x05; // Present | User
            if (seg.flags & 2) != 0 { pte_flags |= 0x02; } // Writable
            if (seg.flags & 4) == 0 { pte_flags |= 1u64 << 63; } // Set NX if NOT executable
            
            for p in 0..num_pages {
                let vaddr = start_page + (p * 4096);
                let frame_phys = alloc_frame_safe()?;
                unsafe { 
                    vmm::map_page_in_pml4(pml4_addr, vaddr, frame_phys, pte_flags); 
                    
                    let hhdm_offset = vmm::VMM_HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
                    let dest_virt = frame_phys + hhdm_offset;
                    
                    // Copy data if within segment file bounds
                    let offset_in_seg = (vaddr.saturating_sub(seg.vaddr)) as usize;
                    if (seg.offset as usize + offset_in_seg) < data.len() {
                        let remaining_in_seg = seg.size as usize - offset_in_seg;
                        let copy_len = core::cmp::min(4096, remaining_in_seg);
                        
                        core::ptr::copy_nonoverlapping(
                            data.as_ptr().add(seg.offset as usize + offset_in_seg),
                            dest_virt as *mut u8,
                            copy_len
                        );
                        
                        // Zero out rest of page
                        if copy_len < 4096 {
                            core::ptr::write_bytes((dest_virt + copy_len as u64) as *mut u8, 0, 4096 - copy_len);
                        }
                    } else {
                        // Beyond file range (BSS/Zero init)
                        core::ptr::write_bytes(dest_virt as *mut u8, 0, 4096);
                    }
                }
            }
        }
    }

    // Setup Stack (0x4000_0000)
    let stack_top = 0x40000000;
    let stack_bottom = stack_top - 4096 * 4;
    for i in 0..4 {
        let vaddr = stack_bottom + i * 4096;
        let frame_phys = alloc_frame_safe()?;
        unsafe { vmm::map_page_in_pml4(pml4_addr, vaddr as u64, frame_phys, 0x07); }
    }

    // Spawn
    scheduler::spawn_user(header.entry, stack_top as u64, pml4_addr);

    // Return PID
    let tasks = scheduler::TASKS.lock();
    for i in 0..scheduler::MAX_TASKS {
        if let Some(ref t) = tasks[i] {
            if t.cr3 == pml4_addr {
                return Ok(t.id);
            }
        }
    }
    Ok(0)
}

pub fn load_alo_from_file(path: &str) -> Result<usize, LoadError> {
    let root = crate::fs::vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
        if let Ok(inode) = root_inode.lookup(path) {
            if let Ok(handle) = inode.open(0) {
                let mut buf = alloc::vec![0u8; 1024 * 1024]; 
                if let Ok(n) = handle.read(&mut buf, 0) {
                    buf.truncate(n);
                    return load_alo(&buf);
                }
            }
        }
    }
    Err(LoadError::InvalidAlo)
}
