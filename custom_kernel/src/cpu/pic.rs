use core::arch::asm;

const PIC1: u16 = 0x20;
const PIC2: u16 = 0xA0;
const PIC1_COMMAND: u16 = PIC1;
const PIC1_DATA: u16 = PIC1 + 1;
const PIC2_COMMAND: u16 = PIC2;
const PIC2_DATA: u16 = PIC2 + 1;

unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

unsafe fn inb(port: u16) -> u8 {
    let mut val: u8;
    asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

unsafe fn io_wait() {
    outb(0x80, 0);
}

pub unsafe fn init() {
    // Save masks
    let a1 = inb(PIC1_DATA);
    let a2 = inb(PIC2_DATA);

    // ICW1: Init
    outb(PIC1_COMMAND, 0x11);
    io_wait();
    outb(PIC2_COMMAND, 0x11);
    io_wait();

    // ICW2: Offsets (32, 40)
    outb(PIC1_DATA, 0x20); // Offset 32 for Master
    io_wait();
    outb(PIC2_DATA, 0x28); // Offset 40 for Slave
    io_wait();

    // ICW3: Cascade
    outb(PIC1_DATA, 4);
    io_wait();
    outb(PIC2_DATA, 2);
    io_wait();

    // ICW4: 8086 mode
    outb(PIC1_DATA, 1);
    io_wait();
    outb(PIC2_DATA, 1);
    io_wait();

    // Restore masks (Actually, let's Mask ALL initially to be safe)
    outb(PIC1_DATA, 0xFF);
    outb(PIC2_DATA, 0xFF);
}

pub unsafe fn notify_eoi(irq: u8) {
    if irq >= 8 {
        outb(PIC2_COMMAND, 0x20);
    }
    outb(PIC1_COMMAND, 0x20);
}

pub unsafe fn unmask_irq(irq: u8) {
     let port;
     let value;
     if irq < 8 {
         port = PIC1_DATA;
         value = 1 << irq;
     } else {
         port = PIC2_DATA;
         value = 1 << (irq - 8);
     }
     
     let mask = inb(port);
     outb(port, mask & !value);
}
