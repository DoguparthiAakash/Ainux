use core::arch::asm;
use spin::Mutex;

pub struct AtaDevice {
    pub base_port: u16,
    pub ctrl_port: u16,
    pub drive: u8, // 0xA0 or 0xB0
}

pub static ACTIVE_ATA: Mutex<AtaDevice> = Mutex::new(AtaDevice {
    base_port: 0x1F0,
    ctrl_port: 0x3F6,
    drive: 0xA0,
});

const CMD_READ_SECTORS: u8 = 0x20;
const CMD_WRITE_SECTORS: u8 = 0x30;
const CMD_FLUSH_CACHE: u8 = 0xE7;
const CMD_IDENTIFY: u8 = 0xEC;
const CMD_IDENTIFY_PACKET: u8 = 0xA1;

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

fn wait_bsy(base_port: u16) -> bool {
    for _ in 0..10_000_000 {
        unsafe {
            let status = inb(base_port + 7);
            if status == 0xFF { return false; } 
            if (status & STATUS_BSY) == 0 { return true; }
        }
        core::hint::spin_loop();
    }
    unsafe { crate::drivers::video::put_str("ATA: wait_bsy timeout!\n"); }
    false
}

fn wait_drq(base_port: u16) -> bool {
    for _ in 0..10_000_000 {
        unsafe {
            let status = inb(base_port + 7);
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
    let dev = ACTIVE_ATA.lock();
    let base = dev.base_port;
    let ctrl = dev.ctrl_port;
    let drive = dev.drive;
    drop(dev);

    unsafe {
        if !wait_bsy(base) { return false; }
        outb(ctrl, 0x02); // Disable IRQs
        outb(base + 2, sectors);
        outb(base + 3, (lba & 0xFF) as u8);
        outb(base + 4, ((lba >> 8) & 0xFF) as u8);
        outb(base + 5, ((lba >> 16) & 0xFF) as u8);
        outb(base + 6, drive | ((lba >> 24) & 0x0F) as u8);
        outb(base + 7, CMD_READ_SECTORS);

        for s in 0..sectors {
            if !wait_bsy(base) { return false; }
            if !wait_drq(base) { return false; }
            let offset = (s as usize) * 256;
            for i in 0..256 {
                if offset + i < target.len() {
                    target[offset + i] = inw(base);
                } else {
                    inw(base);
                }
            }
        }
        true
    }
}

pub fn write_sectors(data: &[u16], lba: u32, sectors: u8) -> bool {
    let dev = ACTIVE_ATA.lock();
    let base = dev.base_port;
    let ctrl = dev.ctrl_port;
    let drive = dev.drive;
    drop(dev);

    unsafe {
        if !wait_bsy(base) { return false; }
        outb(ctrl, 0x02); // Disable IRQs
        outb(base + 2, sectors);
        outb(base + 3, (lba & 0xFF) as u8);
        outb(base + 4, ((lba >> 8) & 0xFF) as u8);
        outb(base + 5, ((lba >> 16) & 0xFF) as u8);
        outb(base + 6, drive | ((lba >> 24) & 0x0F) as u8);
        outb(base + 7, CMD_WRITE_SECTORS);

        for s in 0..sectors {
            if !wait_bsy(base) { return false; }
            if !wait_drq(base) { return false; }
            let offset = (s as usize) * 256;
            for i in 0..256 {
                let val = if offset + i < data.len() { data[offset + i] } else { 0 };
                outw(base, val);
            }
        }
        outb(base + 7, CMD_FLUSH_CACHE);
        if !wait_bsy(base) { return false; }
        true
    }
}

fn probe_drive(base: u16, ctrl: u16, drive: u8, target: &mut [u16]) -> bool {
    unsafe {
        // Select drive
        outb(base + 6, drive);
        // Small delay (400ns)
        for _ in 0..4 { inb(ctrl); }
        
        // Reset ports
        outb(base + 2, 0);
        outb(base + 3, 0);
        outb(base + 4, 0);
        outb(base + 5, 0);
        
        outb(base + 7, CMD_IDENTIFY);
        
        // Wait for status
        for _ in 0..4 { inb(ctrl); }
        let status = inb(base + 7);
        if status == 0 { return false; }
        
        let mut is_atapi = false;
        let lba_mid = inb(base + 4);
        let lba_high = inb(base + 5);
        
        if lba_mid == 0x14 && lba_high == 0xEB {
            is_atapi = true;
        } else if lba_mid != 0 || lba_high != 0 {
            return false; // Unknown
        }
        
        if is_atapi {
            outb(base + 7, CMD_IDENTIFY_PACKET);
            for _ in 0..4 { inb(ctrl); }
        }
        
        if !wait_bsy(base) { return false; }
        if !wait_drq(base) { return false; }

        for i in 0..256 {
            target[i] = inw(base);
        }
        true
    }
}

pub fn identify_buffer(target: &mut [u16]) -> bool {
    if target.len() < 256 { return false; }
    
    let buses = [
        (0x1F0, 0x3F6, 0xA0),
        (0x1F0, 0x3F6, 0xB0),
        (0x170, 0x376, 0xA0),
        (0x170, 0x376, 0xB0),
    ];

    for &(base, ctrl, drive) in &buses {
        if probe_drive(base, ctrl, drive, target) {
            let mut active = ACTIVE_ATA.lock();
            active.base_port = base;
            active.ctrl_port = ctrl;
            active.drive = drive;
            return true;
        }
    }
    false
}
