use core::arch::asm;

const CMOS_ADDR: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

pub struct RtcTime {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub day: u8,
    pub month: u8,
    pub year: usize,
}

pub fn init() {
    // RTC is always present on x86
}

fn read_register(reg: u8) -> u8 {
    unsafe {
        asm!("out dx, al", in("dx") CMOS_ADDR, in("al") reg, options(nomem, nostack, preserves_flags));
        let mut ret: u8;
        asm!("in al, dx", out("al") ret, in("dx") CMOS_DATA, options(nomem, nostack, preserves_flags));
        ret
    }
}

fn bcd_to_binary(bcd: u8) -> u8 {
    (bcd & 0x0F) + ((bcd / 16) * 10)
}

pub fn read_time() -> RtcTime {
    let mut seconds = read_register(0x00);
    let mut minutes = read_register(0x02);
    let mut hours = read_register(0x04);
    let mut day = read_register(0x07);
    let mut month = read_register(0x08);
    let mut year = read_register(0x09);
    let register_b = read_register(0x0B);

    // Convert BCD to binary if needed
    if (register_b & 0x04) == 0 {
        seconds = bcd_to_binary(seconds);
        minutes = bcd_to_binary(minutes);
        hours = bcd_to_binary(hours); // Handle 12/24 hour format? careful.
        day = bcd_to_binary(day);
        month = bcd_to_binary(month);
        year = bcd_to_binary(year);
    }
    
    // Convert year to full year (assume 20xx)
    let full_year = 2000 + year as usize;
    
    RtcTime {
        seconds,
        minutes,
        hours,
        day,
        month,
        year: full_year,
    }
}
