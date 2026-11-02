use alloc::string::{String, ToString};
use crate::drivers::video;

pub struct ClockApp {
    pub hours:   u8,
    pub minutes: u8,
    pub seconds: u8,
    pub year:    usize,
    pub month:   u8,
    pub day:     u8,
    pub ticks:   u32,
}

impl ClockApp {
    pub fn new() -> Self {
        let t = crate::drivers::rtc::read_time();
        Self {
            hours: t.hours,
            minutes: t.minutes,
            seconds: t.seconds,
            year: t.year,
            month: t.month,
            day: t.day,
            ticks: 0,
        }
    }

    #[inline(always)]
    fn put_pixel(buf: &mut [u32], bw: i32, bh: i32, x: i32, y: i32, color: u32) {
        if x >= 0 && x < bw && y >= 0 && y < bh {
            buf[(y * bw + x) as usize] = color;
        }
    }

    fn draw_line(buf: &mut [u32], bw: i32, bh: i32, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = (if dx > dy { dx } else { -dy }) / 2;
        loop {
            Self::put_pixel(buf, bw, bh, x0, y0, color);
            if x0 == x1 && y0 == y1 { break; }
            let e2 = err;
            if e2 > -dx { err -= dy; x0 += sx; }
            if e2 < dy { err += dx; y0 += sy; }
        }
    }

    fn draw_thick_line(buf: &mut [u32], bw: i32, bh: i32, x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32, color: u32) {
        let half = thickness / 2;
        for off in -half..=half {
            Self::draw_line(buf, bw, bh, x0 + off, y0, x1 + off, y1, color);
            Self::draw_line(buf, bw, bh, x0, y0 + off, x1, y1 + off, color);
        }
    }

    fn draw_circle(buf: &mut [u32], bw: i32, bh: i32, cx: i32, cy: i32, r: i32, color: u32) {
        let mut x = 0;
        let mut y = r;
        let mut d = 3 - 2 * r;
        while x <= y {
            Self::put_pixel(buf, bw, bh, cx + x, cy + y, color);
            Self::put_pixel(buf, bw, bh, cx - x, cy + y, color);
            Self::put_pixel(buf, bw, bh, cx + x, cy - y, color);
            Self::put_pixel(buf, bw, bh, cx - x, cy - y, color);
            Self::put_pixel(buf, bw, bh, cx + y, cy + x, color);
            Self::put_pixel(buf, bw, bh, cx - y, cy + x, color);
            Self::put_pixel(buf, bw, bh, cx + y, cy - x, color);
            Self::put_pixel(buf, bw, bh, cx - y, cy - x, color);
            if d < 0 {
                d += 4 * x + 6;
            } else {
                d += 4 * (x - y) + 10;
                y -= 1;
            }
            x += 1;
        }
    }

    fn fill_circle(buf: &mut [u32], bw: i32, bh: i32, cx: i32, cy: i32, r: i32, color: u32) {
        let r_sq = r * r;
        for dy in -r..=r {
            let y = cy + dy;
            if y < 0 || y >= bh { continue; }
            let span = r_sq - dy * dy;
            if span >= 0 {
                let dx_max = integer_sqrt(span as u32) as i32;
                let start_x = core::cmp::max(0, cx - dx_max);
                let end_x = core::cmp::min(bw - 1, cx + dx_max);
                for x in start_x..=end_x {
                    buf[(y * bw + x) as usize] = color;
                }
            }
        }
    }

    fn fill_rect(buf: &mut [u32], bw: i32, bh: i32, x: i32, y: i32, w: i32, h: i32, color: u32) {
        for dy in 0..h {
            let sy = y + dy;
            if sy < 0 || sy >= bh { continue; }
            for dx in 0..w {
                let sx = x + dx;
                if sx < 0 || sx >= bw { continue; }
                buf[(sy * bw + sx) as usize] = color;
            }
        }
    }
}

fn integer_sqrt(n: u32) -> u32 {
    let mut x0 = n / 2;
    if x0 != 0 {
        let mut x1 = (x0 + n / x0) / 2;
        while x1 < x0 {
            x0 = x1;
            x1 = (x0 + n / x0) / 2;
        }
        x0
    } else {
        n
    }
}

// 60-point sine/cosine table normalized to 1024 (starts at 12 o'clock: 0, -1024)
const SIN_COS: [(i32, i32); 60] = [
    (0, -1024), (107, -1018), (213, -1002), (316, -974), (416, -935), 
    (512, -887), (602, -831), (685, -766), (760, -693), (828, -613),
    (887, -526), (935, -434), (974, -337), (1002, -237), (1018, -134),
    (1024, 0), (1018, 134), (1002, 237), (974, 337), (935, 434),
    (887, 526), (828, 613), (760, 693), (685, 766), (602, 831),
    (512, 887), (416, 935), (316, 974), (213, 1002), (107, 1018),
    (0, 1024), (-107, 1018), (-213, 1002), (-316, 974), (-416, 935),
    (-512, 887), (-602, 831), (-685, 766), (-760, 693), (-828, 613),
    (-887, 526), (-935, 434), (-974, 337), (-1002, 237), (-1018, 134),
    (-1024, 0), (-1018, -134), (-1002, -237), (-974, -337), (-935, -434),
    (-887, -526), (-828, -613), (-760, -693), (-685, -766), (-602, -831),
    (-512, -887), (-416, -935), (-316, -974), (-213, -1002), (-107, -1018)
];

pub fn clock_main() {
    let id = crate::gui::wm::generate_window_id();
    let width: usize = 320;
    let height: usize = 380;
    let bw = width as i32;
    let bh = height as i32;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: String::from("Mithl Clock"),
        x: 320,
        y: 120,
        w: bw,
        h: bh,
    });
    
    let mut buffer = alloc::vec![0xFF0F111A; width * height];
    let mut app = ClockApp::new();
    let mut last_second: u8 = 255;
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::WindowClosed => { return; }
                _ => {}
            }
        }
        
        let t = crate::drivers::rtc::read_time();
        app.hours = t.hours;
        app.minutes = t.minutes;
        app.seconds = t.seconds;
        app.year = t.year;
        app.month = t.month;
        app.day = t.day;
        
        if app.seconds != last_second {
            last_second = app.seconds;
            
            // Clear background with obsidian gradient
            ClockApp::fill_rect(&mut buffer, bw, bh, 0, 0, bw, bh, 0xFF0D0F18);
            
            // Header bar
            ClockApp::fill_rect(&mut buffer, bw, bh, 0, 0, bw, 26, 0xFF141826);
            ClockApp::fill_rect(&mut buffer, bw, bh, 0, 26, bw, 1, 0xFF1F263B);
            video::draw_text_to_buffer(&mut buffer, bw as i64, bh as i64, 12, 6, "Mithl System Time", 0xFF00E5FF);
            
            // Center of analog clock
            let cx = bw / 2;
            let cy = 145;
            let radius = 100;
            
            // Outer bezel ring (glowing cyan / blue)
            ClockApp::fill_circle(&mut buffer, bw, bh, cx, cy, radius + 5, 0xFF1B2338);
            ClockApp::draw_circle(&mut buffer, bw, bh, cx, cy, radius + 5, 0xFF00E5FF);
            ClockApp::draw_circle(&mut buffer, bw, bh, cx, cy, radius + 4, 0xFF0088CC);
            
            // Inner clock dial face
            ClockApp::fill_circle(&mut buffer, bw, bh, cx, cy, radius, 0xFF10131F);
            ClockApp::draw_circle(&mut buffer, bw, bh, cx, cy, radius, 0xFF2A3650);
            
            // Draw 60 ticks
            for i in 0..60 {
                let (sx, sy) = SIN_COS[i];
                if i % 5 == 0 {
                    // Major hour tick
                    let x0 = cx + (sx * (radius - 14)) / 1024;
                    let y0 = cy + (sy * (radius - 14)) / 1024;
                    let x1 = cx + (sx * (radius - 3)) / 1024;
                    let y1 = cy + (sy * (radius - 3)) / 1024;
                    ClockApp::draw_thick_line(&mut buffer, bw, bh, x0, y0, x1, y1, 2, 0xFF00F0FF);
                } else {
                    // Minor minute tick
                    let x0 = cx + (sx * (radius - 7)) / 1024;
                    let y0 = cy + (sy * (radius - 7)) / 1024;
                    let x1 = cx + (sx * (radius - 3)) / 1024;
                    let y1 = cy + (sy * (radius - 3)) / 1024;
                    ClockApp::draw_line(&mut buffer, bw, bh, x0, y0, x1, y1, 0xFF3E4D6E);
                }
            }
            
            // Hour hand (thick white/silver)
            let h_idx = ((app.hours as usize % 12) * 5 + (app.minutes as usize / 12)) % 60;
            let (hx, hy) = SIN_COS[h_idx];
            let h_len = radius * 52 / 100;
            let hx_end = cx + (hx * h_len) / 1024;
            let hy_end = cy + (hy * h_len) / 1024;
            ClockApp::draw_thick_line(&mut buffer, bw, bh, cx, cy, hx_end, hy_end, 4, 0xFFFFFFFF);
            
            // Minute hand (thick cyan)
            let m_idx = app.minutes as usize % 60;
            let (mx, my) = SIN_COS[m_idx];
            let m_len = radius * 76 / 100;
            let mx_end = cx + (mx * m_len) / 1024;
            let my_end = cy + (my * m_len) / 1024;
            ClockApp::draw_thick_line(&mut buffer, bw, bh, cx, cy, mx_end, my_end, 3, 0xFF00E5FF);
            
            // Second hand (crimson needle with counter-tail)
            let s_idx = app.seconds as usize % 60;
            let (sx, sy) = SIN_COS[s_idx];
            let s_len = radius * 88 / 100;
            let tail_len = 16;
            let sx_start = cx - (sx * tail_len) / 1024;
            let sy_start = cy - (sy * tail_len) / 1024;
            let sx_end = cx + (sx * s_len) / 1024;
            let sy_end = cy + (sy * s_len) / 1024;
            ClockApp::draw_thick_line(&mut buffer, bw, bh, sx_start, sy_start, sx_end, sy_end, 2, 0xFFFF3366);
            
            // Center pivot cap
            ClockApp::fill_circle(&mut buffer, bw, bh, cx, cy, 6, 0xFFFFFFFF);
            ClockApp::fill_circle(&mut buffer, bw, bh, cx, cy, 3, 0xFFFF3366);
            
            // Digital Box Panel
            let panel_x = 20;
            let panel_y = 262;
            let panel_w = bw - 40;
            let panel_h = 92;
            ClockApp::fill_rect(&mut buffer, bw, bh, panel_x, panel_y, panel_w, panel_h, 0xFF141828);
            ClockApp::draw_line(&mut buffer, bw, bh, panel_x, panel_y, panel_x + panel_w, panel_y, 0xFF2A3654);
            ClockApp::draw_line(&mut buffer, bw, bh, panel_x, panel_y + panel_h, panel_x + panel_w, panel_y + panel_h, 0xFF2A3654);
            ClockApp::draw_line(&mut buffer, bw, bh, panel_x, panel_y, panel_x, panel_y + panel_h, 0xFF2A3654);
            ClockApp::draw_line(&mut buffer, bw, bh, panel_x + panel_w, panel_y, panel_x + panel_w, panel_y + panel_h, 0xFF2A3654);
            
            // Digital Time Text (e.g. "10:48:20 PM")
            let am_pm = if app.hours >= 12 { "PM" } else { "AM" };
            let hour_12 = if app.hours == 0 { 12 } else if app.hours > 12 { app.hours - 12 } else { app.hours };
            let time_str = alloc::format!("{:02}:{:02}:{:02} {}", hour_12, app.minutes, app.seconds, am_pm);
            let time_x = (bw - (time_str.len() as i32 * 8)) / 2;
            video::draw_text_to_buffer(&mut buffer, bw as i64, bh as i64, time_x as i64, (panel_y + 16) as i64, &time_str, 0xFF00FFCC);
            
            // Date String
            let months = ["", "January", "February", "March", "April", "May", "June",
                          "July", "August", "September", "October", "November", "December"];
            let month_name = if app.month >= 1 && app.month <= 12 { months[app.month as usize] } else { "Unknown" };
            let date_str = alloc::format!("{} {:02}, {:04}", month_name, app.day, app.year);
            let date_x = (bw - (date_str.len() as i32 * 8)) / 2;
            video::draw_text_to_buffer(&mut buffer, bw as i64, bh as i64, date_x as i64, (panel_y + 40) as i64, &date_str, 0xFFB0C0D8);
            
            // Hardware Status
            let sub_str = "RTC Synced · 60 FPS Compositor";
            let sub_x = (bw - (sub_str.len() as i32 * 8)) / 2;
            video::draw_text_to_buffer(&mut buffer, bw as i64, bh as i64, sub_x as i64, (panel_y + 64) as i64, sub_str, 0xFF4A6080);
            
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::sleep(2);
    }
}
