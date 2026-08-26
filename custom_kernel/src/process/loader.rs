use alloc::vec::Vec;
use crate::mm::vmm;
use crate::fs::vfs::{self, ArcInode, FileType};

/// Translate a user virtual address to a kernel-writable pointer via page table walk + HHDM.
unsafe fn user_virt_to_hhdm_ptr(cr3: u64, vaddr: u64) -> Option<*mut u8> {
    let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
    let pml4 = core::slice::from_raw_parts((cr3 + hhdm) as *const u64, 512);
    let p4i = ((vaddr >> 39) & 0x1FF) as usize;
    if pml4[p4i] & 1 == 0 { return None; }
    let pdpt = core::slice::from_raw_parts(((pml4[p4i] & 0x000FFFFFFFFFF000) + hhdm) as *const u64, 512);
    let p3i = ((vaddr >> 30) & 0x1FF) as usize;
    if pdpt[p3i] & 1 == 0 { return None; }
    let pd = core::slice::from_raw_parts(((pdpt[p3i] & 0x000FFFFFFFFFF000) + hhdm) as *const u64, 512);
    let p2i = ((vaddr >> 21) & 0x1FF) as usize;
    if pd[p2i] & 1 == 0 { return None; }
    let pt = core::slice::from_raw_parts(((pd[p2i] & 0x000FFFFFFFFFF000) + hhdm) as *const u64, 512);
    let p1i = ((vaddr >> 12) & 0x1FF) as usize;
    if pt[p1i] & 1 == 0 { return None; }
    let frame = pt[p1i] & 0x000FFFFFFFFFF000;
    let offset = vaddr & 0xFFF;
    Some((frame + hhdm + offset) as *mut u8)
}

/// Write a u64 to a user virtual address via HHDM.
unsafe fn write_user_u64(cr3: u64, vaddr: u64, val: u64) {
    if let Some(ptr) = user_virt_to_hhdm_ptr(cr3, vaddr) {
        (ptr as *mut u64).write_unaligned(val);
    }
}

/// Set up the initial user stack per the System V AMD64 ABI.
/// Returns the initial RSP value (user virtual address).
///
/// Stack layout (growing downward from stack_top):
///   AT_NULL (0, 0)      -- end of auxv
///   AT_ENTRY (entry)    -- auxv
///   AT_PAGESZ (4096)    -- auxv  
///   NULL                -- end of envp
///   NULL                -- end of argv
///   argv[0] ptr         -- points to program name string
///   argc = 1
///   [program name string "ainux" at a known location above]
pub fn setup_user_stack(cr3: u64, stack_top: u64, entry: u64, args: &[alloc::string::String]) -> u64 {
    // We'll build the stack from the top, pushing downward.
    // First, write the strings near the top.
    let mut current_string_addr = stack_top;
    
    // We need to store the pointers to each string
    let mut argv_ptrs = alloc::vec::Vec::new();
    
    for arg in args.iter().rev() {
        let bytes = arg.as_bytes();
        current_string_addr -= (bytes.len() + 1) as u64;
        unsafe {
            for (i, &byte) in bytes.iter().enumerate() {
                if let Some(ptr) = user_virt_to_hhdm_ptr(cr3, current_string_addr + i as u64) {
                    *ptr = byte;
                }
            }
            // Null terminator
            if let Some(ptr) = user_virt_to_hhdm_ptr(cr3, current_string_addr + bytes.len() as u64) {
                *ptr = 0;
            }
        }
        argv_ptrs.push(current_string_addr);
    }
    // Reverse again so they are in original order
    argv_ptrs.reverse();

    // Now build the stack frame below the string area.
    let mut sp = current_string_addr;
    sp &= !0xF; // 16-byte align
    
    // Calculate total size of pointers and headers
    // argc + argv array + NULL + NULL (envp) + AT_PAGESZ + val + AT_ENTRY + val + AT_NULL + val
    let ptrs_count = 1 + argv_ptrs.len() + 1 + 1 + 2 + 2 + 2;
    sp -= (ptrs_count * 8) as u64;
    sp &= !0xF; // Re-align

    unsafe {
        let mut cur = sp;
        write_user_u64(cr3, cur, args.len() as u64); // argc
        cur += 8;
        
        for &ptr in &argv_ptrs {
            write_user_u64(cr3, cur, ptr); // argv[i]
            cur += 8;
        }
        
        write_user_u64(cr3, cur, 0); // argv terminator
        cur += 8;
        
        write_user_u64(cr3, cur, 0); // envp terminator
        cur += 8;
        
        write_user_u64(cr3, cur, 6); // AT_PAGESZ
        cur += 8;
        write_user_u64(cr3, cur, 4096); // page size value
        cur += 8;
        
        write_user_u64(cr3, cur, 9); // AT_ENTRY
        cur += 8;
        write_user_u64(cr3, cur, entry); // entry point
        cur += 8;
        
        write_user_u64(cr3, cur, 0); // AT_NULL
        cur += 8;
        write_user_u64(cr3, cur, 0); // AT_NULL value
    }

    sp
}
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
    let handle = match inode.open(0) {
        Ok(h) => h,
        Err(_) => {
            {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "ELF LOAD ERROR: open failed\n");
}
            return Err(());
        }
    };
    
    let mut header = ElfHeader::default();
    let header_ptr = &mut header as *mut _ as *mut u8;
    let header_slice = unsafe { core::slice::from_raw_parts_mut(header_ptr, core::mem::size_of::<ElfHeader>()) };
    
    if let Err(_) = handle.read(header_slice, 0) {
        crate::drivers::video::put_str("ELF LOAD ERROR: read header failed\n");
        return Err(());
    }

    if header.magic != [0x7f, b'E', b'L', b'F'] {
        crate::drivers::video::put_str("ELF LOAD ERROR: invalid magic\n");
        return Err(()); // Not an ELF
    }

    let mut load_bias = 0u64;
    if header.e_type == 3 { // ET_DYN (PIE)
        let rand = crate::lib::prng::get_random_u64();
        load_bias = 0x100000000 + (rand & 0x3FFFFFFFF000);
    }

    // Parse Program Headers
    for i in 0..header.phnum {
        let mut ph = unsafe { core::mem::zeroed::<ProgramHeader>() };
        let ph_ptr = &mut ph as *mut _ as *mut u8;
        let ph_slice = unsafe { core::slice::from_raw_parts_mut(ph_ptr, core::mem::size_of::<ProgramHeader>()) };
        
        let offset = header.phoff + (i as u64 * header.phentsize as u64);
        if let Err(_) = handle.read(ph_slice, offset) {
            {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "ELF LOAD ERROR: read program header failed\n");
}
            return Err(());
        }

        if ph.p_type == 1 { // PT_LOAD
             // Map segment in memory
             let memsz = ph.p_memsz;
             let vaddr = ph.p_vaddr + load_bias;
             let offset_in_page = vaddr & 0xFFF;
             let pages = (memsz + offset_in_page + 4095) / 4096;
             
             let mut page_flags = 0x05; // Present, User
             if ph.p_flags & 2 != 0 {
                 page_flags |= 0x02; // Writable
             }
             
             for p in 0..pages {
                 let virt = (vaddr & !0xFFF) + (p * 4096);
                 let frame = crate::mm::pmm::PMM.lock().as_mut().unwrap().alloc_frame().unwrap();
                 unsafe {
                     crate::mm::vmm::map_page_in_pml4(cr3, virt, frame, page_flags);
                     let hhdm_offset = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
                     let phys_ptr = (frame + hhdm_offset) as *mut u8;
                     core::ptr::write_bytes(phys_ptr, 0, 4096);
                     
                     // Compute how many bytes from the file should go into this page
                     // For the first page, we start at `offset_in_page`.
                     // For subsequent pages, we start at 0.
                     let dest_offset = if p == 0 { offset_in_page } else { 0 };
                     let mut file_offset_for_page = ph.p_offset;
                     if p > 0 {
                         file_offset_for_page += (p * 4096) - offset_in_page;
                     }
                     
                     // Determine how many bytes we can actually read from the file for this page
                     let mut bytes_to_read = 4096 - dest_offset;
                     
                     // Make sure we don't read past the end of the file segment
                     if file_offset_for_page >= ph.p_offset + ph.p_filesz {
                         bytes_to_read = 0;
                     } else if file_offset_for_page + bytes_to_read > ph.p_offset + ph.p_filesz {
                         bytes_to_read = (ph.p_offset + ph.p_filesz) - file_offset_for_page;
                     }
                     
                     if bytes_to_read > 0 {
                         let mut bytes_read = 0;
                         while bytes_read < bytes_to_read {
                             let mut chunk = alloc::vec![0u8; (bytes_to_read - bytes_read) as usize];
                             match handle.read(&mut chunk, file_offset_for_page + bytes_read) {
                                 Ok(n) if n > 0 => {
                                     core::ptr::copy_nonoverlapping(
                                         chunk.as_ptr(),
                                         phys_ptr.add(dest_offset as usize + bytes_read as usize),
                                         n
                                     );
                                     bytes_read += n as u64;
                                 },
                                 Ok(_) => break, // EOF reached early
                                 Err(_) => {
                                     crate::drivers::video::put_str("ELF LOAD ERROR: IOError on read!\n");
                                     return Err(());
                                 }
                             }
                         }
                     }
                 }
             }
        }
    }

    Ok(header.entry + load_bias)
}

pub fn exec_elf(path: &str, state: *mut crate::process::scheduler::SyscallState) -> Result<(), ()> {
    match crate::shell::find_inode(path) {
        Ok(inode) => {
            let address_space = alloc::sync::Arc::new(spin::Mutex::new(crate::mm::address_space::AddressSpace::new_user()));
            let cr3 = address_space.lock().pml4_phys;
            if cr3 == 0 { return Err(()); }
            
            match load_elf(inode, cr3) {
                Ok(entry) => {
                    let stack_base = 0x00007FFFFFFFE000u64;
                    let stack_pages = 256u64;
                    for p in 0..stack_pages {
                        let frame = crate::mm::pmm::PMM.lock().as_mut().unwrap().alloc_frame().unwrap();
                        unsafe { crate::mm::vmm::map_page_in_pml4(cr3, stack_base - (p * 4096), frame, 0x07); }
                    }
                    
                    let stack_top = stack_base + 4096;
                    let args = alloc::vec![alloc::string::String::from(path)];
                    let initial_sp = setup_user_stack(cr3, stack_top, entry, &args);
                    
                    unsafe {
                        (*state).rcx = entry;
                        (*state).user_rsp = initial_sp;
                    }
                    
                    let pid = crate::process::scheduler::get_current_pid();
                    crate::cpu::without_interrupts(|| {
                        let mut tasks = crate::process::scheduler::TASKS.lock();
                        if let Some(task) = &mut tasks[pid] {
                            task.cr3 = cr3;
                            task.address_space = Some(address_space);
                            unsafe {
                                core::arch::asm!("mov cr3, {}", in(reg) cr3);
                            }
                        }
                    });
                    
                    Ok(())
                },
                Err(_) => Err(())
            }
        },
        Err(_) => Err(())
    }
}

pub fn load_elf_from_file(path: &str) -> Result<usize, ()> {
    match crate::shell::find_inode(path) {
        Ok(inode) => {
            let address_space = alloc::sync::Arc::new(spin::Mutex::new(crate::mm::address_space::AddressSpace::new_user()));
            let cr3 = address_space.lock().pml4_phys;
            if cr3 == 0 { 
                {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "ELF LOAD ERROR: create_address_space failed\n");
}
                return Err(()); 
            }
            
            match load_elf(inode, cr3) {
                Ok(entry) => {
                    {
                        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
                        use core::fmt::Write;
                        let _ = write!(serial, "DEBUG load_elf returned entry: {:#x}\n", entry);
                    }
                    // Allocate a user stack (1MB)
                    let stack_base = 0x00007FFFFFFFE000u64; // top page
                    let stack_pages = 256u64;
                    for p in 0..stack_pages {
                        let frame = crate::mm::pmm::PMM.lock().as_mut().unwrap().alloc_frame().unwrap();
                        unsafe { crate::mm::vmm::map_page_in_pml4(cr3, stack_base - (p * 4096), frame, 0x07); }
                    }
                    
                    // Build the initial user stack (System V ABI)
                    // musl _start expects: [argc, argv[0], NULL, envp NULL, auxv AT_NULL]
                    // We write from the top of the stack downward using HHDM.
                    let stack_top = stack_base + 4096; // 0x7FFFFFFFFFFF000 + 0x1000
                    let args = alloc::vec![alloc::string::String::from(path)];
                    let initial_sp = setup_user_stack(cr3, stack_top, entry, &args);
                    
                    let pid = crate::process::scheduler::spawn_user(entry, initial_sp, address_space, path);
                    return Ok(pid);
                },
                Err(_) => {
                    {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "ELF LOAD ERROR: load_elf returned Err(())\n");
}
                    return Err(());
                }
            }
        },
        Err(_) => {
            {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "ELF LOAD ERROR: find_inode failed\n");
}
            return Err(());
        }
    }
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
                     let hhdm_offset = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
                     let phys_ptr = (frame + hhdm_offset) as *mut u8;
                     core::ptr::write_bytes(phys_ptr, 0, 4096);
                     if p * 4096 < seg.size {
                         let to_copy = core::cmp::min(4096, seg.size - (p * 4096));
                         let mut buf = alloc::vec![0u8; 4096];
                         handle.read(&mut buf[..to_copy as usize], seg.offset + (p * 4096)).unwrap();
                         core::ptr::copy_nonoverlapping(buf.as_ptr(), phys_ptr, to_copy as usize);
                     }
                 }
             }
        }
    }

    Ok(header.entry)
}

pub fn load_alo_from_file(path: &str) -> Result<usize, ()> {
    if let Ok(inode) = crate::shell::find_inode(path) {
        let address_space = alloc::sync::Arc::new(spin::Mutex::new(crate::mm::address_space::AddressSpace::new_user()));
        let cr3 = address_space.lock().pml4_phys;
        if cr3 == 0 { return Err(()); }
        
        if let Ok(entry) = load_alo(inode, cr3) {
            // Allocate a user stack (1MB)
            let stack_top = 0x00007FFFFFFFF000;
            let stack_pages = 256;
            for p in 0..stack_pages {
                let frame = crate::mm::pmm::PMM.lock().as_mut().unwrap().alloc_frame().unwrap();
                unsafe { crate::mm::vmm::map_page_in_pml4(cr3, stack_top - (p * 4096), frame, 0x07); }
            }
            
            let pid = crate::process::scheduler::spawn_user(entry, stack_top, address_space, path);
            return Ok(pid);
        }
    }
    Err(())
}
