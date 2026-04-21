// =============================================================================
// Ainux Task Manager (TM) — Live System Monitor
// =============================================================================

extern crate alloc;
use alloc::string::String;
use alloc::format;
use crate::drivers::{video, keyboard};
use crate::process::scheduler::{TASKS, get_ticks};
use crate::process::task::TaskState;
use crate::mm::pmm::PMM;

pub fn main(_args: &[&str]) {
    // Hide cursor for immersion
    video::clear();
    
    loop {
        render_task_manager();
        
        // Wait or check for exit
        for _ in 0..10 {
            if let Some(c) = keyboard::pop_char() {
                if c == 'q' || c == '\x1B' { // 'q' or ESC
                    video::clear();
                    return;
                }
            }
            // Sleep 100ms
            unsafe { 
                let end = get_ticks() + 10;
                while get_ticks() < end {
                    core::arch::asm!("pause");
                    crate::process::scheduler::yield_now();
                }
            }
        }
    }
}

fn render_task_manager() {
    let theme = video::THEME.lock();
    let a_col = theme.accent;
    let bg_col = theme.bg;
    let box_bg = 0x002D2D2D;
    drop(theme);

    let (fb_w, fb_h) = video::get_resolution();
    // Grid dimensions
    let grid_w = fb_w / 8;
    let grid_h = fb_h / 12;

    // Draw Main Frame
    video::draw_tui_box(1, 1, grid_w - 2, grid_h - 2, a_col);
    
    // Header
    let title = " AINUX SYSTEM TASK MANAGER - V0.1 ";
    video::put_str_at((grid_w - title.len()) / 2, 1, title, 0xFFFFFF, a_col);

    // RAM Stats
    let (used_frames, total_frames) = {
        let pmm = PMM.lock();
        if let Some(pmm) = pmm.as_ref() {
            pmm.get_stats_fast()
        } else { (0, 0) }
    };
    let total_mb = (total_frames * 4096) / 1024 / 1024;
    let used_mb = (used_frames * 4096) / 1024 / 1024;
    let ram_pct = (used_frames * 100) / total_frames.max(1);

    video::put_str_at(3, 3, "Physical Memory Usage:", 0xAAAAAA, bg_col);
    let mut ram_bar = String::from("[");
    let filled = (ram_pct / 5) as usize; // 20 dots
    for i in 0..20 {
        if i < filled { ram_bar.push_str("█"); }
        else { ram_bar.push_str("░"); }
    }
    ram_bar.push_str(&format!("] {}% ({}MB / {}MB)", ram_pct, used_mb, total_mb));
    video::put_str_at(28, 3, &ram_bar, a_col, bg_col);

    // CPU Info (Simplified)
    let cpu_count = crate::cpu::percpu::get_cpu_count();
    video::put_str_at(3, 4, &format!("Processing Units:  {} Active Cores", cpu_count), 0xAAAAAA, bg_col);

    // Table Header
    let cols = [
        ("PID", 5),
        ("STATE", 10),
        ("PRIO", 6),
        ("MEMORY", 12),
        ("CPU TIME", 12),
        ("COMMAND/NAME", 20),
    ];

    let mut cur_x = 3;
    let row_y = 6;
    video::draw_rect_grid(2, row_y, grid_w - 4, 1, 0xFFFFFF, a_col);
    for (label, width) in cols {
        video::put_str_at(cur_x, row_y, label, 0x000000, a_col);
        cur_x += width;
    }

    // Task List
    let tasks = TASKS.lock();
    let mut count = 0;
    for i in 0..crate::process::scheduler::MAX_TASKS {
        if let Some(task) = &tasks[i] {
            if task.state == crate::process::task::TaskState::Free { continue; }
            
            let y = row_y + 1 + count;
            if y >= grid_h - 4 { break; }

            let state_str = match task.state {
                TaskState::Running => "RUNNING",
                TaskState::Ready => "READY",
                TaskState::Waiting => "SLEEPING",
                TaskState::Zombie => "ZOMBIE",
                _ => "UNK",
            };

            let mem_str = format!("{} Pgs", task.page_count);
            let cpu_str = format!("{} Tks", task.cpu_time_ticks);
            let name = if i == 0 { "Kernel" } else if i == 1 { "Shell" } else { "Task" };

            video::put_str_at(3, y, &format!("{}", task.id), 0xFFFFFF, bg_col);
            video::put_str_at(8, y, state_str, if task.state == TaskState::Running { 0x00FF00 } else { 0xCCCCCC }, bg_col);
            video::put_str_at(18, y, &format!("{}", task.priority), 0xAAAAAA, bg_col);
            video::put_str_at(24, y, &mem_str, 0x00AAAA, bg_col);
            video::put_str_at(36, y, &cpu_str, 0xAAAA00, bg_col);
            video::put_str_at(48, y, name, 0xFFFFFF, bg_col);

            count += 1;
        }
    }

    // Footer
    video::put_str_at(3, grid_h - 3, " [Q] EXIT  [K] KILL (STUB)  [TAB] REFRESH ", a_col, box_bg);
}
