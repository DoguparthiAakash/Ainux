use crate::gui::app::App;
use crate::drivers::keyboard;
use alloc::string::String;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Display,
    System,
    About,
}

// Wallpaper color presets — written to a global so the compositor can read it
pub static WALLPAPER_COLOR: spin::Mutex<u32> = spin::Mutex::new(0x008080);

pub struct SettingsApp {
    active_tab: Tab,
    selected_color_idx: usize,
    needs_redraw: bool,
}

const COLOR_PRESETS: [(u32, &str); 6] = [
    (0x1E1E2E, "Catppuccin Dark"),
    (0x008080, "Classic Teal"),
    (0x003366, "Midnight Blue"),
    (0x2D3B2D, "Forest Green"),
    (0x1A1A1A, "Pure Dark"),
    (0x4B0082, "Indigo"),
];

impl SettingsApp {
    pub fn new() -> Self {
        Self {
            active_tab: Tab::Display,
            selected_color_idx: 0,
            needs_redraw: true,
        }
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

    fn text(buf: &mut [u32], bw: usize, bh: usize, x: i32, y: i32, s: &str, c: u32) {
        crate::drivers::video::draw_text_to_buffer(buf, bw as i64, bh as i64, x as i64, y as i64, s, c);
    }

    fn draw_display_tab(&mut self, buf: &mut [u32], w: usize, h: usize) {
        Self::text(buf, w, h, 10, 60, "Wallpaper Color Presets", 0xFFCDD6F4);

        for (i, (color, label)) in COLOR_PRESETS.iter().enumerate() {
            let cx = 10 + (i as i32 % 2) * 120; // 2 cols for colors
            let cy = 85 + (i as i32 / 2) * 60;
            // Color swatch
            let selected = i == self.selected_color_idx;
            let border = if selected { 0xFFCBA6F7 } else { 0xFF45475A };
            Self::fill(buf, w, h, cx - 2, cy - 2, 54, 34, border);
            Self::fill(buf, w, h, cx, cy, 50, 30, *color | 0xFF000000);
            Self::text(buf, w, h, cx, cy + 33, label, 0xFFBAC2E8);
        }

        // --- Appearance toggles ---
        Self::text(buf, w, h, 280, 60, "Appearance Settings", 0xFFCDD6F4);
        
        let theme = crate::drivers::video::THEME.lock();
        let scroll_btn_text = if theme.scrollbar_enabled { "Scrollbar: ON " } else { "Scrollbar: OFF" };
        let page_wise_text = if theme.scroll_page_wise { "Scroll: Page" } else { "Scroll: Smooth" };
        let font_text = alloc::format!("Font Scale: {}x", theme.font_size);
        drop(theme);

        // Buttons for Appearance (x=280)
        let app_btns = [
            ("Resolution: Auto", 85),
            ("Resolution: 1024x768", 125),
            ("Resolution: 1920x1080", 165),
            (scroll_btn_text, 205),
            (page_wise_text, 245),
            (&font_text, 285),
        ];

        for (label, cy) in app_btns.iter() {
            Self::fill(buf, w, h, 280, *cy, 180, 30, 0xFF313244);
            Self::fill(buf, w, h, 280, *cy + 28, 180, 2, 0xFF45475A); // button bottom edge
            Self::text(buf, w, h, 290, *cy + 8, label, 0xFFCDD6F4);
        }

        let vw = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock();
        let vh = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock();
        let res_str = alloc::format!("Current Framebuffer: {}x{}", vw, vh);
        Self::text(buf, w, h, 10, h as i32 - 30, &res_str, 0xFF6C7086);

        // Live preview swatch
        let cur_color = *WALLPAPER_COLOR.lock();
        Self::fill(buf, w, h, w as i32 - 90, 60, 70, 50, cur_color | 0xFF000000);
        Self::text(buf, w, h, w as i32 - 90, 115, "Preview", 0xFF6C7086);
    }

    fn draw_system_tab(&mut self, buf: &mut [u32], w: usize, h: usize) {
        let ticks = crate::process::scheduler::get_ticks();
        let uptime_s = ticks / 100; // approx ticks per second
        let uptime_str = alloc::format!(
            "Uptime:   {}h {}m {}s",
            uptime_s / 3600,
            (uptime_s % 3600) / 60,
            uptime_s % 60
        );

        // Process count
        let mut proc_count = 0usize;
        crate::cpu::without_interrupts(|| {
            let tasks = crate::process::scheduler::TASKS.lock();
            for i in 0..crate::process::scheduler::MAX_TASKS {
                if tasks[i].is_some() { proc_count += 1; }
            }
        });

        // Zone memory stats
        let zone_stats = crate::mm::zone::zone_stats();
        let total_bytes: usize = zone_stats.iter().map(|z| z.total_elements * z.element_size).sum();
        let used_bytes: usize = zone_stats.iter().map(|z| z.allocated_elements * z.element_size).sum();

        let mem_str = if total_bytes > 0 {
            alloc::format!("Zone Mem: {} KB used / {} KB total",
                used_bytes / 1024, total_bytes / 1024)
        } else {
            String::from("Zone Mem: N/A")
        };

        let items = [
            ("System", "Ainux OS"),
            ("Version", "1.0.0-alpha"),
            ("Architecture", "x86_64"),
            ("Ticks", ""),
            ("Processes", ""),
            ("Memory", ""),
        ];

        let labels = [
            alloc::format!("System:       Ainux OS"),
            alloc::format!("Version:      1.0.0-alpha"),
            alloc::format!("Architecture: x86_64"),
            uptime_str,
            alloc::format!("Processes:    {}", proc_count),
            mem_str,
        ];
        let _ = items;

        for (i, label) in labels.iter().enumerate() {
            Self::fill(buf, w, h, 8, 60 + i as i32 * 28, w as i32 - 16, 24, 0xFF313244);
            Self::text(buf, w, h, 14, 66 + i as i32 * 28, label, 0xFFCDD6F4);
        }

        // Memory bar
        if total_bytes > 0 {
            let bar_x = 8i32;
            let bar_y = h as i32 - 50;
            let bar_w = (w as i32 - 16).max(1);
            let bar_h = 16i32;
            Self::fill(buf, w, h, bar_x, bar_y, bar_w, bar_h, 0xFF45475A);
            let fill_w = ((used_bytes as i64 * bar_w as i64) / total_bytes as i64) as i32;
            let fill_col = if used_bytes * 100 / total_bytes > 80 { 0xFFF38BA8 } else { 0xFFA6E3A1 };
            Self::fill(buf, w, h, bar_x, bar_y, fill_w, bar_h, fill_col);
            Self::text(buf, w, h, bar_x, bar_y + 20, "Zone Memory Usage", 0xFF6C7086);
        }
    }

    fn draw_about_tab(&mut self, buf: &mut [u32], w: usize, h: usize) {
        let lines = [
            ("  ___    _           ", 0xFFCBA6F7),
            (" / _ \\  (_)_ __  _   ___ __", 0xFFCBA6F7),
            ("/ /_\\ \\ | | '_ \\| | | \\ \\/ /", 0xFF89DCEB),
            ("|  _  | | | | | | |_| |>  < ", 0xFF89DCEB),
            ("|_| |_|_|_|_| |_|\\__,_/_/\\_\\", 0xFFA6E3A1),
            ("", 0xFFFFFFFF),
            ("Version 1.0.0-alpha", 0xFFCDD6F4),
            ("Custom x86_64 Kernel written in Rust", 0xFFCDD6F4),
            ("", 0xFFFFFFFF),
            ("  MIT License — Open Source", 0xFF6C7086),
            ("  Kernel + GUI + VFS + Networking", 0xFF6C7086),
            ("  Sandboxed shell  |  No libc", 0xFF6C7086),
        ];

        for (i, (line, color)) in lines.iter().enumerate() {
            Self::text(buf, w, h, 16, 55 + i as i32 * 18, line, *color);
        }
    }
}

impl App for SettingsApp {
    fn update(&mut self) {
        // Refresh every ~200 ticks for live system stats
        static LAST: spin::Mutex<u64> = spin::Mutex::new(0);
        let t = crate::process::scheduler::get_ticks();
        let mut last = LAST.lock();
        if t.saturating_sub(*last) > 200 && self.active_tab == Tab::System {
            self.needs_redraw = true;
            *last = t;
        }
    }

    fn draw(&mut self, buf: &mut [u32], w: usize, h: usize) {
        if !self.needs_redraw { return; }

        // Background
        Self::fill(buf, w, h, 0, 0, w as i32, h as i32, 0xFF1E1E2E);

        // Tab bar
        let tabs = [("Display", Tab::Display), ("System", Tab::System), ("About", Tab::About)];
        for (i, (name, tab)) in tabs.iter().enumerate() {
            let tx = i as i32 * 90 + 4;
            let active = *tab == self.active_tab;
            let bg = if active { 0xFF313244 } else { 0xFF1E1E2E };
            let fg = if active { 0xFFCBA6F7 } else { 0xFF6C7086 };
            Self::fill(buf, w, h, tx, 4, 86, 28, bg);
            if active {
                Self::fill(buf, w, h, tx, 30, 86, 2, 0xFFCBA6F7); // underline
            }
            Self::text(buf, w, h, tx + 10, 12, name, fg);
        }

        // Divider
        Self::fill(buf, w, h, 0, 34, w as i32, 1, 0xFF45475A);

        // Tab content
        match self.active_tab {
            Tab::Display => self.draw_display_tab(buf, w, h),
            Tab::System  => self.draw_system_tab(buf, w, h),
            Tab::About   => self.draw_about_tab(buf, w, h),
        }

        self.needs_redraw = false;
    }

    fn on_mouse_event(&mut self, x: i32, y: i32, buttons: u8) {
        if buttons & 1 == 0 { return; }

        // Tab clicks
        if y >= 4 && y <= 32 {
            if x < 94 { self.active_tab = Tab::Display; self.needs_redraw = true; }
            else if x < 184 { self.active_tab = Tab::System; self.needs_redraw = true; }
            else if x < 274 { self.active_tab = Tab::About; self.needs_redraw = true; }
        }

        // Color swatch clicks (Display tab)
        if self.active_tab == Tab::Display && y >= 85 {
            for i in 0..COLOR_PRESETS.len() {
                let cx = 10 + (i as i32 % 2) * 120; // Matches 2 columns
                let cy = 85 + (i as i32 / 2) * 60;
                if x >= cx && x < cx + 50 && y >= cy && y < cy + 30 {
                    self.selected_color_idx = i;
                    *WALLPAPER_COLOR.lock() = COLOR_PRESETS[i].0;
                    self.needs_redraw = true;
                    break;
                }
            }

            // Button clicks (Appearance)
            if x >= 280 && x <= 460 {
                if y >= 85 && y <= 115 {
                    crate::drivers::video::auto_resolution();
                    self.needs_redraw = true;
                } else if y >= 125 && y <= 155 {
                    crate::drivers::video::set_resolution(1024, 768, 32);
                    self.needs_redraw = true;
                } else if y >= 165 && y <= 195 {
                    crate::drivers::video::set_resolution(1920, 1080, 32);
                    self.needs_redraw = true;
                } else if y >= 205 && y <= 235 {
                    let mut t = crate::drivers::video::THEME.lock();
                    t.scrollbar_enabled = !t.scrollbar_enabled;
                    self.needs_redraw = true;
                } else if y >= 245 && y <= 275 {
                    let mut t = crate::drivers::video::THEME.lock();
                    t.scroll_page_wise = !t.scroll_page_wise;
                    self.needs_redraw = true;
                } else if y >= 285 && y <= 315 {
                    let mut t = crate::drivers::video::THEME.lock();
                    t.font_size = (t.font_size % 3) + 1;
                    self.needs_redraw = true;
                }
            }
        }
    }

    fn on_key_event(&mut self, c: char) {
        match c {
            '1' => { self.active_tab = Tab::Display; self.needs_redraw = true; }
            '2' => { self.active_tab = Tab::System; self.needs_redraw = true; }
            '3' => { self.active_tab = Tab::About; self.needs_redraw = true; }
            keyboard::KEY_LEFT | keyboard::KEY_RIGHT => {
                self.active_tab = match self.active_tab {
                    Tab::Display => if c == keyboard::KEY_RIGHT { Tab::System } else { Tab::About },
                    Tab::System  => if c == keyboard::KEY_RIGHT { Tab::About } else { Tab::Display },
                    Tab::About   => if c == keyboard::KEY_RIGHT { Tab::Display } else { Tab::System },
                };
                self.needs_redraw = true;
            }
            _ => {}
        }
    }
}
