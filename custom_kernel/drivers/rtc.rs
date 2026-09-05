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

use core::sync::atomic::{AtomicI32, Ordering};
pub static TIMEZONE_OFFSET_HOURS: AtomicI32 = AtomicI32::new(0);

pub fn init() {
    // RTC is always present on x86, no init needed. 
    // Kept for API compatibility.
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


fn get_update_in_progress_flag() -> bool {
    unsafe {
        asm!("out dx, al", in("dx") CMOS_ADDR, in("al") 0x0Au8, options(nomem, nostack, preserves_flags));
        let mut ret: u8;
        asm!("in al, dx", out("al") ret, in("dx") CMOS_DATA, options(nomem, nostack, preserves_flags));
        (ret & 0x80) != 0
    }
}

pub fn read_time() -> RtcTime {
    let mut t = RtcTime { seconds: 0, minutes: 0, hours: 0, day: 0, month: 0, year: 0 };
    let mut last = RtcTime { seconds: 0, minutes: 0, hours: 0, day: 0, month: 0, year: 0 };

    loop {
        // Wait until update is not in progress
        while get_update_in_progress_flag() {
             core::hint::spin_loop();
        }

        t.seconds = read_register(0x00);
        t.minutes = read_register(0x02);
        t.hours = read_register(0x04);
        t.day = read_register(0x07);
        t.month = read_register(0x08);
        t.year = read_register(0x09) as usize;

        // Verify stability (read twice)
         while get_update_in_progress_flag() {
             core::hint::spin_loop();
        }
        
        last.seconds = read_register(0x00);
        last.minutes = read_register(0x02);
        last.hours = read_register(0x04);
        last.day = read_register(0x07);
        last.month = read_register(0x08);
        last.year = read_register(0x09) as usize;

        if t.seconds == last.seconds && t.minutes == last.minutes && t.hours == last.hours &&
           t.day == last.day && t.month == last.month && t.year == last.year {
            break;
        }
    }

    let register_b = read_register(0x0B);

    // Convert BCD to binary if needed
    if (register_b & 0x04) == 0 {
        t.seconds = bcd_to_binary(t.seconds);
        t.minutes = bcd_to_binary(t.minutes);
        // Handle 12-hour format bit (0x80) if present in BCD mode before masking? 
        // Usually Hour is BCD encoded 0-23 or 1-12. 
        // 0x80 bit usually means PM in 12h mode.
        // Let's safe-guard:
        t.hours = bcd_to_binary(t.hours & 0x7F); 
        t.day = bcd_to_binary(t.day);
        t.month = bcd_to_binary(t.month);
        t.year = bcd_to_binary(t.year as u8) as usize;
    }

    // Convert 12 hour to 24 hour if necessary
    // Bit 1 of Reg B: 1 = 24h, 0 = 12h
    // In 12h mode, bit 7 of hour byte determines PM.
    if (register_b & 0x02) == 0 && (t.hours & 0x80) != 0 {
         t.hours = ((t.hours & 0x7F) + 12) % 24;
    }
    
    // Convert year to full year (assume 20xx)
    let full_year = 2000 + t.year;
    t.year = full_year;

    // Apply Timezone Offset
    let offset = TIMEZONE_OFFSET_HOURS.load(Ordering::Relaxed);
    if offset != 0 {
        let mut h = t.hours as i32 + offset;
        if h < 0 {
            h += 24;
            // Simplistic day wrap (doesn't handle month/year boundaries properly, good enough for toy OS)
            if t.day > 1 { t.day -= 1; }
        } else if h >= 24 {
            h -= 24;
            t.day += 1; // Simplistic day wrap
        }
        t.hours = h as u8;
    }
    
    t
}

