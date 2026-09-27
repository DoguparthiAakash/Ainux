
use crate::drivers::video;

/// Read month/year from CMOS RTC
unsafe fn cmos_read(reg: u8) -> u8 {
    let val: u8;
    core::arch::asm!("out 0x70, al", in("al") reg, options(nostack, preserves_flags));
    core::arch::asm!("in al, 0x71",  out("al") val, options(nostack, preserves_flags));
    val
}

fn bcd_to_bin(v: u8) -> u8 { (v >> 4) * 10 + (v & 0x0F) }

fn read_rtc() -> (u16, u8, u8) {
    unsafe {
        let d  = bcd_to_bin(cmos_read(0x07));
        let mo = bcd_to_bin(cmos_read(0x08));
        let yr = bcd_to_bin(cmos_read(0x09));
        (2000 + yr as u16, mo, d)
    }
}

/// Compute day-of-week for the 1st of a given month/year (0=Sun, 1=Mon, ... 6=Sat)
fn first_weekday(year: u16, month: u8) -> u8 {
    // Tomohiko Sakamoto's algorithm
    let t: [u8; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = year;
    let m = month as u16;
    if m < 3 { y -= 1; }
    ((y + y/4 - y/100 + y/400 + t[(m-1) as usize] as u16 + 1) % 7) as u8
}

/// Days in month (non-leap year handling)
fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11               => 30,
        2 => if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 { 29 } else { 28 },
        _ => 30,
    }
}

pub struct CalendarApp {
    year:  u16,
    month: u8,
    day:   u8,   // today's day (highlighted)
}

impl CalendarApp {
    pub fn new() -> Self {
        let (y, mo, d) = read_rtc();
        Self { year: y, month: mo, day: d }
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

    fn draw_month(&self, buf: &mut [u32], w: usize, h: usize) {
        const MONTHS: [&str; 13] = [
            "", "January","February","March","April","May","June",
            "July","August","September","October","November","December"
        ];
        const DAYS_HEADER: [&str; 7] = ["Su","Mo","Tu","We","Th","Fr","Sa"];

        let col_w = 38i32;
        let row_h = 22i32;
        let pad_x = ((w as i32) - col_w * 7) / 2;
        let pad_y = 52i32;

        // Month + Year header
        let month_name = MONTHS[self.month.min(12) as usize];
        let mut header = alloc::string::String::from(month_name);
        header.push(' ');
        header.push((b'0' + (self.year / 1000) as u8) as char);
        header.push((b'0' + ((self.year % 1000) / 100) as u8) as char);
        header.push((b'0' + ((self.year % 100) / 10) as u8) as char);
        header.push((b'0' + (self.year % 10) as u8) as char);

        let hx = (w as i32 - header.len() as i32 * 8) / 2;
        video::draw_text_to_buffer(buf, w as i64, h as i64, hx as i64, 30, &header, 0xFF000080);

        // Day-of-week headers
        for (i, &dh) in DAYS_HEADER.iter().enumerate() {
            let dx = pad_x + i as i32 * col_w + 10;
            let col = if i == 0 || i == 6 { 0xFFCC0000 } else { 0xFF000000 };
            video::draw_text_to_buffer(buf, w as i64, h as i64, dx as i64, (pad_y - row_h) as i64, dh, col);
        }

        // Separator
        Self::fill(buf, w, h, pad_x, pad_y - 4, col_w * 7, 1, 0xFF888888);

        // Calendar grid
        let first_wd = first_weekday(self.year, self.month) as i32; // 0=Sun
        let total_days = days_in_month(self.year, self.month) as i32;

        for day in 1..=total_days {
            let cell = first_wd + day - 1;
            let col  = (cell % 7) as i32;
            let row  = (cell / 7) as i32;

            let cx = pad_x + col * col_w;
            let cy = pad_y + row * row_h;

            // Highlight today
            if day == self.day as i32 {
                Self::fill(buf, w, h, cx + 2, cy + 1, col_w - 4, row_h - 2, 0xFF000080);
            }

            let is_weekend = col == 0 || col == 6;
            let fg = if day == self.day as i32  { 0xFFFFFFFF }
                     else if is_weekend         { 0xFFCC0000 }
                     else                       { 0xFF000000 };

            // Day number (right-aligned in cell)
            let day_str = {
                let mut s = alloc::string::String::new();
                if day < 10 { s.push(' '); }
                s.push((b'0' + (day / 10) as u8) as char);
                s.push((b'0' + (day % 10) as u8) as char);
                s
            };
            video::draw_text_to_buffer(buf, w as i64, h as i64, (cx + 10) as i64, (cy + 3) as i64, &day_str, fg);
        }
    }
}

pub fn calendar_main() {
    let id = 11;
    let width = 300;
    let height = 240;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("Calendar"),
        x: 450,
        y: 150,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFFFFFFF0; width * height];
    let mut app = CalendarApp::new();
    let mut needs_redraw = true;
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    match c {
                        'a' | 'A' | '\x1B' => {  // left / prev month
                            if app.month == 1 { app.month = 12; app.year -= 1; }
                            else { app.month -= 1; }
                            app.day = 1;
                            needs_redraw = true;
                        }
                        'd' | 'D' | '\r' => {   // right / next month
                            if app.month == 12 { app.month = 1; app.year += 1; }
                            else { app.month += 1; }
                            app.day = 1;
                            needs_redraw = true;
                        }
                        't' | 'T' => {           // jump to today
                            let (y, mo, d) = read_rtc();
                            app.year = y; app.month = mo; app.day = d;
                            needs_redraw = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        
        if needs_redraw {
            // Cream/white background
            CalendarApp::fill(&mut buffer, width, height, 0, 0, width as i32, height as i32, 0xFFFFFFF0);

            // Top strip
            CalendarApp::fill(&mut buffer, width, height, 0, 0, width as i32, 24, 0xFF000080);
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 10, 4, "Calendar", 0xFFFFFFFF);

            // Nav hint
            video::draw_text_to_buffer(&mut buffer, width as i64, height as i64, 10, height as i64 - 18,
                "[ Left/Right arrows or A/D to change month ]", 0xFF888888);

            app.draw_month(&mut buffer, width, height);
            
            needs_redraw = false;
            
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::yield_now();
    }
}
