use core::arch::{asm, naked_asm};
use crate::cpu::pic::notify_eoi;
use crate::drivers::video;
use spin::Mutex;

#[derive(Clone, Copy)]
pub struct MouseEvent {
    pub x: isize,
    pub y: isize,
    pub buttons: u8, // bit 0: left, bit 1: right, bit 2: middle
}

const BUFFER_SIZE: usize = 128;
struct MouseRingBuffer {
    data: [MouseEvent; BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
    count: usize,
}

impl MouseRingBuffer {
    const fn new() -> Self {
        Self {
            data: [MouseEvent { x: 0, y: 0, buttons: 0 }; BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
            count: 0,
        }
    }
    fn push(&mut self, ev: MouseEvent) {
        if self.count < BUFFER_SIZE {
            self.data[self.write_pos] = ev;
            self.write_pos = (self.write_pos + 1) % BUFFER_SIZE;
            self.count += 1;
        }
    }
    fn pop(&mut self) -> Option<MouseEvent> {
        if self.count > 0 {
            let ev = self.data[self.read_pos];
            self.read_pos = (self.read_pos + 1) % BUFFER_SIZE;
            self.count -= 1;
            Some(ev)
        } else {
            None
        }
    }
}

static MOUSE_BUFFER: Mutex<MouseRingBuffer> = Mutex::new(MouseRingBuffer::new());
static mut MOUSE_X: isize = 512;
static mut MOUSE_Y: isize = 384;
static mut MOUSE_BUTTONS: u8 = 0;

pub fn pop_event() -> Option<MouseEvent> {
    MOUSE_BUFFER.lock().pop()
}

pub fn get_position() -> (isize, isize) {
    unsafe { (MOUSE_X, MOUSE_Y) }
}

pub fn get_buttons() -> u8 {
    unsafe { MOUSE_BUTTONS }
}

pub fn get_grid_position() -> (usize, usize) {
    let (x, y) = get_position();
    // TUI is 8x12 usually
    ((x / 8) as usize, (y / 12) as usize)
}

fn update_position(dx: isize, dy: isize, buttons: u8) {
    unsafe {
        MOUSE_X += dx;
        MOUSE_Y -= dy;
        MOUSE_BUTTONS = buttons;

        let w = if let Some(fw) = video::FRAMEBUFFER_WIDTH.try_lock() { *fw as isize } else { 1024 };
        let h = if let Some(fh) = video::FRAMEBUFFER_HEIGHT.try_lock() { *fh as isize } else { 768 };
        let w = w.max(80 * 8);
        let h = h.max(25 * 12);

        if MOUSE_X < 0 { MOUSE_X = 0; }
        if MOUSE_Y < 0 { MOUSE_Y = 0; }
        if MOUSE_X >= w { MOUSE_X = w - 1; }
        if MOUSE_Y >= h { MOUSE_Y = h - 1; }

        MOUSE_BUFFER.lock().push(MouseEvent {
            x: MOUSE_X,
            y: MOUSE_Y,
            buttons: MOUSE_BUTTONS,
        });
    }
}

pub fn init() {
    unsafe {
        // Enable Auxiliary Device (Mouse)
        wait_write();
        outb(0x64, 0xA8); 
        
        wait_write();
        outb(0x64, 0x20); 
        wait_read();      
        let mut status = inb(0x60);
        status |= (1 << 0) | (1 << 1) | (1 << 6); 
        
        wait_write();
        outb(0x64, 0x60); 
        wait_write();
        outb(0x60, status);
        
        mouse_write(0xF6); // Set Default
        mouse_read(); 
        
        mouse_write(0xF4); // Enable Streaming
        mouse_read(); 

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
        
        match MOUSE_CYCLE {
            0 => {
                if (byte & 0x08) != 0 { 
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
                
                let flags = MOUSE_BYTE[0];
                let dx = MOUSE_BYTE[1] as i8 as isize * 3;
                let dy = MOUSE_BYTE[2] as i8 as isize * 3;
                
                update_position(dx, dy, flags & 0x07);
            }
            _ => MOUSE_CYCLE = 0,
        }

        notify_eoi(12);
    }
}