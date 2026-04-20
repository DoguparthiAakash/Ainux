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
const ACCENT_COLOR: u32 = 0x0000AAFF; // Metro Blue

pub fn main() {
    let mut last_cycles: [u64; MAX_TASKS] = [0; MAX_TASKS];
    let mut last_sample_time = crate::cpu::cpuid::rdtsc();

    video::clear();
    
    loop {
        draw_header();
        
        let current_time = crate::cpu::cpuid::rdtsc();
        let time_delta = current_time.wrapping_sub(last_sample_time);
        
        {
            let lock = TASKS.lock();
            if let Some(tasks) = lock.as_ref() {
                let mut row = 4;
                draw_table_header();
                for i in 0..MAX_TASKS {
                    let task = &tasks[i];
                    if task.state != TaskState::Free {
                        // Calculate CPU %
                        let cycle_delta = task.total_cycles.wrapping_sub(last_cycles[i]);
                        let cpu_usage = if time_delta > 0 {
                            (cycle_delta * 100) / time_delta
                        } else {
                            0
                        };
                        
                        // Update baseline
                        last_cycles[i] = task.total_cycles;

                        // Draw Row
                        draw_task_row(row, task, cpu_usage as u32);
                        row += 1;
                    }
                }
            }
        }
        last_sample_time = current_time;

        draw_footer();

        // Check for exit
        if crate::process::scheduler::check_current_signal(crate::process::task::SIGINT) {
            video::clear();
            return;
        }

        if let Some(ch) = keyboard::pop_char() {
            if ch == 'q' || ch == '\x1B' || ch == '\x03' {
                video::clear();
                return;
            }
            if ch == 'k' {
                 // kill logic stub
            }
        }

        // Reduced refresh rate for stability
        for _ in 0..10 {
            unsafe { core::arch::asm!("hlt"); }
        }
    }
}

fn draw_header() {
    video::draw_rect_grid(0, 0, 80, 1, TEXT_COLOR, HEADER_BG);
    video::put_str_at(2, 0, " AINUX METUS ", TEXT_COLOR, ACCENT_COLOR);
    video::put_str_at(16, 0, "|  Industrial Maturation Layer  |  v1.0  ", TEXT_COLOR, HEADER_BG);
    
    // Uptime / Global Stats
    let ticks = crate::process::scheduler::get_ticks();
    let uptime_str = format!("UPTIME: {}s", ticks / 100);
    video::put_str_at(60, 0, &uptime_str, TEXT_COLOR, HEADER_BG);
}

fn draw_table_header() {
    let y = 2;
    video::draw_rect_grid(0, y, 80, 1, 0x00AAAAAA, 0x00333333);
    video::put_str_at(2,  y, "PID", 0x00FFFF00, 0x00333333);
    video::put_str_at(8,  y, "STATE", 0x00FFFF00, 0x00333333);
    video::put_str_at(18, y, "PAGES", 0x00FFFF00, 0x00333333);
    video::put_str_at(28, y, "MEM (KB)", 0x00FFFF00, 0x00333333);
    video::put_str_at(40, y, "SYSCALLS", 0x00FFFF00, 0x00333333);
    video::put_str_at(52, y, "CPU %", 0x00FFFF00, 0x00333333);
    video::put_str_at(62, y, "SIGNALS", 0x00FFFF00, 0x00333333);
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
    let color = if task.state == TaskState::Running { 0x0000FF00 } else { TEXT_COLOR };

    video::put_str_at(2,  y, &format!("{}", task.id), color, BG_COLOR);
    video::put_str_at(8,  y, state_str, color, BG_COLOR);
    video::put_str_at(18, y, &format!("{}", task.page_count), color, BG_COLOR);
    video::put_str_at(28, y, &format!("{} KB", mem_kb), color, BG_COLOR);
    video::put_str_at(40, y, &format!("{}", task.syscall_count), color, BG_COLOR);
    
    // CPU Graph-ish
    let cpu_str = format!("{:>3}%", cpu);
    video::put_str_at(52, y, &cpu_str, if cpu > 50 { 0x00FF0000 } else { color }, BG_COLOR);
    
    video::put_str_at(62, y, &format!("{:#010x}", task.signals), color, BG_COLOR);
}

fn draw_footer() {
    video::draw_rect_grid(0, 24, 80, 1, TEXT_COLOR, 0x00444444);
    video::put_str_at(2, 24, "[Q] Exit  [K] Kill  [M] Metering Details", TEXT_COLOR, 0x00444444);
}
