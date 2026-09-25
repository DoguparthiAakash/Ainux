
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

pub fn clock_main() {
    let id = 10;
    let width = 360;
    let height = 140;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("Clock"),
        x: 400,
        y: 100,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFF1A1A2E; width * height];
    let mut app = ClockApp::new();
    
    loop {
        let mut needs_redraw = false;
        
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    if c == 'r' || c == 'R' {
                        let (h, m, s) = read_rtc_time();
                        let (y, mo, d) = read_rtc_date();
                        app.hours = h; app.minutes = m; app.seconds = s;
                        app.year = y; app.month = mo; app.day = d;
                        needs_redraw = true;
                    }
                }
                _ => {}
            }
        }
        
        app.ticks += 1;
        // Refresh from RTC every ~60 ticks
        if app.ticks % 60 == 0 {
            let (h, m, s) = read_rtc_time();
            app.hours   = h;
            app.minutes = m;
            app.seconds = s;
            let (y, mo, d) = read_rtc_date();
            app.year  = y;
            app.month = mo;
            app.day   = d;
            needs_redraw = true;
        }
        
        if needs_redraw || app.ticks == 1 {
            // Dark background
            ClockApp::fill(&mut buffer, width, height, 0, 0, width as i32, height as i32, 0xFF1A1A2E);

            // Title
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 10, 8, "Clock", 0xFF888888);

            // Separator line
            ClockApp::fill(&mut buffer, width, height, 0, 24, width as i32, 1, 0xFF333355);

            // --- Digital clock ---
            let digit_w = 46i32;
            let colon_w = 14i32;
            let total_w = digit_w * 6 + colon_w * 2;
            let start_x = (width as i32 - total_w) / 2;
            let dy = 38i32;

            // HH : MM : SS
            let segments = [
                (app.hours   / 10, 0),
                (app.hours   % 10, 1),
                (app.minutes / 10, 3),
                (app.minutes % 10, 4),
                (app.seconds / 10, 6),
                (app.seconds % 10, 7),
            ];

            let clock_color = 0xFF00FFCC;

            for (digit, slot) in segments {
                let px = start_x + slot * digit_w + (slot / 2) * colon_w;
                ClockApp::draw_digit(&mut buffer, width, height, px, dy, digit, clock_color);
            }

            // Colons (blink on odd seconds)
            let colon_color = if app.seconds % 2 == 0 { clock_color } else { 0xFF004444 };
            let c1x = start_x + 2 * digit_w;
            let c2x = start_x + 4 * digit_w + colon_w;
            ClockApp::fill(&mut buffer, width, height, c1x + 4, dy + 16, 5, 5, colon_color);
            ClockApp::fill(&mut buffer, width, height, c1x + 4, dy + 32, 5, 5, colon_color);
            ClockApp::fill(&mut buffer, width, height, c2x + 4, dy + 16, 5, 5, colon_color);
            ClockApp::fill(&mut buffer, width, height, c2x + 4, dy + 32, 5, 5, colon_color);

            // --- Date line ---
            let months = ["", "January","February","March","April","May","June",
                          "July","August","September","October","November","December"];
            let month_name = if app.month >= 1 && app.month <= 12 {
                months[app.month as usize]
            } else { "Unknown" };

            let mut date_str = alloc::string::String::new();
            date_str.push_str(month_name);
            date_str.push(' ');
            date_str.push((b'0' + app.day / 10) as char);
            date_str.push((b'0' + app.day % 10) as char);
            date_str.push_str(", ");
            date_str.push((b'0' + (app.year / 1000) as u8) as char);
            date_str.push((b'0' + ((app.year % 1000) / 100) as u8) as char);
            date_str.push((b'0' + ((app.year % 100)  / 10)  as u8) as char);
            date_str.push((b'0' + (app.year % 10)    as u8) as char);

            let date_x = (width as i32 - date_str.len() as i32 * 8) / 2;
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, date_x as i64, (dy + 72) as i64, &date_str, 0xFFAAAAAA);

            // Dim hint
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 10, height as i64 - 18, "Press R to refresh manually", 0xFF444466);
            
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::yield_now();
    }
}
