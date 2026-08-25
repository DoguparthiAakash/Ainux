pub mod elf;
pub mod driver;

use alloc::vec::Vec;
use spin::Mutex;
use alloc::string::String;

pub struct LoadedModule {
    pub name: String,
    pub base_addr: usize,
    pub size: usize,
}

pub static MODULES: Mutex<Vec<LoadedModule>> = Mutex::new(Vec::new());

pub fn load_module(name: &str, elf_data: &[u8]) -> Result<usize, &'static str> {
    let (base_addr, size) = elf::load_elf(elf_data)?;
    
    let mut mods = MODULES.lock();
    mods.push(LoadedModule {
        name: String::from(name),
        base_addr,
        size,
    });
    
    Ok(base_addr)
}
