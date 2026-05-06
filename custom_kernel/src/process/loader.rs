use alloc::vec::Vec;
use crate::mm::vmm;
use core::fmt::Write;
use crate::fs::vfs::{self, ArcInode, FileType};

#[repr(C)]
#[derive(Debug, Default)]
pub struct ElfHeader {
    pub magic: [u8; 4],
    pub class: u8,
    pub ord: u8,
    pub version: u8,
    pub os_abi: u8,
    pub abi_version: u8,
    pub pad: [u8; 7],
    pub e_type: u16,
    pub machine: u16,
    pub version2: u32,
    pub entry: u64,
    pub phoff: u64,
    pub shoff: u64,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

#[repr(C)]
#[derive(Debug)]
pub struct ProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

pub fn load_elf(inode: ArcInode, cr3: u64) -> Result<u64, ()> {
    let handle = inode.open(0).map_err(|_| ())?;
    let mut header = ElfHeader::default();
    let header_ptr = &mut header as *mut _ as *mut u8;
    let header_slice = unsafe { core::slice::from_raw_parts_mut(header_ptr, core::mem::size_of::<ElfHeader>()) };
    
    handle.read(header_slice, 0).map_err(|_| ())?;

    if header.magic != [0x7f, b'E', b'L', b'F'] {
        return Err(()); // Not an ELF
    }

    // Parse Program Headers
    for i in 0..header.phnum {
        let mut ph = unsafe { core::mem::zeroed::<ProgramHeader>() };
        let ph_ptr = &mut ph as *mut _ as *mut u8;
        let ph_slice = unsafe { core::slice::from_raw_parts_mut(ph_ptr, core::mem::size_of::<ProgramHeader>()) };
        
        let offset = header.phoff + (i as u64 * header.phentsize as u64);
        handle.read(ph_slice, offset).map_err(|_| ())?;

        if ph.p_type == 1 { // PT_LOAD
             // Map segment in memory
             // For simple functional kernel, we allocate pages and copy data
             let pages = (ph.p_memsz + 4095) / 4096;
              for p in 0..pages {
                 let virt = ph.p_vaddr + (p * 4096);
                 let frame = crate::mm::pmm::PMM.lock().as_mut().unwrap().alloc_frame().unwrap();
                 unsafe {
                     crate::mm::vmm::map_page_in_pml4(cr3, virt, frame, 0x07); // Present, Write, User
                     if p * 4096 < ph.p_filesz {
                         let to_copy = core::cmp::min(4096, ph.p_filesz - (p * 4096));
                         let mut buf = alloc::vec![0u8; 4096];
                         handle.read(&mut buf[..to_copy as usize], ph.p_offset + (p * 4096)).unwrap();
                         
                         // Use HHDM to copy data into the frame
                         let dest_virt = crate::mm::vmm::phys_to_virt(frame);
                         core::ptr::copy_nonoverlapping(buf.as_ptr(), dest_virt as *mut u8, to_copy as usize);
                     }
                 }
             }
        }
    }

    Ok(header.entry)
}

pub fn load_elf_from_file(path: &str) -> Result<usize, ()> {
    if let Ok(inode) = crate::shell::find_inode(path) {
        unsafe {
            let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
            let _ = write!(serial, "[Loader] Found inode for {}\n", path);
        }
        let cr3 = crate::mm::vmm::create_address_space();
        if cr3 == 0 { return Err(()); }
        unsafe {
            let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
            let _ = write!(serial, "[Loader] Address space created: {:#x}\n", cr3);
        }
        
        if let Ok(entry) = load_elf(inode, cr3) {
            unsafe {
                let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
                let _ = write!(serial, "[Loader] ELF segments loaded. Entry: {:#x}\n", entry);
            }
            // Allocate a user stack (1MB)
            let stack_top = 0x00007FFFFFFFF000;
            let stack_pages = 256;
            for p in 0..stack_pages {
                let frame = crate::mm::pmm::PMM.lock().as_mut().unwrap().alloc_frame().unwrap();
                unsafe { crate::mm::vmm::map_page_in_pml4(cr3, stack_top - (p * 4096), frame, 0x07); }
            }
            unsafe {
                let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
                let _ = write!(serial, "[Loader] User stack allocated.\n");
            }
            
            let pid = crate::process::scheduler::spawn_user(entry, stack_top, cr3);
            return Ok(pid);
        } else {
            unsafe {
                let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
                let _ = write!(serial, "loader: load_elf failed for path: {}\n", path);
            }
        }
    } else {
        unsafe {
            let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
            let _ = write!(serial, "loader: find_inode failed for path: {}\n", path);
        }
    }
    Err(())
}

#[repr(C, packed)]
#[derive(Debug, Default, Clone, Copy)]
pub struct AloHeader {
    pub magic: [u8; 4],
    pub entry: u64,
    pub segment_count: u32,
    pub header_size: u32,
}

#[repr(C, packed)]
#[derive(Debug, Default, Clone, Copy)]
pub struct AloSegment {
    pub typ: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub size: u64,
}

pub fn load_alo(inode: ArcInode, cr3: u64) -> Result<u64, ()> {
    let handle = inode.open(0).map_err(|_| ())?;
    let mut header = AloHeader::default();
    let header_ptr = &mut header as *mut _ as *mut u8;
    let header_slice = unsafe { core::slice::from_raw_parts_mut(header_ptr, core::mem::size_of::<AloHeader>()) };
    
    handle.read(header_slice, 0).map_err(|_| ())?;

    if header.magic != *b"ALO\x02" && header.magic != *b"ALO\0" {
        return Err(()); // Not an ALO
    }

    // Parse Segments
    for i in 0..header.segment_count {
        let mut seg = AloSegment::default();
        let seg_ptr = &mut seg as *mut _ as *mut u8;
        let seg_slice = unsafe { core::slice::from_raw_parts_mut(seg_ptr, core::mem::size_of::<AloSegment>()) };
        
        let offset = header.header_size as u64 + (i as u64 * core::mem::size_of::<AloSegment>() as u64);
        handle.read(seg_slice, offset).map_err(|_| ())?;

        if seg.typ == 1 || seg.typ == 2 { // CODE or DATA
             let pages = (seg.size + 4095) / 4096;
             for p in 0..pages {
                 let virt = seg.vaddr + (p * 4096);
                 let frame = crate::mm::pmm::PMM.lock().as_mut().unwrap().alloc_frame().unwrap();
                 unsafe {
                     crate::mm::vmm::map_page_in_pml4(cr3, virt, frame, 0x07); // Present, Write, User
                     if p * 4096 < seg.size {
                         let to_copy = core::cmp::min(4096, seg.size - (p * 4096));
                         let mut buf = alloc::vec![0u8; 4096];
                         handle.read(&mut buf[..to_copy as usize], seg.offset + (p * 4096)).unwrap();
                         
                         // Use HHDM to copy data into the frame
                         let dest_virt = crate::mm::vmm::phys_to_virt(frame);
                         core::ptr::copy_nonoverlapping(buf.as_ptr(), dest_virt as *mut u8, to_copy as usize);
                     }
                 }
             }
        }
    }

    Ok(header.entry)
}

pub fn load_alo_from_file(path: &str) -> Result<usize, ()> {
    if let Ok(inode) = crate::shell::find_inode(path) {
        let cr3 = crate::mm::vmm::create_address_space();
        if cr3 == 0 { return Err(()); }
        
        if let Ok(entry) = load_alo(inode, cr3) {
            // Allocate a user stack (1MB)
            let stack_top = 0x00007FFFFFFFF000;
            let stack_pages = 256;
            for p in 0..stack_pages {
                let frame = crate::mm::pmm::PMM.lock().as_mut().unwrap().alloc_frame().unwrap();
                unsafe { crate::mm::vmm::map_page_in_pml4(cr3, stack_top - (p * 4096), frame, 0x07); }
            }
            
            let pid = crate::process::scheduler::spawn_user(entry, stack_top, cr3);
            return Ok(pid);
        }
    }
    Err(())
}
