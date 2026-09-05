// Mithl OS ELF Loader
// Parses and loads x86_64 ELF executables (like our future self-hosted LLVM)

#[repr(C, packed)]
pub struct ElfHeader {
    pub ident: [u8; 16],
    pub e_type: u16,
    pub machine: u16,
    pub version: u32,
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

#[repr(C, packed)]
pub struct ProgramHeader {
    pub p_type: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

pub const PT_LOAD: u32 = 1;
pub const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

pub fn load_elf(elf_data: &[u8]) -> Result<u64, &'static str> {
    if elf_data.len() < core::mem::size_of::<ElfHeader>() {
        return Err("File too small");
    }

    let header = unsafe { &*(elf_data.as_ptr() as *const ElfHeader) };
    if header.ident[0..4] != ELF_MAGIC {
        return Err("Invalid ELF magic");
    }

    if header.machine != 0x3E { // x86_64
        return Err("Unsupported architecture");
    }

    let ph_offset = header.phoff as usize;
    let ph_size = header.phentsize as usize;
    let ph_num = header.phnum as usize;

    for i in 0..ph_num {
        let ph_ptr = elf_data.as_ptr() as usize + ph_offset + (i * ph_size);
        let ph = unsafe { &*(ph_ptr as *const ProgramHeader) };

        let p_type = ph.p_type;
        if p_type == PT_LOAD {
            let offset = ph.offset;
            let vaddr = ph.vaddr;
            let filesz = ph.filesz;
            let memsz = ph.memsz;
            crate::println!(
                "Load segment: offset {:#x}, vaddr {:#x}, filesz {:#x}, memsz {:#x}",
                offset, vaddr, filesz, memsz
            );
            // TODO: Map pages in user space, copy from elf_data to vaddr, and zero BSS.
        }
    }

    Ok(header.entry)
}
