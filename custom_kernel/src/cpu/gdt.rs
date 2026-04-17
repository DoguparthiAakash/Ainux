use core::arch::{asm, naked_asm};
use core::mem::size_of;

// Segment Selectors
pub const KERNEL_CODE: u16 = 0x08;
pub const KERNEL_DATA: u16 = 0x10;
pub const USER_DATA: u16 = 0x18 | 3;  // Must come BEFORE User Code for SYSRET
pub const USER_CODE: u16 = 0x20 | 3;  // SYSRET loads CS = base+16
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

// Global TSS Instance
pub static mut TSS: Tss = Tss::new();

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

#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

// GDT Table (7 entries: Null, KC, KD, UC, UD, TSS_Low, TSS_High)
static mut GDT: [GdtDescriptor; 7] = [
    // 0: Null
    GdtDescriptor::new(0, 0, 0, 0),
    // 1: Kernel Code
    GdtDescriptor::new(0, 0, 0x9A, 0x20),
    // 2: Kernel Data
    GdtDescriptor::new(0, 0, 0x92, 0x00),
    // 3: User Data (Access 0xF2: Present, Ring 3, Data, Writable) — MUST be before User Code for SYSRET
    GdtDescriptor::new(0, 0, 0xF2, 0x00),
    // 4: User Code (Access 0xFA: Present, Ring 3, Code, Readable)
    GdtDescriptor::new(0, 0, 0xFA, 0x20),
    // 5: TSS Low (will be filled in init)
    GdtDescriptor::new(0, 0, 0, 0),
    // 6: TSS High (will be filled in init)
    GdtDescriptor::new(0, 0, 0, 0),
];

// Helper to write TSS Descriptor (16 bytes)
unsafe fn set_tss_descriptor(index: usize, tss: &'static Tss) {
    let base = tss as *const _ as u64;
    let limit = (size_of::<Tss>() - 1) as u64;
    
    // Low Descriptor (Standard GDT layout)
    // Type 0x89 (Present, Ring 0, System, 64-bit TSS Available)
    // Wait, Type 9 for 64-bit TSS Available? 
    // AMD64 Vol 2: System-Segment Descriptor (Type 9 = Available 64-bit TSS)
    // Access byte: Present(1) | DPL(00) | S(0) | Type(1001) = 10001001 = 0x89?
    // If we want it available.
    
    GDT[index] = GdtDescriptor {
        limit_low: (limit & 0xFFFF) as u16,
        base_low: (base & 0xFFFF) as u16,
        base_middle: ((base >> 16) & 0xFF) as u8,
        access: 0x89, 
        granularity: ((limit >> 16) & 0x0F) as u8, // No granularity flags for TSS usually?
        base_high: ((base >> 24) & 0xFF) as u8,
    };
    
    // High Descriptor (Extension)
    // Base 63:32, Reserved, Zero
    GDT[index + 1] = GdtDescriptor {
        limit_low: (base >> 32) as u16,
        base_low: (base >> 48) as u16,
        base_middle: 0,
        access: 0,
        granularity: 0,
        base_high: 0,
    };
}

pub fn init() {
    unsafe {
        // Setup TSS Descriptor
        set_tss_descriptor(5, &TSS);

        let gdt_ptr = GdtPointer {
            limit: (size_of::<[GdtDescriptor; 7]>() - 1) as u16,
            base: GDT.as_ptr() as u64,
        };

        // Load GDT
        asm!("lgdt [{}]", in(reg) &gdt_ptr, options(nostack));

        // Reload Segments
        load_segments();
        
        // Load Task Register (TSS)
        asm!("ltr ax", in("ax") TSS_SELECTOR, options(nostack, preserves_flags));
    }
}

pub fn set_kernel_stack(stack_top: u64) {
    unsafe {
        TSS.rsp0 = stack_top;
    }
}

#[unsafe(naked)]
unsafe extern "C" fn load_segments() {
    naked_asm!(
        "push 0x08",        // Push code segment
        "lea rax, [rip + 1f]", // Push return address
        "push rax",
        "retfq",            // Far return to reload CS
        "1:",
        "mov ax, 0x10",      // Load data segment
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        "mov ss, ax",
        "ret",
    );
}
