use core::sync::atomic::{AtomicUsize, Ordering};

#[repr(C, align(64))]
pub struct PRCB {
    /// Self-pointer for GS:0 access
    pub self_ptr: u64,
    pub cpu_id: u32,
    pub lapic_id: u32,
    pub current_pid: usize,
    pub stack_top: u64,
    pub total_ticks: u64,
}

impl PRCB {
    pub const fn new(id: u32) -> Self {
        Self {
            self_ptr: 0,
            cpu_id: id,
            lapic_id: 0,
            current_pid: 0,
            stack_top: 0,
            total_ticks: 0,
        }
    }

    pub fn init(&mut self, addr: u64) {
        self.self_ptr = addr;
    }
}

pub static CPU_COUNT: AtomicUsize = AtomicUsize::new(0);

// For fixed-size array of PRCBs (one per core)
pub const MAX_CPUS: usize = 32;
pub static mut CPUS: [PRCB; MAX_CPUS] = [const { PRCB::new(0) }; MAX_CPUS];

pub fn get_cpu_count() -> usize {
    CPU_COUNT.load(Ordering::Relaxed)
}

pub fn get_prcb_addr(id: usize) -> u64 {
    unsafe { &CPUS[id] as *const PRCB as u64 }
}

pub fn get_current_cpu_id() -> usize {
    let id: u32;
    unsafe {
        core::arch::asm!("mov {:e}, gs:[8]", out(reg) id, options(nostack, nomem, preserves_flags));
    }
    id as usize
}

/// Sets the GS_BASE MSR to point to this CPU's PRCB
pub unsafe fn write_gs_base(addr: u64) {
    let low = (addr & 0xFFFFFFFF) as u32;
    let high = (addr >> 32) as u32;
    // MSR_GS_BASE = 0xC0000101
    core::arch::asm!(
        "wrmsr",
        in("rcx") 0xC0000101u32,
        in("rax") low,
        in("rdx") high,
        options(nostack, preserves_flags)
    );
}
