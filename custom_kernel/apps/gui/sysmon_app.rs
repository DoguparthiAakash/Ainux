use crate::gui::app::App;
use alloc::string::String;
use alloc::vec::Vec;

pub struct SysMonApp {
    last_refresh: u64,
    tick_history: [u64; 60], // rolling 60-sample CPU tick deltas
    tick_history_idx: usize,
    proc_history: [usize; 60],
    last_ticks: u64,
    needs_redraw: bool,
    frame: usize,
}

impl SysMonApp {
    pub fn new() -> Self {
        Self {
            last_refresh: 0,
            tick_history: [0; 60],
            tick_history_idx: 0,
            proc_history: [0; 60],
            last_ticks: 0,
            needs_redraw: true,
            frame: 0,
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

    fn draw_bar_graph(buf: &mut [u32], bw: usize, bh: usize, x: i32, y: i32, w: i32, h: i32,
                       samples: &[u64], max_val: u64, bar_color: u32, label: &str) {
        // Background
        Self::fill(buf, bw, bh, x, y, w, h, 0xFF313244);
        // Border
        for px in x..x+w { if px >= 0 && px < bw as i32 && y >= 0 && y < bh as i32 { buf[(y * bw as i32 + px) as usize] = 0xFF45475A; } }
        for px in x..x+w { let py = y+h-1; if px >= 0 && px < bw as i32 && py >= 0 && py < bh as i32 { buf[(py * bw as i32 + px) as usize] = 0xFF45475A; } }
        for py in y..y+h { if x >= 0 && x < bw as i32 && py >= 0 && py < bh as i32 { buf[(py * bw as i32 + x) as usize] = 0xFF45475A; } }
        for py in y..y+h { let px = x+w-1; if px >= 0 && px < bw as i32 && py >= 0 && py < bh as i32 { buf[(py * bw as i32 + px) as usize] = 0xFF45475A; } }

        let n = samples.len();
        if n == 0 || max_val == 0 { return; }

        let inner_w = w - 2;
        let inner_h = h - 2;
        let bar_w = (inner_w / n as i32).max(1);

        for (i, &val) in samples.iter().enumerate() {
            let bar_h = ((val as i64 * inner_h as i64) / max_val as i64) as i32;
            let bx = x + 1 + i as i32 * bar_w;
            let by = y + 1 + inner_h - bar_h;
            // Gradient: brighter bars for higher values
            let intensity = if max_val > 0 { (val * 255 / max_val) as u8 } else { 0 };
            let color = (bar_color & 0xFF000000)
                | (((((bar_color >> 16) & 0xFF) as u16 * intensity as u16 / 255) as u32) << 16)
                | (((((bar_color >> 8) & 0xFF) as u16 * intensity as u16 / 255) as u32) << 8)
                | (((bar_color & 0xFF) as u16 * intensity as u16 / 255) as u32);
            Self::fill(buf, bw, bh, bx, by, bar_w.min(inner_w), bar_h.max(1), color);
        }

        Self::text(buf, bw, bh, x + 4, y + 3, label, 0xFFCDD6F4);
    }
}

impl App for SysMonApp {
    fn update(&mut self) {
        let t = crate::process::scheduler::get_ticks();
        if t.saturating_sub(self.last_refresh) >= 30 {
            // Sample tick delta
            let delta = t.saturating_sub(self.last_ticks);
            self.tick_history[self.tick_history_idx] = delta;

            // Sample process count
            let mut proc_count = 0usize;
            crate::cpu::without_interrupts(|| {
                let tasks = crate::process::scheduler::TASKS.lock();
                for i in 0..crate::process::scheduler::MAX_TASKS {
                    if tasks[i].as_ref().map(|t| t.state != crate::process::task::TaskState::Free).unwrap_or(false) {
                        proc_count += 1;
                    }
                }
            });
            self.proc_history[self.tick_history_idx] = proc_count;

            self.tick_history_idx = (self.tick_history_idx + 1) % 60;
            self.last_ticks = t;
            self.last_refresh = t;
            self.frame += 1;
            self.needs_redraw = true;
        }
    }

    fn draw(&mut self, buf: &mut [u32], w: usize, h: usize) {
        if !self.needs_redraw { return; }

        Self::fill(buf, w, h, 0, 0, w as i32, h as i32, 0xFF1E1E2E);

        // Title
        Self::fill(buf, w, h, 0, 0, w as i32, 22, 0xFF313244);
        Self::text(buf, w, h, 4, 4, "System Monitor", 0xFFCBA6F7);

        // Animated dots to show it's live
        let dot_str = match self.frame % 4 {
            0 => "Live ●   ",
            1 => "Live  ●  ",
            2 => "Live   ● ",
            _ => "Live    ●",
        };
        Self::text(buf, w, h, w as i32 - 80, 4, dot_str, 0xFFA6E3A1);

        let ticks = crate::process::scheduler::get_ticks();
        let uptime_s = ticks / 100;

        // Quick stats
        let stats: Vec<String> = alloc::vec![
            alloc::format!("Uptime:   {}:{:02}:{:02}", uptime_s/3600, (uptime_s%3600)/60, uptime_s%60),
            alloc::format!("Ticks:    {}", ticks),
        ];
        for (i, s) in stats.iter().enumerate() {
            Self::fill(buf, w, h, 4, 26 + i as i32 * 18, w as i32 - 8, 16, 0xFF24273A);
            Self::text(buf, w, h, 8, 28 + i as i32 * 18, s, 0xFFCDD6F4);
        }

        // Zone memory
        let zone_stats = crate::mm::zone::zone_stats();
        let total_kb: usize = zone_stats.iter().map(|z| z.total_elements * z.element_size / 1024).sum();
        let used_kb:  usize = zone_stats.iter().map(|z| z.allocated_elements * z.element_size / 1024).sum();
        let mem_str = alloc::format!("Zone Mem: {} / {} KB", used_kb, total_kb.max(1));
        Self::fill(buf, w, h, 4, 62, w as i32 - 8, 16, 0xFF24273A);
        Self::text(buf, w, h, 8, 64, &mem_str, 0xFFCDD6F4);

        // Memory usage bar
        let bar_x = 4i32;
        let bar_y = 82i32;
        let bar_w  = w as i32 - 8;
        let bar_h  = 12i32;
        Self::fill(buf, w, h, bar_x, bar_y, bar_w, bar_h, 0xFF45475A);
        if total_kb > 0 {
            let fill_w = ((used_kb as i64 * bar_w as i64) / total_kb as i64) as i32;
            let pct = used_kb * 100 / total_kb;
            let col = if pct > 80 { 0xFFF38BA8 } else { 0xFFA6E3A1 };
            Self::fill(buf, w, h, bar_x, bar_y, fill_w.max(1), bar_h, col);
        }

        // Tick activity graph
        let graph_y = 102i32;
        let graph_h = (h as i32 - graph_y - 28) / 2;
        let max_tick = *self.tick_history.iter().max().unwrap_or(&1);
        Self::draw_bar_graph(buf, w, h, 4, graph_y, w as i32 - 8, graph_h,
            &self.tick_history, max_tick.max(1), 0xFF89DCEB, "Tick Activity");

        // Process count graph
        let proc_graph_y = graph_y + graph_h + 6;
        let proc_graph_h = h as i32 - proc_graph_y - 22;
        let max_proc = *self.proc_history.iter().max().unwrap_or(&1);
        let proc_u64: Vec<u64> = self.proc_history.iter().map(|&v| v as u64).collect();
        Self::draw_bar_graph(buf, w, h, 4, proc_graph_y, w as i32 - 8, proc_graph_h,
            &proc_u64, max_proc.max(1) as u64, 0xFFCBA6F7, "Process Count");

        // Status bar
        Self::fill(buf, w, h, 0, h as i32 - 18, w as i32, 18, 0xFF313244);
        let proc_count = self.proc_history[(self.tick_history_idx + 59) % 60];
        let status = alloc::format!("{} processes running", proc_count);
        Self::text(buf, w, h, 4, h as i32 - 15, &status, 0xFF6C7086);

        self.needs_redraw = false;
    }

    fn on_mouse_event(&mut self, _x: i32, _y: i32, _buttons: u8) {}
    fn on_key_event(&mut self, _c: char) {}
}
