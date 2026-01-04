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

fn wait_bsy() {
    unsafe {
        while (inb(STATUS_PORT) & STATUS_BSY) != 0 {}
    }
}

fn wait_drq() {
    unsafe {
        while (inb(STATUS_PORT) & STATUS_DRQ) == 0 {}
    }
}

pub fn read_sectors(target: &mut [u16], lba: u32, sectors: u8) {
    unsafe {
        wait_bsy();
        outb(0x3F6, 0x02); // Disable IRQs on Secondary (Control Reg) ? No, we are using Primary. 
                           // Actually 0x3F6 is Control register for Primary? 
                           // 0x3F6 is Device Control for Primary. 0x02 = nIEN (Disable Interrupts)
        
        outb(SECTOR_COUNT_PORT, sectors);
        outb(LBA_LOW_PORT, (lba & 0xFF) as u8);
        outb(LBA_MID_PORT, ((lba >> 8) & 0xFF) as u8);
        outb(LBA_HIGH_PORT, ((lba >> 16) & 0xFF) as u8);
        outb(DRIVE_HEAD_PORT, 0xE0 | ((lba >> 24) & 0x0F) as u8); // 0xE0 = LBA Mode, Master Drive

        outb(COMMAND_PORT, CMD_READ_SECTORS);

        for _ in 0..sectors {
            wait_bsy();
            wait_drq();

            for i in 0..256 {
                target[i] = inw(DATA_PORT);
            }
        }
    }
}

pub fn identify() -> bool {
    unsafe {
        // Select Drive
        outb(DRIVE_HEAD_PORT, 0xA0); // Master
        // Zero counts
        outb(SECTOR_COUNT_PORT, 0);
        outb(LBA_LOW_PORT, 0);
        outb(LBA_MID_PORT, 0);
        outb(LBA_HIGH_PORT, 0);
        
        outb(COMMAND_PORT, CMD_IDENTIFY);
        
        let status = inb(STATUS_PORT);
        if status == 0 {
            return false;
        }
        
        wait_bsy();
        
        // Check LBA Mid/High to see if it's ATA
        let mid = inb(LBA_MID_PORT);
        let high = inb(LBA_HIGH_PORT);
        
        if mid != 0 || high != 0 {
            // Not ATA (Maybe ATAPI)
            return false;
        }

        loop {
            let s = inb(STATUS_PORT);
            if (s & STATUS_ERR) != 0 {
                return false; 
            }
            if (s & STATUS_DRQ) != 0 {
                break;
            }
        }
        
        // Read 256 words (garbage or info)
        for _ in 0..256 {
            inw(DATA_PORT);
        }
        
        return true;
    }
}
