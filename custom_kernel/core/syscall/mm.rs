use core::sync::atomic::{AtomicUsize, Ordering};
use crate::mm::pmm::{PAGE_SIZE, PMM, HHDM_OFFSET};
use crate::mm::vmm::{map_page, unmap_page, PRESENT, WRITABLE, USER};

// A bump allocator for anonymous mmap
static NEXT_MMAP_ADDR: AtomicUsize = AtomicUsize::new(0x4000_0000_0000);
// A bump allocator for brk
static BRK_CURRENT: AtomicUsize = AtomicUsize::new(0x6000_0000_0000);

pub const MAP_ANONYMOUS: i32 = 0x20;

pub fn sys_mmap(
    addr: usize,
    len: usize,
    prot: i32,
    flags: i32,
    fd: usize,
    offset: usize,
) -> isize {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "sys_mmap: addr={:#x}, len={}, prot={}, flags={:#x}, fd={}\n", addr, len, prot, flags, fd);

    if flags & MAP_ANONYMOUS != 0 {
        let mut vaddr = addr;
        let aligned_len = (len + PAGE_SIZE as usize - 1) & !(PAGE_SIZE as usize - 1);
        
        if vaddr == 0 {
            vaddr = NEXT_MMAP_ADDR.fetch_add(aligned_len, Ordering::SeqCst);
        }
        
        let pages = aligned_len / PAGE_SIZE as usize;
        let mut pmm_lock = PMM.lock();
        
        for i in 0..pages {
            let phys = if let Some(ref mut pmm) = *pmm_lock {
                pmm.alloc_frame().unwrap_or(0)
            } else {
                return -12; // ENOMEM
            };
            
            if phys == 0 { return -12; }
            
            unsafe {
                let hhdm = HHDM_OFFSET.load(Ordering::Relaxed);
                core::ptr::write_bytes((phys + hhdm) as *mut u8, 0, PAGE_SIZE as usize);
                let _ = map_page((vaddr + i * PAGE_SIZE as usize) as u64, phys, USER | PRESENT | WRITABLE);
            }
        }
        
        return vaddr as isize;
    }
    
    -38 // ENOSYS for non-anonymous mmap
}

pub fn sys_munmap(addr: usize, len: usize) -> isize {
    let aligned_len = (len + PAGE_SIZE as usize - 1) & !(PAGE_SIZE as usize - 1);
    let pages = aligned_len / PAGE_SIZE as usize;
    for i in 0..pages {
        unsafe { unmap_page((addr + i * PAGE_SIZE as usize) as u64); }
    }
    0
}

pub fn sys_brk(addr: usize) -> isize {
    let current = BRK_CURRENT.load(Ordering::SeqCst);
    if addr == 0 {
        return current as isize;
    }
    
    if addr > current {
        let diff = addr - current;
        let pages = (diff + PAGE_SIZE as usize - 1) / PAGE_SIZE as usize;
        let mut pmm_lock = PMM.lock();
        for i in 0..pages {
            let phys = if let Some(ref mut pmm) = *pmm_lock {
                pmm.alloc_frame().unwrap_or(0)
            } else {
                return -12;
            };
            
            if phys == 0 { return -12; }
            
            unsafe {
                let hhdm = HHDM_OFFSET.load(Ordering::Relaxed);
                core::ptr::write_bytes((phys + hhdm) as *mut u8, 0, PAGE_SIZE as usize);
                let _ = map_page((current + i * PAGE_SIZE as usize) as u64, phys, USER | PRESENT | WRITABLE);
            }
        }
    }
    
    BRK_CURRENT.store(addr, Ordering::SeqCst);
    addr as isize
}

pub const ARCH_SET_GS: i32 = 0x1001;
pub const ARCH_SET_FS: i32 = 0x1002;

pub fn sys_arch_prctl(code: i32, addr: usize) -> isize {
    match code {
        ARCH_SET_FS => {
            unsafe {
                core::arch::asm!("wrmsr", in("ecx") 0xC0000100_u32, in("eax") (addr & 0xFFFFFFFF) as u32, in("edx") (addr >> 32) as u32);
            }
            0
        }
        ARCH_SET_GS => {
            unsafe {
                core::arch::asm!("wrmsr", in("ecx") 0xC0000101_u32, in("eax") (addr & 0xFFFFFFFF) as u32, in("edx") (addr >> 32) as u32);
            }
            0
        }
        _ => -22 // EINVAL
    }
}
