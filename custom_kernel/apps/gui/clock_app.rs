use crate::gui::app::App;
use crate::drivers::video;

/// Read a byte from a CMOS register via ports 0x70/0x71
unsafe fn cmos_read(reg: u8) -> u8 {
    let val: u8;
    core::arch::asm!(
        "out 0x70, al",
        in("al") reg,
        options(nostack, preserves_flags)
    );
    core::arch::asm!(
        "in al, 0x71",
        out("al") val,
        options(nostack, preserves_flags)
    );
    val
}

fn bcd_to_bin(v: u8) -> u8 {
    (v >> 4) * 10 + (v & 0x0F)
}

fn read_rtc_time() -> (u8, u8, u8) {
    unsafe {
        // Wait until RTC update is not in progress
        let mut tries = 0u32;
        loop {
            let status = cmos_read(0x0A);
            if status & 0x80 == 0 { break; }
            tries += 1;
            if tries > 100_000 { break; }
        }
        let s = bcd_to_bin(cmos_read(0x00));
        let m = bcd_to_bin(cmos_read(0x02));
        let h = bcd_to_bin(cmos_read(0x04));
        (h, m, s)
    }
}

fn read_rtc_date() -> (u16, u8, u8) {
    unsafe {
        let d = bcd_to_bin(cmos_read(0x07));
        let mo = bcd_to_bin(cmos_read(0x08));
        let yr = bcd_to_bin(cmos_read(0x09));
        (2000 + yr as u16, mo, d)
    }
}

pub struct ClockApp {
    hours:   u8,
    minutes: u8,
    seconds: u8,
    year:    u16,
    month:   u8,
    day:     u8,
    ticks:   u32,   // incremented each update(); ~60 Hz
}

impl ClockApp {
    pub fn new() -> Self {
        let (h, m, s) = read_rtc_time();
        let (y, mo, d) = read_rtc_date();
        Self { hours: h, minutes: m, seconds: s, year: y, month: mo, day: d, ticks: 0 }
    }

    fn fill(buf: &mut [u32], bw: usize, bh: usize, x: i32, y: i32, w: i32, h: i32, c: u32) {
        for dy in 0..h {
            let sy = y + dy; if sy < 0 || sy >= bh as i32 { continue; }
            for dx in 0..w {
                let sx = x + dx; if sx < 0 || sx >= bw as i32 { continue; }
                buf[(sy * bw as i32 + sx) as usize] = c;
            }
        }
    }

    /// Draw a large BCD-style 7-segment digit at (x,y), scale pixels per segment cell
    fn draw_digit(buf: &mut [u32], bw: usize, bh: usize, x: i32, y: i32, digit: u8, color: u32) {
        // 7-segment encoding: segments [top, top-left, top-right, mid, bot-left, bot-right, bot]
        const SEGS: [[bool; 7]; 10] = [
            [true,  true,  true,  false, true,  true,  true ],  // 0
            [false, false, true,  false, false, true,  false],  // 1
            [true,  false, true,  true,  true,  false, true ],  // 2
            [true,  false, true,  true,  false, true,  true ],  // 3
            [false, true,  true,  true,  false, true,  false],  // 4
            [true,  true,  false, true,  false, true,  true ],  // 5
            [true,  true,  false, true,  true,  true,  true ],  // 6
            [true,  false, true,  false, false, true,  false],  // 7
            [true,  true,  true,  true,  true,  true,  true ],  // 8
            [true,  true,  true,  true,  false, true,  true ],  // 9
        ];
        let d = (digit % 10) as usize;
        let s = &SEGS[d];
        let sw = 16i32; // segment width
        let sh = 4i32;  // segment thickness
        let sl = 24i32; // segment length

        // top
        if s[0] { Self::fill(buf, bw, bh, x + sh, y,           sl, sh, color); }
        // top-left
        if s[1] { Self::fill(buf, bw, bh, x,       y + sh,      sh, sl, color); }
        // top-right
        if s[2] { Self::fill(buf, bw, bh, x+sh+sl, y + sh,      sh, sl, color); }
        // middle
        if s[3] { Self::fill(buf, bw, bh, x + sh, y + sh + sl, sl, sh, color); }
        // bot-left
        if s[4] { Self::fill(buf, bw, bh, x,       y+sh*2+sl,   sh, sl, color); }
        // bot-right
        if s[5] { Self::fill(buf, bw, bh, x+sh+sl, y+sh*2+sl,   sh, sl, color); }
        // bottom
        if s[6] { Self::fill(buf, bw, bh, x + sh, y+sh*2+sl*2, sl, sh, color); }
    }
}

impl App for ClockApp {
    fn update(&mut self) {
        self.ticks += 1;
        // Refresh from RTC every ~60 ticks (≈1 second at 60Hz update)
        if self.ticks % 60 == 0 {
            let (h, m, s) = read_rtc_time();
            self.hours   = h;
            self.minutes = m;
            self.seconds = s;
            let (y, mo, d) = read_rtc_date();
            self.year  = y;
            self.month = mo;
            self.day   = d;
        }
    }

    fn draw(&mut self, buf: &mut [u32], w: usize, h: usize) {
        // Dark background
        Self::fill(buf, w, h, 0, 0, w as i32, h as i32, 0xFF1A1A2E);

        // Title
        video::draw_text_to_buffer(buf, w as i64, h as i64, 10, 8, "Clock", 0xFF888888);

        // Separator line
        Self::fill(buf, w, h, 0, 24, w as i32, 1, 0xFF333355);

        // --- Digital clock ---
        // Each digit is ~44px wide (sw=16, sl=24, sh=4 → width = sh+sl+sh = 44)
        let digit_w = 46i32;
        let colon_w = 14i32;
        let total_w = digit_w * 6 + colon_w * 2;
        let start_x = (w as i32 - total_w) / 2;
        let dy = 38i32;

        // HH : MM : SS
        let segments = [
            (self.hours   / 10, 0),
            (self.hours   % 10, 1),
            (self.minutes / 10, 3),
            (self.minutes % 10, 4),
            (self.seconds / 10, 6),
            (self.seconds % 10, 7),
        ];

        let clock_color = 0xFF00FFCC;

        for (digit, slot) in segments {
            let px = start_x + slot * digit_w + (slot / 2) * colon_w;
            Self::draw_digit(buf, w, h, px, dy, digit, clock_color);
        }

        // Colons (blink on odd seconds)
        let colon_color = if self.seconds % 2 == 0 { clock_color } else { 0xFF004444 };
        let c1x = start_x + 2 * digit_w;
        let c2x = start_x + 4 * digit_w + colon_w;
        Self::fill(buf, w, h, c1x + 4, dy + 16, 5, 5, colon_color);
        Self::fill(buf, w, h, c1x + 4, dy + 32, 5, 5, colon_color);
        Self::fill(buf, w, h, c2x + 4, dy + 16, 5, 5, colon_color);
        Self::fill(buf, w, h, c2x + 4, dy + 32, 5, 5, colon_color);

        // --- Date line ---
        let months = ["", "January","February","March","April","May","June",
                      "July","August","September","October","November","December"];
        let month_name = if self.month >= 1 && self.month <= 12 {
            months[self.month as usize]
        } else { "Unknown" };

        let mut date_str = alloc::string::String::new();
        date_str.push_str(month_name);
        date_str.push(' ');
        date_str.push((b'0' + self.day / 10) as char);
        date_str.push((b'0' + self.day % 10) as char);
        date_str.push_str(", ");
        date_str.push((b'0' + (self.year / 1000) as u8) as char);
        date_str.push((b'0' + ((self.year % 1000) / 100) as u8) as char);
        date_str.push((b'0' + ((self.year % 100)  / 10)  as u8) as char);
        date_str.push((b'0' + (self.year % 10)    as u8) as char);

        let date_x = (w as i32 - date_str.len() as i32 * 8) / 2;
        video::draw_text_to_buffer(buf, w as i64, h as i64, date_x as i64, (dy + 72) as i64, &date_str, 0xFFAAAAAA);

        // Dim hint
        video::draw_text_to_buffer(buf, w as i64, h as i64, 10, h as i64 - 18, "Press R to refresh manually", 0xFF444466);
    }

    fn on_mouse_event(&mut self, _x: i32, _y: i32, _buttons: u8) {}

    fn on_key_event(&mut self, c: char) {
        if c == 'r' || c == 'R' {
            let (h, m, s) = read_rtc_time();
            let (y, mo, d) = read_rtc_date();
            self.hours = h; self.minutes = m; self.seconds = s;
            self.year = y; self.month = mo; self.day = d;
        }
    }
}
