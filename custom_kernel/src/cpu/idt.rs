// =============================================================================
// Ainux Interrupt Descriptor Table (IDT) — Safe & Diagnostic
// =============================================================================

use core::arch::{asm, naked_asm};
use core::mem::size_of;

// IDT Entry Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    types_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry {
    pub fn new(offset: u64, selector: u16, types_attr: u8) -> Self {
        Self {
            offset_low: offset as u16,
            selector,
            ist: 0,
            types_attr,
            offset_mid: (offset >> 16) as u16,
            offset_high: (offset >> 32) as u32,
            zero: 0,
        }
    }

    pub fn new_ist(offset: u64, selector: u16, types_attr: u8, ist: u8) -> Self {
        let mut entry = Self::new(offset, selector, types_attr);
        entry.ist = ist & 0x07;
        entry
    }
}

#[repr(C, packed)]
pub struct IdtPointer {
    limit: u16,
    base: u64,
}

static mut IDT: Idt = Idt {
    entries: [IdtEntry {
        offset_low: 0,
        selector: 0,
        ist: 0,
        types_attr: 0,
        offset_mid: 0,
        offset_high: 0,
        zero: 0,
    }; 256],
};

#[repr(C, align(16))]
struct Idt {
    entries: [IdtEntry; 256],
}

pub fn init() {
    unsafe {
        // Exceptions (0-31)
        IDT.entries[0] = IdtEntry::new(exc_divide_error as u64, 0x08, 0x8E);
        IDT.entries[3] = IdtEntry::new(exc_breakpoint as u64, 0x08, 0x8E);
        IDT.entries[6] = IdtEntry::new(exc_invalid_opcode as u64, 0x08, 0x8E);
        IDT.entries[8] = IdtEntry::new_ist(exc_double_fault as u64, 0x08, 0x8E, 1);
        IDT.entries[13] = IdtEntry::new(exc_gpf as u64, 0x08, 0x8E);
        IDT.entries[14] = IdtEntry::new(exc_page_fault as u64, 0x08, 0x8E);

        // IRQs (32-47)
        IDT.entries[32] = IdtEntry::new(irq_timer as u64, 0x08, 0x8E);
        IDT.entries[33] = IdtEntry::new(irq_keyboard as u64, 0x08, 0x8E);
        IDT.entries[44] = IdtEntry::new(irq_mouse as u64, 0x08, 0x8E);
        IDT.entries[43] = IdtEntry::new(irq_net as u64, 0x08, 0x8E);

        // Stub remaining IRQs
        for i in 34..43 { IDT.entries[i] = IdtEntry::new(irq_stub as u64, 0x08, 0x8E); }
        for i in 45..48 { IDT.entries[i] = IdtEntry::new(irq_stub as u64, 0x08, 0x8E); }

        let idt_ptr = IdtPointer {
            limit: (size_of::<Idt>() - 1) as u16,
            base: (&IDT as *const Idt) as u64,
        };

        asm!("lidt [{}]", in(reg) &idt_ptr, options(nostack));
    }
}

// =============================================================================
// ASSEMBLY STUBS (Conditional swapgs + Register Save)
// =============================================================================

macro_rules! interrupt_stub {
    ($name:ident, $rust_handler:ident) => {
        #[unsafe(naked)]
        extern "C" fn $name() {
            unsafe {
                naked_asm!(
                    "push 0", // Dummy Error Code
                    "push r15", "push r14", "push r13", "push r12",
                    "push r11", "push r10", "push r9", "push r8",
                    "push rbp", "push rdi", "push rsi", "push rdx", "push rcx", "push rax",
                    "push rbx",

                    "mov rax, [rsp + 136]", // CS at [rsp + 17*8]
                    "and rax, 3",
                    "cmp rax, 3",
                    "jne 2f",
                    "swapgs",
                    "2:",

                    "mov rdi, rsp",
                    "call {}",

                    "mov rax, [rsp + 136]",
                    "and rax, 3",
                    "cmp rax, 3",
                    "jne 3f",
                    "swapgs",
                    "3:",

                    "pop rbx",
                    "pop rax", "pop rcx", "pop rdx", "pop rsi", "pop rdi", "pop rbp",
                    "pop r8", "pop r9", "pop r10", "pop r11",
                    "pop r12", "pop r13", "pop r14", "pop r15",
                    "add rsp, 8", // Pop dummy error code
                    "iretq",
                    sym $rust_handler
                );
            }
        }
    };
}

macro_rules! interrupt_stub_err {
    ($name:ident, $rust_handler:ident) => {
        #[unsafe(naked)]
        extern "C" fn $name() {
            unsafe {
                naked_asm!(
                    // Error code already pushed by CPU
                    "push r15", "push r14", "push r13", "push r12",
                    "push r11", "push r10", "push r9", "push r8",
                    "push rbp", "push rdi", "push rsi", "push rdx", "push rcx", "push rax",
                    "push rbx",

                    "mov rax, [rsp + 136]",
                    "and rax, 3",
                    "cmp rax, 3",
                    "jne 2f",
                    "swapgs",
                    "2:",

                    "mov rdi, rsp",
                    "call {}",

                    "mov rax, [rsp + 136]",
                    "and rax, 3",
                    "cmp rax, 3",
                    "jne 3f",
                    "swapgs",
                    "3:",

                    "pop rbx",
                    "pop rax", "pop rcx", "pop rdx", "pop rsi", "pop rdi", "pop rbp",
                    "pop r8", "pop r9", "pop r10", "pop r11",
                    "pop r12", "pop r13", "pop r14", "pop r15",
                    "add rsp, 8", // Pop real error code
                    "iretq",
                    sym $rust_handler
                );
            }
        }
    };
}

// Common structure for Rust exception handlers to receive
#[repr(C)]
pub struct InterruptFrame {
    pub rbx: u64,
    pub rax: u64, pub rcx: u64, pub rdx: u64, pub rsi: u64, pub rdi: u64, pub rbp: u64,
    pub r8: u64,  pub r9: u64,  pub r10: u64, pub r11: u64, pub r12: u64, pub r13: u64, pub r14: u64, pub r15: u64,
    pub err_code: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

interrupt_stub!(exc_divide_error, rust_exc_divide_error);
interrupt_stub!(exc_breakpoint, rust_exc_breakpoint);
interrupt_stub!(exc_invalid_opcode, rust_exc_invalid_opcode);
interrupt_stub_err!(exc_double_fault, rust_exc_double_fault);
interrupt_stub_err!(exc_gpf, rust_exc_gpf);
interrupt_stub_err!(exc_page_fault, rust_exc_page_fault);

interrupt_stub!(irq_timer, rust_irq_timer);
interrupt_stub!(irq_keyboard, rust_irq_keyboard);
interrupt_stub!(irq_mouse, rust_irq_mouse);
interrupt_stub!(irq_net, rust_irq_net);
interrupt_stub!(irq_stub, rust_irq_stub);

// =============================================================================
// RUST HANDLERS
// =============================================================================

#[no_mangle]
extern "C" fn rust_exc_divide_error(frame: &InterruptFrame) {
    unsafe {
        print_serial("\n!!! EXCEPTION: DIVIDE BY ZERO !!!\n");
        dump_frame(frame);
        loop { asm!("hlt"); }
    }
}

#[no_mangle]
extern "C" fn rust_exc_breakpoint(frame: &InterruptFrame) {
    unsafe {
        print_serial("\n[Breakpoint] RIP: "); print_hex(frame.rip);
        print_serial("\n");
    }
}

#[no_mangle]
extern "C" fn rust_exc_invalid_opcode(frame: &InterruptFrame) {
    unsafe {
        print_serial("\n!!! EXCEPTION: INVALID OPCODE !!!\n");
        dump_frame(frame);
        loop { asm!("hlt"); }
    }
}

#[no_mangle]
extern "C" fn rust_exc_double_fault(frame: &InterruptFrame) {
    unsafe {
        print_serial("\n!!! EXCEPTION: DOUBLE FAULT (IST1) !!!\n");
        dump_frame(frame);
        
        let cr2: u64; asm!("mov {}, cr2", out(reg) cr2);
        print_serial("  CR2: "); print_hex(cr2);
        
        let cr3: u64; asm!("mov {}, cr3", out(reg) cr3);
        print_serial("  CR3: "); print_hex(cr3);
        print_serial("\nCPU HALTED.\n");
        loop { asm!("hlt"); }
    }
}

#[no_mangle]
extern "C" fn rust_exc_gpf(frame: &InterruptFrame) {
    unsafe {
        print_serial("\n!!! EXCEPTION: GENERAL PROTECTION FAULT !!!\n");
        dump_frame(frame);
        loop { asm!("hlt"); }
    }
}

#[no_mangle]
extern "C" fn rust_exc_page_fault(frame: &InterruptFrame) {
    unsafe {
        let cr2: u64; asm!("mov {}, cr2", out(reg) cr2);
        let gs_base: u64;
        let low: u32; let high: u32;
        asm!("rdmsr", in("rcx") 0xC0000101u32, out("eax") low, out("edx") high, options(nostack, preserves_flags));
        gs_base = ((high as u64) << 32) | (low as u64);

        print_serial("\n!!! EXCEPTION: PAGE FAULT !!!\n");
        print_serial("  CR2: "); print_hex(cr2);
        print_serial("  GS_BASE: "); print_hex(gs_base);
        dump_frame(frame);
        loop { asm!("hlt"); }
    }
}

#[no_mangle]
extern "C" fn rust_irq_timer(_frame: &InterruptFrame) {
    unsafe {
        crate::cpu::pic::notify_eoi(0);
        crate::process::scheduler::tick();
    }
}

#[no_mangle]
extern "C" fn rust_irq_keyboard(_frame: &InterruptFrame) {
    unsafe {
        crate::drivers::keyboard::rust_keyboard_handler();
        crate::cpu::pic::notify_eoi(1);
    }
}

#[no_mangle]
extern "C" fn rust_irq_mouse(_frame: &InterruptFrame) {
    unsafe {
        crate::drivers::mouse::rust_mouse_handler();
        crate::cpu::pic::notify_eoi(12);
    }
}

#[no_mangle]
extern "C" fn rust_irq_net(_frame: &InterruptFrame) {
    crate::drivers::net::rtl8139::RTL8139::handle_interrupt();
    crate::net::dispatch_packets();
    unsafe { crate::cpu::pic::notify_eoi(11); }
}

#[no_mangle]
extern "C" fn rust_irq_stub(_frame: &InterruptFrame) {
    unsafe { crate::cpu::pic::notify_eoi(0); }
}

// =============================================================================
// UTILITIES
// =============================================================================

unsafe fn dump_frame(f: &InterruptFrame) {
    print_serial("  RIP: "); print_hex(f.rip);
    print_serial("  CS: "); print_hex(f.cs);
    print_serial("  ERR: "); print_hex(f.err_code);
    print_serial("\n  RSP: "); print_hex(f.rsp);
    print_serial("  SS: "); print_hex(f.ss);
    print_serial("\n  RAX: "); print_hex(f.rax);
    print_serial(" RBX: "); print_hex(f.rbx);
    print_serial("\n");
}

pub unsafe fn print_hex(mut n: u64) {
    let hex = b"0123456789ABCDEF";
    let mut buf = [0u8; 18];
    buf[0] = b'0'; buf[1] = b'x';
    for i in 0..16 {
        buf[17 - i] = hex[(n & 0xF) as usize];
        n >>= 4;
    }
    for b in buf {
        asm!("out dx, al", in("dx") 0x3F8, in("al") b, options(nomem, nostack, preserves_flags));
    }
}

pub unsafe fn print_serial(s: &str) {
    for b in s.bytes() {
        asm!("out dx, al", in("dx") 0x3F8, in("al") b, options(nomem, nostack, preserves_flags));
    }
}
