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
        // Breakpoint (INT3) - No Error Code
        let bp_handler = breakpoint_handler as u64;
        IDT.entries[3] = IdtEntry::new(bp_handler, 0x08, 0x8E);

        // Double Fault (INT8) - Has Error Code
        let df_handler = double_fault_handler as u64;
        IDT.entries[8] = IdtEntry::new(df_handler, 0x08, 0x8E);

        // General Protection Fault (INT13) - Has Error Code
        let gp_handler = gp_fault_handler as u64;
        IDT.entries[13] = IdtEntry::new(gp_handler, 0x08, 0x8E);

        // Page Fault (INT14) - Has Error Code
        let pf_handler = page_fault_handler as u64;
        IDT.entries[14] = IdtEntry::new(pf_handler, 0x08, 0x8E);
        
        // IRQ0: Timer (32)
        let timer_handler = timer_handler_addr();
        IDT.entries[32] = IdtEntry::new(timer_handler, 0x08, 0x8E);

        // IRQ1: Keyboard (32 + 1 = 33)
        let kb_handler = crate::drivers::keyboard::keyboard_handler_addr();
        IDT.entries[33] = IdtEntry::new(kb_handler, 0x08, 0x8E);

        // IRQ12: Mouse (32 + 12 = 44)
        let mouse_handler = crate::drivers::mouse::mouse_handler_addr();
        IDT.entries[44] = IdtEntry::new(mouse_handler, 0x08, 0x8E);

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

exception_err_handler!(double_fault_handler, rust_double_fault_handler);
exception_err_handler!(gp_fault_handler, rust_gp_fault_handler);
exception_err_handler!(page_fault_handler, rust_page_fault_handler);

#[no_mangle]
extern "C" fn rust_breakpoint_handler() {
    unsafe { print_serial("INT3\n"); }
}

#[no_mangle]
extern "C" fn rust_double_fault_handler(_err: u64) {
    unsafe { print_serial("DOUBLE FAULT\n"); }
    loop {}
}

#[no_mangle]
extern "C" fn rust_gp_fault_handler(_err: u64) {
    unsafe { print_serial("GP FAULT\n"); }
    loop {}
}

#[no_mangle]
extern "C" fn rust_page_fault_handler(err: u64) {
    let cr2: u64;
    unsafe { 
        asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack)); 
        print_serial("PAGE FAULT at ");
        print_hex(cr2);
        print_serial(" Err: ");
        print_hex(err);
        print_serial("\n");
        
        // Stack Dump?
        // let rsp: u64;
        // asm!("mov {}, rsp", out(reg) rsp);
        // print_serial("RSP: ");
        // print_hex(rsp);
        // print_serial("\n");
    }
    loop {}
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
extern "C" fn timer_handler_wrapper() {
    naked_asm!(
        "push rax", "push rcx", "push rdx", "push rsi", "push rdi", "push r8", "push r9", "push r10", "push r11",
        "call rust_timer_handler",
        "pop r11", "pop r10", "pop r9", "pop r8", "pop rdi", "pop rsi", "pop rdx", "pop rcx", "pop rax",
        "iretq"
    );
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
