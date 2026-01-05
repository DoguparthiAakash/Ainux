use core::arch::{asm, naked_asm};
use crate::cpu::pic::notify_eoi;

pub fn init() {
    unsafe {
        // Enable Auxiliary Device (Mouse)
        // 1. Wait for Init
        wait_write();
        outb(0x64, 0xA8); // command: enable aux
        
        // 2. Enable Interrupts
        wait_write();
        outb(0x64, 0x20); // read command byte
        let mut status = inb(0x60);
        status |= 2; // Enable IRQ12
        wait_write();
        outb(0x64, 0x60); // write command byte
        wait_write();
        outb(0x60, status);
        
        // 3. Defaults
        mouse_write(0xF6); // Set Default
        mouse_read(); // ACK
        
        mouse_write(0xF4); // Enable Streaming
        mouse_read(); // ACK

        // Unmask IRQ12
        crate::cpu::pic::unmask_irq(12);
    }
}

pub unsafe extern "C" fn mouse_handler_addr() -> u64 {
    mouse_handler as u64
}

unsafe fn wait_write() {
    while (inb(0x64) & 2) != 0 {}
}

unsafe fn wait_read() {
    while (inb(0x64) & 1) == 0 {}
}

unsafe fn mouse_write(byte: u8) {
    wait_write();
    outb(0x64, 0xD4);
    wait_write();
    outb(0x60, byte);
}

unsafe fn mouse_read() -> u8 {
    wait_read();
    inb(0x60)
}

unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

unsafe fn inb(port: u16) -> u8 {
    let mut val: u8;
    asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

#[unsafe(naked)]
extern "C" fn mouse_handler() {
    naked_asm!(
        "push rax", "push rcx", "push rdx", "push rsi", "push rdi", "push r8", "push r9", "push r10", "push r11",
        "call rust_mouse_handler",
        "pop r11", "pop r10", "pop r9", "pop r8", "pop rdi", "pop rsi", "pop rdx", "pop rcx", "pop rax",
        "iretq"
    );
}

static mut MOUSE_CYCLE: u8 = 0;
static mut MOUSE_BYTE: [u8; 3] = [0; 3];

#[no_mangle]
extern "C" fn rust_mouse_handler() {
    unsafe {
        let byte = inb(0x60);
        
        // Simple State Machine
        match MOUSE_CYCLE {
            0 => {
                if (byte & 0x08) != 0 { // Bit 3 must be 1
                    MOUSE_BYTE[0] = byte;
                    MOUSE_CYCLE = 1;
                }
            }
            1 => {
                MOUSE_BYTE[1] = byte;
                MOUSE_CYCLE = 2;
            }
            2 => {
                MOUSE_BYTE[2] = byte;
                MOUSE_CYCLE = 0;
                
                // Process Packet
                let flags = MOUSE_BYTE[0];
                let _x = MOUSE_BYTE[1] as i8;
                let _y = MOUSE_BYTE[2] as i8;
                
                // Print "M" to indicate movement
                 asm!("out dx, al", in("dx") 0x3F8, in("al") 0x4D as u8, options(nomem, nostack, preserves_flags));
                 
                 update_position(_x, _y);
            }
            _ => MOUSE_CYCLE = 0,
        }

        notify_eoi(12);
    }
}

static mut MOUSE_X: isize = 400;
static mut MOUSE_Y: isize = 300;

pub fn get_position() -> (isize, isize) {
    unsafe { (MOUSE_X, MOUSE_Y) }
}

fn update_position(dx: i8, dy: i8) {
    unsafe {
        MOUSE_X += dx as isize;
        MOUSE_Y -= dy as isize; // Y is inverted usually
        if MOUSE_X < 0 { MOUSE_X = 0; }
        if MOUSE_Y < 0 { MOUSE_Y = 0; }
        // Clamp to screen? Need access to Width/Height or just assume.
        if MOUSE_X > 1024 { MOUSE_X = 1024; }
        if MOUSE_Y > 768 { MOUSE_Y = 768; }
    }
}