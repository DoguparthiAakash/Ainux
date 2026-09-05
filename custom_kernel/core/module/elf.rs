use alloc::alloc::{alloc, Layout};
use core::ptr::copy_nonoverlapping;

#[repr(C)]
struct Elf64_Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
struct Elf64_Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

const PT_LOAD: u32 = 1;

pub fn load_elf(data: &[u8]) -> Result<(usize, usize), &'static str> {
    if data.len() < core::mem::size_of::<Elf64_Ehdr>() {
        return Err("ELF file too small");
    }

    let ehdr = unsafe { &*(data.as_ptr() as *const Elf64_Ehdr) };

    if ehdr.e_ident[0..4] != [0x7f, b'E', b'L', b'F'] {
        return Err("Invalid ELF magic");
    }

    let phoff = ehdr.e_phoff as usize;
    let phnum = ehdr.e_phnum as usize;
    let phentsize = ehdr.e_phentsize as usize;

    let mut mem_sz = 0;
    for i in 0..phnum {
        let phdr_offset = phoff + i * phentsize;
        let phdr = unsafe { &*(data.as_ptr().add(phdr_offset) as *const Elf64_Phdr) };
        if phdr.p_type == PT_LOAD {
            let end = phdr.p_vaddr + phdr.p_memsz;
            if end > mem_sz {
                mem_sz = end;
            }
        }
    }

    let mem_sz_usize = mem_sz as usize;
    if mem_sz_usize == 0 {
        return Err("No PT_LOAD segments");
    }

    // Allocate memory for the module
    let layout = Layout::from_size_align(mem_sz_usize, 4096).map_err(|_| "Invalid layout")?;
    let base_addr = unsafe { alloc(layout) };
    if base_addr.is_null() {
        return Err("Memory allocation failed");
    }

    // Load segments
    for i in 0..phnum {
        let phdr_offset = phoff + i * phentsize;
        let phdr = unsafe { &*(data.as_ptr().add(phdr_offset) as *const Elf64_Phdr) };
        if phdr.p_type == PT_LOAD {
            let dest = unsafe { base_addr.add(phdr.p_vaddr as usize) };
            let src = unsafe { data.as_ptr().add(phdr.p_offset as usize) };
            let filesz = phdr.p_filesz as usize;
            let memsz = phdr.p_memsz as usize;

            unsafe {
                copy_nonoverlapping(src, dest, filesz);
                // Zero out BSS
                if memsz > filesz {
                    core::ptr::write_bytes(dest.add(filesz), 0, memsz - filesz);
                }
            }
        }
    }

    // Optional: Call entry point (e_entry)
    // let entry_ptr = unsafe { base_addr.add(ehdr.e_entry as usize) };
    // let init_fn: extern "C" fn() = unsafe { core::mem::transmute(entry_ptr) };
    // init_fn();

    Ok((base_addr as usize, mem_sz_usize))
}
