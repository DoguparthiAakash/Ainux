// =============================================================================
// Ainux Metus — Industrial System & Resource Monitor
// Part of Operation Maturitas: Maturing & Metering
// =============================================================================

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use crate::drivers::video::{self, THEME};
use crate::drivers::keyboard;
use crate::process::scheduler::{TASKS, MAX_TASKS};
use crate::process::task::TaskState;

const BG_COLOR: u32 = 0x00000022; // Deep Dark
const HEADER_BG: u32 = 0x00444444;
const TEXT_COLOR: u32 = 0xFFFFFFFF;
const ACCENT_COLOR: u32 = 0x0000FF99; // Cyber Green

pub fn main() {
    let mut last_cycles: [u64; MAX_TASKS] = [0; MAX_TASKS];
    let mut last_sample_time = crate::cpu::cpuid::rdtsc();

    video::clear();
    
    loop {
        draw_header();
        
        let current_time = crate::cpu::cpuid::rdtsc();
        let time_delta = current_time.wrapping_sub(last_sample_time);
        
        let tasks = TASKS.lock();
        let mut row = 4;
        
        draw_table_header();

        // Pass 1: Calculate total cycles elapsed across all tasks to avoid VirtualBox rdtsc/hlt scaling issues
        let mut sum_cycle_delta: u64 = 0;
        let mut deltas: [u64; MAX_TASKS] = [0; MAX_TASKS];
        for i in 0..MAX_TASKS {
            if let Some(task) = &tasks[i] {
                deltas[i] = task.total_cycles.wrapping_sub(last_cycles[i]);
                sum_cycle_delta = sum_cycle_delta.wrapping_add(deltas[i]);
                last_cycles[i] = task.total_cycles;
            }
        }

        // Pass 2: Draw tasks
        for i in 0..MAX_TASKS {
            if let Some(task) = &tasks[i] {
                let cpu_usage = if sum_cycle_delta > 0 {
                    (deltas[i] * 100) / sum_cycle_delta
                } else {
                    0
                };
                
                // Draw Row
                draw_task_row(row, task, cpu_usage as u32);
                row += 1;
            }
        }
        
        last_sample_time = current_time;
        drop(tasks);

        draw_footer();

        // Check for exit
        if crate::process::scheduler::check_current_signal(crate::process::task::SIGINT) {
            video::clear();
            return;
        }

        if let Some(ch) = keyboard::pop_char() {
            if ch == 'q' || ch == 'Q' || ch == '\x1B' || ch == '\x03' {
                video::clear();
                return;
            }
            if ch == 'k' || ch == 'K' {
                prompt_and_kill();
            }
            if ch == 'm' || ch == 'M' {
                show_metering_details();
            }
        }

        // Reduced refresh rate for stability
        for _ in 0..10 {
            unsafe { core::arch::asm!("hlt"); }
        }
    }
}

fn draw_header() {
    video::draw_tui_title_box(0, 0, 80, 24, "", 0x00555555); // Outer border
    video::draw_rect_grid(1, 1, 78, 1, TEXT_COLOR, HEADER_BG);
    video::put_str_at(2, 1, " AINUX METUS - TASK MANAGER ", TEXT_COLOR, ACCENT_COLOR);
    
    // RAM Status
    let (used_frames, total_frames) = {
        let pmm_lock = crate::mm::pmm::PMM.lock();
        if let Some(pmm) = pmm_lock.as_ref() {
            pmm.get_stats_fast()
        } else {
            (0, 0)
        }
    };
    let used_mb = (used_frames * 4) / 1024;
    let total_mb = (total_frames * 4) / 1024;
    let mem_str = format!("RAM: {}MB/{}MB", used_mb, total_mb);
    video::put_str_at(35, 1, &format!("{:<18}", mem_str), TEXT_COLOR, HEADER_BG);
    
    // Uptime / Global Stats
    let ticks = crate::process::scheduler::get_ticks();
    let uptime_str = format!("UPTIME: {}s", ticks / 100);
    video::put_str_at(60, 1, &uptime_str, TEXT_COLOR, HEADER_BG);
}

fn draw_table_header() {
    let y = 3;
    video::draw_rect_grid(1, y, 78, 1, 0x00AAAAAA, 0x00333333);
    video::put_str_at(2,  y, "PID", 0x00FFFF00, 0x00333333);
    video::put_str_at(7,  y, "COMMAND", 0x00FFFF00, 0x00333333);
    video::put_str_at(22, y, "STATE", 0x00FFFF00, 0x00333333);
    video::put_str_at(32, y, "MEM", 0x00FFFF00, 0x00333333);
    video::put_str_at(42, y, "SYSCALLS", 0x00FFFF00, 0x00333333);
    video::put_str_at(52, y, "CPU %", 0x00FFFF00, 0x00333333);
    video::put_str_at(60, y, "CPU BAR", 0x00FFFF00, 0x00333333);
}

fn draw_task_row(y: usize, task: &crate::process::task::Task, cpu: u32) {
    let state_str = match task.state {
        TaskState::Running => "RUNNING",
        TaskState::Ready   => "READY",
        TaskState::Waiting => "WAITING",
        TaskState::Zombie  => "ZOMBIE",
        TaskState::Free    => "FREE",
    };

    let mem_kb = task.page_count * 4;
    let mem_str = if mem_kb > 1024 {
        format!("{}.{} MB", mem_kb / 1024, (mem_kb % 1024) / 102)
    } else {
        format!("{} KB", mem_kb)
    };

    let color = if task.state == TaskState::Running { 0x0000FF00 } else { TEXT_COLOR };

    video::put_str_at(2,  y, &format!("{:<4}", task.id), color, BG_COLOR);
    video::put_str_at(7,  y, &format!("{:<14}", if task.name.len() > 14 { &task.name[..14] } else { &task.name }), color, BG_COLOR);
    video::put_str_at(22, y, &format!("{:<8}", state_str), color, BG_COLOR);
    video::put_str_at(32, y, &format!("{:<8}", mem_str), color, BG_COLOR);
    video::put_str_at(42, y, &format!("{:<8}", task.syscall_count), color, BG_COLOR);
    
    // CPU Graph-ish
    let cpu_str = format!("{:>3}%", cpu);
    video::put_str_at(52, y, &cpu_str, if cpu > 50 { 0x00FF0000 } else { color }, BG_COLOR);
    
    // CPU Bar (15 chars wide)
    let bar_len = (cpu * 15 / 100).min(15) as usize;
    let mut bar_str = String::new();
    for _ in 0..bar_len { bar_str.push('#'); }
    for _ in bar_len..15 { bar_str.push('.'); }
    video::put_str_at(60, y, &format!("[{}]", bar_str), if cpu > 75 { 0x00FF0000 } else if cpu > 25 { 0x00FFAA00 } else { 0x0000FF00 }, BG_COLOR);
}

fn draw_footer() {
    video::draw_rect_grid(1, 23, 78, 1, TEXT_COLOR, 0x00444444);
    video::put_str_at(2, 23, " [Q] Exit  |  [K] Kill Task  |  [M] Metering Details ", TEXT_COLOR, 0x00444444);
}

fn prompt_and_kill() {
    video::draw_tui_title_box(20, 10, 40, 5, " KILL TASK ", 0x00FF0000);
    video::put_str_at(22, 12, "Enter PID to kill: ", TEXT_COLOR, BG_COLOR);
    
    let mut pid_str = String::new();
    loop {
        video::put_str_at(41, 12, &format!("{:<10}", pid_str), TEXT_COLOR, BG_COLOR);
        let ch = keyboard::get_char();
        if ch == '\n' || ch == '\r' {
            break;
        } else if ch == '\x08' || ch == '\x7F' { // Backspace
            pid_str.pop();
        } else if ch.is_ascii_digit() && pid_str.len() < 8 {
            pid_str.push(ch);
        } else if ch == '\x1B' || ch == '\x03' {
            return; // Esc to cancel
        }
    }
    
    if let Ok(pid) = pid_str.parse::<usize>() {
        crate::process::scheduler::kill_task(pid);
        video::put_str_at(22, 13, &format!("Sent SIGKILL to {}", pid), 0x0000FF00, BG_COLOR);
    } else {
        video::put_str_at(22, 13, "Invalid PID format.", 0x00FF0000, BG_COLOR);
    }
    
    // Wait briefly before closing prompt
    let start = crate::cpu::cpuid::rdtsc();
    while crate::cpu::cpuid::rdtsc() - start < 1_000_000_000 {}
}

fn show_metering_details() {
    video::draw_tui_title_box(10, 5, 60, 15, " METERING DETAILS ", ACCENT_COLOR);
    
    let (used_frames, total_frames) = {
        let pmm_lock = crate::mm::pmm::PMM.lock();
        if let Some(pmm) = pmm_lock.as_ref() {
            pmm.get_stats_fast()
        } else {
            (0, 0)
        }
    };
    let total_kb = total_frames * 4;
    let used_kb = used_frames * 4;
    
    video::put_str_at(12, 7, "System Memory Diagnostics", 0x00FFFF00, BG_COLOR);
    video::put_str_at(12, 9, &format!("Total Physical RAM: {} KB", total_kb), TEXT_COLOR, BG_COLOR);
    video::put_str_at(12, 10, &format!("Allocated RAM:      {} KB", used_kb), TEXT_COLOR, BG_COLOR);
    video::put_str_at(12, 11, &format!("Free RAM:           {} KB", total_kb - used_kb), TEXT_COLOR, BG_COLOR);
    
    video::put_str_at(12, 13, "Scheduler Diagnostics", 0x00FFFF00, BG_COLOR);
    let tasks = TASKS.lock();
    let mut active = 0;
    for i in 0..MAX_TASKS {
        if let Some(t) = &tasks[i] {
            if t.state != TaskState::Free {
                active += 1;
            }
        }
    }
    drop(tasks);
    video::put_str_at(12, 15, &format!("Total Task Slots:   {}", MAX_TASKS), TEXT_COLOR, BG_COLOR);
    video::put_str_at(12, 16, &format!("Active Tasks:       {}", active), TEXT_COLOR, BG_COLOR);
    
    video::put_str_at(12, 18, "Press any key to close...", 0x00AAAAAA, BG_COLOR);
    
    // Wait for any key
    keyboard::get_char();
}
