use core::arch::{asm, naked_asm};
use core::mem::size_of;
use core::sync::atomic::{AtomicU64, Ordering};

// Segment Selectors
pub const KERNEL_CODE: u16 = 0x08;
pub const KERNEL_DATA: u16 = 0x10;
pub const USER_DATA: u16 = 0x18 | 3;
pub const USER_CODE: u16 = 0x20 | 3;
pub const TSS_SELECTOR: u16 = 0x28;

#[repr(C, packed)]
pub struct Tss {
    reserved1: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    reserved2: u64,
    pub ist1: u64,
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    reserved3: u64,
    reserved4: u16,
    iomap_base: u16,
}

impl Tss {
    pub const fn new() -> Self {
        Self {
            reserved1: 0,
            rsp0: 0, rsp1: 0, rsp2: 0,
            reserved2: 0,
            ist1: 0, ist2: 0, ist3: 0, ist4: 0, ist5: 0, ist6: 0, ist7: 0,
            reserved3: 0,
            reserved4: 0,
            iomap_base: 104, // Size of TSS
        }
    }
}

// IST stack sizes (8KB each — enough for fault handlers)
const IST_STACK_SIZE: usize = 8192;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GdtDescriptor {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtDescriptor {
    pub const fn new(base: u32, limit: u32, access: u8, flags: u8) -> Self {
        Self {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            access,
            granularity: ((limit >> 16) & 0x0F) as u8 | (flags & 0xF0),
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }
}

// Per-CPU state to ensure SMP safety
struct CpuState {
    gdt: [GdtDescriptor; 7],
    tss: Tss,
    ist1: [u8; IST_STACK_SIZE],
}

static mut CPU_STATES: [CpuState; crate::cpu::percpu::MAX_CPUS] = [const { 
    CpuState {
        gdt: [
            GdtDescriptor::new(0, 0, 0, 0),
            GdtDescriptor::new(0, 0xFFFFFFFF, 0x9B, 0xA0), // 1: Kernel Code (G=1, L=1)
            GdtDescriptor::new(0, 0xFFFFFFFF, 0x93, 0x80), // 2: Kernel Data (G=1)
            GdtDescriptor::new(0, 0xFFFFFFFF, 0xF3, 0x80), // 3: User Data 64 (G=1)
            GdtDescriptor::new(0, 0xFFFFFFFF, 0xFB, 0xA0), // 4: User Code 64 (G=1, L=1)
            GdtDescriptor::new(0, 0, 0, 0),
            GdtDescriptor::new(0, 0, 0, 0),
        ],
        tss: Tss::new(),
        ist1: [0; IST_STACK_SIZE],
    }
}; crate::cpu::percpu::MAX_CPUS];

#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

// Helper to write TSS Descriptor (16 bytes)
unsafe fn set_tss_descriptor(gdt: &mut [GdtDescriptor; 7], index: usize, tss: &Tss) {
    let base = tss as *const _ as u64;
    let limit = (size_of::<Tss>() - 1) as u64;
    
    gdt[index] = GdtDescriptor {
        limit_low: (limit & 0xFFFF) as u16,
        base_low: (base & 0xFFFF) as u16,
        base_middle: ((base >> 16) & 0xFF) as u8,
        access: 0x89, 
        granularity: ((limit >> 16) & 0x0F) as u8,
        base_high: ((base >> 24) & 0xFF) as u8,
    };
    
    gdt[index + 1] = GdtDescriptor {
        limit_low: (base >> 32) as u16,
        base_low: (base >> 48) as u16,
        base_middle: 0,
        access: 0,
        granularity: 0,
        base_high: 0,
    };
}

pub unsafe fn init_ap(cpu_id: usize) {
    let state = &mut CPU_STATES[cpu_id];
        
        // Setup IST stacks in TSS for fault isolation
        let ist1_top = state.ist1.as_ptr() as u64 + IST_STACK_SIZE as u64;
        state.tss.ist1 = ist1_top;

        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        use core::fmt::Write;
        let _ = write!(serial, "GDT: CPU {} IST1_TOP: {:#x}\n", cpu_id, ist1_top);

        // Setup TSS Descriptor in per-CPU GDT at index 5
        set_tss_descriptor(&mut state.gdt, 5, &state.tss);

        let gdt_ptr = GdtPointer {
            limit: (size_of::<[GdtDescriptor; 7]>() - 1) as u16,
            base: state.gdt.as_ptr() as u64,
        };

        asm!("lgdt [{}]", in(reg) &gdt_ptr, options(nostack));
        
        // Load Segment Registers
        asm!(
            "push {0}",
            "lea {1}, [rip + 2f]",
            "push {1}",
            "retfq",
            "2:",
            "mov ds, {2:x}",
            "mov es, {2:x}",
            "mov fs, {2:x}",
            "mov gs, {2:x}",
            "mov ss, {2:x}",
            in(reg) KERNEL_CODE as u64,
            out(reg) _,
            in(reg) KERNEL_DATA as u16,
            options(preserves_flags)
        );

        // Load TSS
        asm!("ltr {0:x}", in(reg) TSS_SELECTOR, options(nostack, preserves_flags));

        // CRITICAL: We must set GS_BASE AFTER loading the GS segment register.
        // Loading the GS selector (mov gs, ax) reloads the base from the GDT,
        // which clears any base set by WRMSR.
        let prcb_addr = crate::cpu::percpu::get_prcb_addr(cpu_id);
        crate::cpu::percpu::CPUS[cpu_id].init(prcb_addr, cpu_id as u32);
        crate::cpu::percpu::write_gs_base(prcb_addr);
        crate::cpu::percpu::write_kernel_gs_base(prcb_addr);
}

pub unsafe fn init() {
    init_ap(0);
}

pub fn load_segments() {
    // This is now integrated into init() via retfq
}

pub fn set_kernel_stack(stack_top: u64) {
    let cpu_id = crate::cpu::percpu::get_current_cpu_id();
    unsafe {
        CPU_STATES[cpu_id].tss.rsp0 = stack_top;
    }
}
