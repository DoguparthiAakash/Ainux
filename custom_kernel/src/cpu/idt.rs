use core::arch::{asm, naked_asm};
use core::mem::size_of;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    flags: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    fn missing() -> Self {
        Self {
            offset_low: 0, selector: 0, ist: 0, flags: 0, offset_mid: 0, offset_high: 0, reserved: 0
        }
    }

    fn new(handler: u64, selector: u16, flags: u8) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector,
            ist: 0,
            flags,
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }

    /// Create an IDT entry with a specific IST index (1-7)
    fn new_with_ist(handler: u64, selector: u16, flags: u8, ist_index: u8) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector,
            ist: ist_index & 0x7,
            flags,
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }
}

#[repr(C, align(16))]
struct Idt {
    entries: [IdtEntry; 256],
}

#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

static mut IDT: Idt = Idt {
    entries: [IdtEntry { offset_low: 0, selector: 0, ist: 0, flags: 0, offset_mid: 0, offset_high: 0, reserved: 0 }; 256]
};

pub fn init() {
    unsafe {
        // Divide-by-Zero (INT0) - No Error Code
        let div_handler = divide_error_handler as u64;
        IDT.entries[0] = IdtEntry::new(div_handler, 0x08, 0x8E);

        // Breakpoint (INT3) - No Error Code
        let bp_handler = breakpoint_handler as u64;
        IDT.entries[3] = IdtEntry::new(bp_handler, 0x08, 0x8E);

        // Invalid Opcode (INT6) - No Error Code
        let ud_handler = invalid_opcode_handler as u64;
        IDT.entries[6] = IdtEntry::new(ud_handler, 0x08, 0x8E);

        // Double Fault (INT8) - Has Error Code, uses IST1 for fault isolation
        let df_handler = double_fault_handler as u64;
        IDT.entries[8] = IdtEntry::new_with_ist(df_handler, 0x08, 0x8E, 1);

        // General Protection Fault (INT13) - Has Error Code, uses IST2
        let gp_handler = gp_fault_handler as u64;
        IDT.entries[13] = IdtEntry::new_with_ist(gp_handler, 0x08, 0x8E, 2);

        // Page Fault (INT14) - Has Error Code, uses IST2 for fault isolation from user mode
        let pf_handler = page_fault_handler as u64;
        IDT.entries[14] = IdtEntry::new_with_ist(pf_handler, 0x08, 0x8E, 2);
        
        // IRQ0: Timer (32)
        let timer_handler = timer_handler_addr();
        IDT.entries[32] = IdtEntry::new(timer_handler, 0x08, 0x8E);

        // IRQ1: Keyboard (32 + 1 = 33)
        let kb_handler = crate::drivers::keyboard::keyboard_handler_addr();
        IDT.entries[33] = IdtEntry::new(kb_handler, 0x08, 0x8E);

        // IRQ12: Mouse (44)
        let mouse_handler = crate::drivers::mouse::mouse_handler_addr();
        IDT.entries[44] = IdtEntry::new(mouse_handler, 0x08, 0x8E);

        // IRQ11: RTL8139 (32 + 11 = 43)
        let net_handler = rtl8139_handler_addr();
        IDT.entries[43] = IdtEntry::new(net_handler, 0x08, 0x8E);

        // IRQ11: RTL8139 (32 + 11 = 43)
        let net_handler = rtl8139_handler_addr();
        IDT.entries[43] = IdtEntry::new(net_handler, 0x08, 0x8E);

        // Fill ALL remaining IRQ vectors (32-47)
        let stub = irq_stub_handler_addr();
        for vec in 32..=47 {
            // Skip vectors with real handlers
            if vec == 32 || vec == 33 || vec == 43 || vec == 44 {
                continue;
            }
            IDT.entries[vec] = IdtEntry::new(stub, 0x08, 0x8E);
        }

        let idt_ptr = IdtPointer {
            limit: (size_of::<Idt>() - 1) as u16,
            base: (&IDT as *const Idt) as u64,
        };

        asm!("lidt [{}]", in(reg) &idt_ptr, options(nostack));
    }
}

// MACRO for Exception with Error Code
macro_rules! exception_err_handler {
    ($name:ident, $rust_handler:ident) => {
        #[unsafe(naked)]
        extern "C" fn $name() {
            naked_asm!(
                "push rax", "push rcx", "push rdx", "push rsi", "push rdi", "push r8", "push r9", "push r10", "push r11",
                "mov rdi, [rsp + 72]", // Error Code is at RSP + 9*8 = 72
                "mov rsi, [rsp + 80]", // RIP is at RSP + 72 + 8 = 80
                "call {}",
                "pop r11", "pop r10", "pop r9", "pop r8", "pop rdi", "pop rsi", "pop rdx", "pop rcx", "pop rax",
                "add rsp, 8", // Pop error code
                "iretq",
                sym $rust_handler
            );
        }
    }
}

#[unsafe(naked)]
extern "C" fn breakpoint_handler() {
    naked_asm!(
        "push rax", "push rcx", "push rdx", "push rsi", "push rdi", "push r8", "push r9", "push r10", "push r11",
        "call rust_breakpoint_handler",
        "pop r11", "pop r10", "pop r9", "pop r8", "pop rdi", "pop rsi", "pop rdx", "pop rcx", "pop rax",
        "iretq"
    );
}

// -- Exception Handlers (No Error Code) --

#[unsafe(naked)]
extern "C" fn divide_error_handler() {
    naked_asm!(
        "push rax", "push rcx", "push rdx", "push rsi", "push rdi", "push r8", "push r9", "push r10", "push r11",
        "mov rdi, [rsp + 72]", // RIP at RSP + 9*8
        "call rust_divide_error_handler",
        "pop r11", "pop r10", "pop r9", "pop r8", "pop rdi", "pop rsi", "pop rdx", "pop rcx", "pop rax",
        "iretq"
    );
}

#[unsafe(naked)]
extern "C" fn invalid_opcode_handler() {
    naked_asm!(
        "push rax", "push rcx", "push rdx", "push rsi", "push rdi", "push r8", "push r9", "push r10", "push r11",
        "mov rdi, [rsp + 72]", // RIP at RSP + 9*8
        "call rust_invalid_opcode_handler",
        "pop r11", "pop r10", "pop r9", "pop r8", "pop rdi", "pop rsi", "pop rdx", "pop rcx", "pop rax",
        "iretq"
    );
}

exception_err_handler!(double_fault_handler, rust_double_fault_handler);
exception_err_handler!(gp_fault_handler, rust_gp_fault_handler);

#[no_mangle]
extern "C" fn rust_divide_error_handler(rip: u64) {
    unsafe {
        print_serial("DIVIDE BY ZERO at RIP: ");
        print_hex(rip);
        print_serial("\n");
    }
    loop {}
}

#[no_mangle]
extern "C" fn rust_invalid_opcode_handler(rip: u64) {
    unsafe {
        print_serial("INVALID OPCODE at RIP: ");
        print_hex(rip);
        print_serial("\n");
    }
    loop {}
}

#[no_mangle]
extern "C" fn rust_breakpoint_handler() {
    unsafe { print_serial("INT3\n"); }
}

#[no_mangle]
extern "C" fn rust_double_fault_handler(_err: u64) {
    // Running on IST1 stack — safe even with corrupted kernel stack
    unsafe {
        print_serial("DOUBLE FAULT (IST1)\n");
    }
    loop {}
}

#[no_mangle]
extern "C" fn rust_gp_fault_handler(err: u64, rip: u64) {
    unsafe { 
        print_serial("GP FAULT at RIP: ");
        print_hex(rip);
        print_serial(" Error Code: ");
        print_hex(err);
        print_serial("\n");
    }
    loop {}
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExceptionContext {
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rax: u64,
    pub err: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[unsafe(naked)]
extern "C" fn page_fault_handler() {
    naked_asm!(
        "push rax", "push rcx", "push rdx", "push rsi", "push rdi", "push r8", "push r9", "push r10", "push r11",
        "mov rdi, rsp", // Pass the pointer to the saved context
        "call rust_page_fault_handler",
        "pop r11", "pop r10", "pop r9", "pop r8", "pop rdi", "pop rsi", "pop rdx", "pop rcx", "pop rax",
        "add rsp, 8", // pop error code
        "iretq"
    );
}

#[no_mangle]
extern "C" fn rust_page_fault_handler(ctx: *const ExceptionContext) {
    let cr2: u64;
    unsafe { 
        asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack)); 
        let c = &*ctx;
        print_serial("PAGE FAULT at CR2=");
        print_hex(cr2);
        print_serial(" Err=");
        print_hex(c.err);
        print_serial(" RIP=");
        print_hex(c.rip);
        print_serial(" RSP=");
        print_hex(c.rsp);
        print_serial(" CS="); print_hex(c.cs);
        print_serial(" SS="); print_hex(c.ss);
        print_serial(" RFLAGS="); print_hex(c.rflags);
        print_serial("\n");
        print_serial("RAX="); print_hex(c.rax); print_serial(" RBX=???"); print_serial(" RCX="); print_hex(c.rcx); print_serial(" RDX="); print_hex(c.rdx); print_serial("\n");
        print_serial("RSI="); print_hex(c.rsi); print_serial(" RDI="); print_hex(c.rdi); print_serial(" R8="); print_hex(c.r8); print_serial(" R9="); print_hex(c.r9); print_serial("\n");
        print_serial("TSS.rsp0="); print_hex(crate::cpu::gdt::TSS.rsp0);
        print_serial(" TSS.ist1="); print_hex(crate::cpu::gdt::TSS.ist1);
        print_serial(" TSS.ist2="); print_hex(crate::cpu::gdt::TSS.ist2);
        print_serial("\n");
        // Decode error code bits
        if c.err & 1 != 0 { print_serial(" [PRESENT]"); }
        if c.err & 2 != 0 { print_serial(" [WRITE]"); } else { print_serial(" [READ]"); }
        if c.err & 4 != 0 { print_serial(" [USER]"); } else { print_serial(" [KERNEL]"); }
        if c.err & 8 != 0 { print_serial(" [RSVD]"); }
        if c.err & 16 != 0 { print_serial(" [IFETCH]"); }
        print_serial("\n");
        // Hang
        loop {}
    }
}

pub unsafe fn print_hex(mut n: u64) {
    let hex = b"0123456789ABCDEF";
    let mut buf = [0u8; 18]; // 0x + 16 digits
    buf[0] = b'0';
    buf[1] = b'x';
    for i in 0..16 {
        buf[17 - i] = hex[(n & 0xF) as usize];
        n >>= 4;
    }
    for b in buf {
        asm!("out dx, al", in("dx") 0x3F8, in("al") b, options(nomem, nostack, preserves_flags));
    }
}

#[unsafe(naked)]
pub unsafe extern "C" fn timer_handler_wrapper() {
    core::arch::naked_asm!(
        "push r15",
        "push r14",
        "push r13",
        "push r12",
        "push r11",
        "push r10",
        "push r9",
        "push r8",
        "push rbp",
        "push rdi",
        "push rsi",
        "push rdx",
        "push rcx",
        "push rbx",
        "push rax",
        
        // Pass a pointer to the saved registers as the first argument (rdi)
        "mov rdi, rsp",
        "call rust_timer_handler",
        
        "pop rax",
        "pop rbx",
        "pop rcx",
        "pop rdx",
        "pop rsi",
        "pop rdi",
        "pop rbp",
        "pop r8",
        "pop r9",
        "pop r10",
        "pop r11",
        "pop r12",
        "pop r13",
        "pop r14",
        "pop r15",
        
        "iretq"
    );
}

#[no_mangle]
extern "C" fn debug_timer_stack(rsp: *const u64) {
    unsafe {
        let cs = *rsp.add(1);
        if cs != 0x23 {
            return; // Only care about User Mode
        }
        
        let rip = *rsp;
        let rflags = *rsp.add(2);
        let rsp_val = *rsp.add(3);
        let ss = *rsp.add(4);
        
        crate::cpu::idt::print_serial("IRETQ STACK (RET TO USER):\nRIP: ");
        crate::cpu::idt::print_hex(rip);
        crate::cpu::idt::print_serial("\nCS: ");
        crate::cpu::idt::print_hex(cs);
        crate::cpu::idt::print_serial("\nRFLAGS: ");
        crate::cpu::idt::print_hex(rflags);
        crate::cpu::idt::print_serial("\nRSP: ");
        crate::cpu::idt::print_hex(rsp_val);
        crate::cpu::idt::print_serial("\nSS: ");
        crate::cpu::idt::print_hex(ss);
        crate::cpu::idt::print_serial("\n");
        
        // Let's also check if SS is a valid user descriptor.
        if (cs & 3) == 3 && ss != 0x1B {
            crate::cpu::idt::print_serial("CRITICAL: SS is NOT 0x1B!\n");
        }
    }
}


fn timer_handler_addr() -> u64 {
    timer_handler_wrapper as u64
}

#[no_mangle]
extern "C" fn rust_timer_handler() {
    unsafe {
        crate::cpu::pic::notify_eoi(0);
        crate::process::scheduler::tick();
    }
}

unsafe fn print_serial(s: &str) {
     for b in s.bytes() {
        asm!("out dx, al", in("dx") 0x3F8, in("al") b, options(nomem, nostack, preserves_flags));
     }
}

// Stub handler for unhandled IRQs — just sends EOI
#[unsafe(naked)]
extern "C" fn irq_stub_handler_wrapper() {
    naked_asm!(
        "push rax",
        "call rust_irq_stub_handler",
        "pop rax",
        "iretq"
    );
}

fn irq_stub_handler_addr() -> u64 {
    irq_stub_handler_wrapper as u64
}

#[unsafe(naked)]
extern "C" fn rtl8139_handler() {
    unsafe {
        naked_asm!(
            "push rax", "push rcx", "push rdx", "push rsi", "push rdi", "push r8", "push r9", "push r10", "push r11",
            "call rust_rtl8139_handler",
            "pop r11", "pop r10", "pop r9", "pop r8", "pop rdi", "pop rsi", "pop rdx", "pop rcx", "pop rax",
            "iretq"
        );
    }
}

#[no_mangle]
extern "C" fn rust_rtl8139_handler() {
    crate::drivers::net::rtl8139::RTL8139::handle_interrupt();
    crate::net::dispatch_packets();
    unsafe { crate::cpu::pic::notify_eoi(11); } // IRQ11
}

pub fn rtl8139_handler_addr() -> u64 { rtl8139_handler as u64 }

#[no_mangle]
extern "C" fn rust_irq_stub_handler() {
    unsafe {
        crate::cpu::pic::notify_eoi(0);
    }
}
