use core::arch::asm;

// Primary Bus Ports
const DATA_PORT: u16 = 0x1F0;
const ERROR_PORT: u16 = 0x1F1;
const SECTOR_COUNT_PORT: u16 = 0x1F2;
const LBA_LOW_PORT: u16 = 0x1F3;
const LBA_MID_PORT: u16 = 0x1F4;
const LBA_HIGH_PORT: u16 = 0x1F5;
const DRIVE_HEAD_PORT: u16 = 0x1F6;
const COMMAND_PORT: u16 = 0x1F7;
const STATUS_PORT: u16 = 0x1F7;

const CMD_READ_SECTORS: u8 = 0x20;
const CMD_WRITE_SECTORS: u8 = 0x30;
const CMD_FLUSH_CACHE: u8 = 0xE7;
const CMD_IDENTIFY: u8 = 0xEC;

// Status Flags
const STATUS_BSY: u8 = 0x80;
const STATUS_DRQ: u8 = 0x08;
const STATUS_ERR: u8 = 0x01;

unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

unsafe fn inb(port: u16) -> u8 {
    let mut val: u8;
    asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

unsafe fn inw(port: u16) -> u16 {
    let mut val: u16;
    asm!("in ax, dx", out("ax") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

unsafe fn outw(port: u16, val: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") val, options(nomem, nostack, preserves_flags));
}

fn wait_bsy() -> bool {
    for _ in 0..10_000_000 {
        unsafe {
            let status = inb(STATUS_PORT);
            if status == 0xFF { return false; } 
            if (status & STATUS_BSY) == 0 { return true; }
        }
        core::hint::spin_loop();
    }
    unsafe { crate::drivers::video::put_str("ATA: wait_bsy timeout!\n"); }
    false
}

fn wait_drq() -> bool {
    for _ in 0..10_000_000 {
        unsafe {
            let status = inb(STATUS_PORT);
            if status == 0xFF { return false; }
            if (status & STATUS_ERR) != 0 { return false; }
            if (status & STATUS_DRQ) != 0 { return true; }
        }
        core::hint::spin_loop();
    }
    unsafe { crate::drivers::video::put_str("ATA: wait_drq timeout!\n"); }
    false
}

pub fn read_sectors(target: &mut [u16], lba: u32, sectors: u8) -> bool {
    unsafe {
        if !wait_bsy() { return false; }
        outb(0x3F6, 0x02); // Disable IRQs
        outb(SECTOR_COUNT_PORT, sectors);
        outb(LBA_LOW_PORT, (lba & 0xFF) as u8);
        outb(LBA_MID_PORT, ((lba >> 8) & 0xFF) as u8);
        outb(LBA_HIGH_PORT, ((lba >> 16) & 0xFF) as u8);
        outb(DRIVE_HEAD_PORT, 0xE0 | ((lba >> 24) & 0x0F) as u8);
        outb(COMMAND_PORT, CMD_READ_SECTORS);

        for s in 0..sectors {
            if !wait_bsy() { return false; }
            if !wait_drq() { return false; }
            let offset = (s as usize) * 256;
            for i in 0..256 {
                if offset + i < target.len() {
                    target[offset + i] = inw(DATA_PORT);
                } else {
                    inw(DATA_PORT);
                }
            }
        }
        true
    }
}

pub fn write_sectors(data: &[u16], lba: u32, sectors: u8) -> bool {
    unsafe {
        if !wait_bsy() { return false; }
        outb(0x3F6, 0x02); // Disable IRQs
        outb(SECTOR_COUNT_PORT, sectors);
        outb(LBA_LOW_PORT, (lba & 0xFF) as u8);
        outb(LBA_MID_PORT, ((lba >> 8) & 0xFF) as u8);
        outb(LBA_HIGH_PORT, ((lba >> 16) & 0xFF) as u8);
        outb(DRIVE_HEAD_PORT, 0xE0 | ((lba >> 24) & 0x0F) as u8);
        outb(COMMAND_PORT, CMD_WRITE_SECTORS);

        for s in 0..sectors {
            if !wait_bsy() { return false; }
            if !wait_drq() { return false; }
            let offset = (s as usize) * 256;
            for i in 0..256 {
                let val = if offset + i < data.len() { data[offset + i] } else { 0 };
                outw(DATA_PORT, val);
            }
        }
        outb(COMMAND_PORT, CMD_FLUSH_CACHE);
        if !wait_bsy() { return false; }
        true
    }
}

pub fn identify_buffer(target: &mut [u16]) -> bool {
    if target.len() < 256 { return false; }
    unsafe {
        outb(DRIVE_HEAD_PORT, 0xA0);
        outb(SECTOR_COUNT_PORT, 0);
        outb(LBA_LOW_PORT, 0);
        outb(LBA_MID_PORT, 0);
        outb(LBA_HIGH_PORT, 0);
        outb(COMMAND_PORT, CMD_IDENTIFY);
        
        let status = inb(STATUS_PORT);
        if status == 0 { return false; }
        if !wait_bsy() { return false; }
        if !wait_drq() { return false; }

        for i in 0..256 {
            target[i] = inw(DATA_PORT);
        }
        true
    }
}
