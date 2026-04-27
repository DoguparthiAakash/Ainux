// =============================================================================
// Ainux Task Manager (TM) — Live System Monitor
// Functional re-implementation:
//   - ZERO heap allocations in the render loop (stack-only fmt buffers)
//   - TASKS lock released immediately after snapshot — no deadlock under pressure
//   - Dirty-flag rendering — no flicker, no wasted CPU
//   - Kill actually works (any non-PID-0 task)
//   - Process selection with arrow keys
//   - Survives exhausted heap and degraded memory
// =============================================================================

use crate::drivers::{video, keyboard};
use crate::process::scheduler::{TASKS, MAX_TASKS, get_ticks, kill_task};
use crate::process::task::TaskState;
use crate::mm::pmm::PMM;

// ── Tiny stack-only number formatter — no alloc ───────────────────────────────
fn u64_to_str(mut n: u64, buf: &mut [u8; 20]) -> &str {
    if n == 0 {
        buf[0] = b'0';
        return core::str::from_utf8(&buf[..1]).unwrap_or("0");
    }
    let mut i = 20usize;
    while n > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    core::str::from_utf8(&buf[i..]).unwrap_or("?")
}

// ── Minimal task snapshot — entirely stack-allocated ──────────────────────────
#[derive(Copy, Clone)]
struct Snap {
    id:        usize,
    state:     TaskState,
    priority:  u8,
    pages:     usize,
    cpu_ticks: u64,
}

impl Snap {
    const EMPTY: Self = Self { id: 0, state: TaskState::Free, priority: 0, pages: 0, cpu_ticks: 0 };
}

/// Grab a snapshot of all tasks while holding the lock for the shortest time possible.
/// The lock is dropped before any rendering happens — the key to deadlock-free operation.
fn snapshot() -> (usize, [Snap; MAX_TASKS]) {
    let mut arr = [Snap::EMPTY; MAX_TASKS];
    let mut count = 0usize;
    crate::cpu::without_interrupts(|| {
        let tasks = TASKS.lock();
        for i in 0..MAX_TASKS {
            if let Some(t) = &tasks[i] {
                if t.state == TaskState::Free { continue; }
                arr[count] = Snap {
                    id:        t.id,
                    state:     t.state,
                    priority:  t.priority,
                    pages:     t.page_count,
                    cpu_ticks: t.cpu_time_ticks,
                };
                count += 1;
            }
        }
    }); // ← lock released here, before any video calls
    (count, arr)
}

/// Read memory stats — O(1), lock released immediately, graceful if PMM is unavailable.
fn mem_stats() -> (usize, usize) {
    PMM.lock().as_ref().map(|p| p.get_stats_fast()).unwrap_or((0, 0))
}

// ── Render (all stack locals, no alloc) ───────────────────────────────────────
fn render(snaps: &[Snap; MAX_TASKS], count: usize, selected: usize) {
    let theme = video::THEME.lock();
    let a_col  = theme.accent;
    let bg_col = theme.bg;
    drop(theme);

    let box_bg = 0x002D2D2D;

    let (fb_w, fb_h) = video::get_resolution();
    let grid_w = fb_w / 8;
    let grid_h = fb_h / 12;

    // Main frame
    video::draw_tui_box(1, 1, grid_w - 2, grid_h - 2, a_col);

    // Title
    let title = " AINUX SYSTEM TASK MANAGER ";
    video::put_str_at((grid_w - title.len()) / 2, 1, title, 0xFFFFFF, a_col);

    // ── RAM bar (no alloc) ────────────────────────────────────────────────────
    let (used_frames, total_frames) = mem_stats();
    let total_mb = (total_frames * 4096) / 1024 / 1024;
    let used_mb  = (used_frames  * 4096) / 1024 / 1024;
    let ram_pct  = if total_frames > 0 { (used_frames * 100) / total_frames } else { 0 };

    video::put_str_at(3, 3, "Mem [", 0xAAAAAA, bg_col);
    // Draw bar char-by-char — no String needed
    for i in 0..20usize {
        let threshold = ram_pct * 20 / 100;
        let (ch, col) = if i < threshold {
            ('\u{2588}', if ram_pct > 80 { 0x00FF4444 } else { a_col })
        } else {
            ('\u{2591}', 0x00444444)
        };
        video::put_char_at(8 + i, 3, ch, col, bg_col);
    }
    // Print "29% 512MB/2048MB" — stack buf
    let mut b = [0u8; 20];
    video::put_str_at(29, 3, u64_to_str(ram_pct as u64, &mut b), a_col, bg_col);
    video::put_str_at(32, 3, "% ", 0xAAAAAA, bg_col);
    video::put_str_at(34, 3, u64_to_str(used_mb as u64, &mut b), a_col, bg_col);
    video::put_str_at(38, 3, "MB/", 0xAAAAAA, bg_col);
    video::put_str_at(41, 3, u64_to_str(total_mb as u64, &mut b), a_col, bg_col);
    video::put_str_at(46, 3, "MB]", 0xAAAAAA, bg_col);

    // CPU core count
    let cpu_count = crate::cpu::percpu::get_cpu_count();
    video::put_str_at(3, 4, "CPUs: ", 0xAAAAAA, bg_col);
    video::put_str_at(9, 4, u64_to_str(cpu_count as u64, &mut b), a_col, bg_col);
    video::put_str_at(11, 4, "  Ticks: ", 0xAAAAAA, bg_col);
    video::put_str_at(20, 4, u64_to_str(get_ticks(), &mut b), a_col, bg_col);
    video::put_str_at(30, 4, "  Tasks: ", 0xAAAAAA, bg_col);
    video::put_str_at(39, 4, u64_to_str(count as u64, &mut b), a_col, bg_col);

    // ── Column headers ────────────────────────────────────────────────────────
    let row_y = 6usize;
    video::draw_rect_grid(2, row_y, grid_w - 4, 1, 0xFFFFFF, a_col);
    video::put_str_at( 3, row_y, "PID",      0x000000, a_col);
    video::put_str_at( 8, row_y, "STATE",    0x000000, a_col);
    video::put_str_at(18, row_y, "PRI",      0x000000, a_col);
    video::put_str_at(22, row_y, "MEM(KB)",  0x000000, a_col);
    video::put_str_at(32, row_y, "CPU TICKS",0x000000, a_col);
    video::put_str_at(44, row_y, "NAME",     0x000000, a_col);

    // ── Process rows ──────────────────────────────────────────────────────────
    let max_rows = (grid_h - 4).saturating_sub(row_y + 1);
    let scroll   = if selected >= max_rows { selected - max_rows + 1 } else { 0 };

    for row in 0..max_rows {
        let idx = scroll + row;
        let y   = row_y + 1 + row;
        if idx >= count {
            video::draw_rect_grid(2, y, grid_w - 4, 1, bg_col, bg_col);
            continue;
        }

        let s  = &snaps[idx];
        let is_sel = idx == selected;
        let (fg, bg) = if is_sel { (0x000000u32, a_col) } else { (0xFFFFFF, bg_col) };

        video::draw_rect_grid(2, y, grid_w - 4, 1, fg, bg);

        // PID
        video::put_str_at(3, y, u64_to_str(s.id as u64, &mut b), fg, bg);

        // State
        let (state_str, state_col) = match s.state {
            TaskState::Running => ("RUNNING",  if is_sel { fg } else { 0x00FF88 }),
            TaskState::Ready   => ("READY",    fg),
            TaskState::Waiting => ("SLEEPING", fg),
            TaskState::Zombie  => ("ZOMBIE",   if is_sel { fg } else { 0xFF4444 }),
            TaskState::Free    => ("FREE",     fg),
        };
        video::put_str_at(8, y, state_str, state_col, bg);

        // Priority
        video::put_str_at(18, y, u64_to_str(s.priority as u64, &mut b), fg, bg);

        // Memory in KB (pages × 4)
        video::put_str_at(22, y, u64_to_str((s.pages * 4) as u64, &mut b), fg, bg);

        // CPU ticks
        video::put_str_at(32, y, u64_to_str(s.cpu_ticks, &mut b), fg, bg);

        // Name (static lookup — no alloc)
        let name = match s.id {
            0 => "kernel",
            1 => "shell",
            _ => "task",
        };
        video::put_str_at(44, y, name, fg, bg);
    }

    // ── Footer ────────────────────────────────────────────────────────────────
    video::put_str_at(3, grid_h - 3,
        " [↑↓] Select  [Del] Kill  [Q/ESC] Exit ",
        a_col, box_bg);
}

// ── Main entry ────────────────────────────────────────────────────────────────
pub fn main(args: &[&str]) {
    cmd_taskman(args);
}

pub fn cmd_taskman(_args: &[&str]) {
    video::clear();

    let mut selected = 0usize;
    let mut dirty    = true;

    // Pre-allocate snapshot on stack — never touches heap
    let mut count = 0usize;
    let mut snaps = [Snap::EMPTY; MAX_TASKS];

    loop {
        if dirty {
            // Snapshot — lock held only for the copy, released before rendering
            let (c, s) = snapshot();
            count = c;
            snaps = s;
            render(&snaps, count, selected);
            dirty = false;
        }

        // Input
        if let Some(ch) = keyboard::pop_char() {
            dirty = true;
            match ch {
                '\u{2191}' => { if selected > 0 { selected -= 1; } }
                '\u{2193}' => { if selected + 1 < count { selected += 1; } }

                // Del or Backspace — kill selected task (never PID 0)
                '\x7F' | '\x08' => {
                    if count > 0 {
                        let pid = snaps[selected].id;
                        if pid != 0 {
                            kill_task(pid);
                            // Keep selection valid
                            if selected > 0 { selected -= 1; }
                        }
                    }
                }

                'q' | 'Q' | '\x1B' => {
                    video::clear();
                    return;
                }

                _ => { dirty = false; }
            }
        }

        crate::net::poll();
        unsafe { core::arch::asm!("hlt"); }
    }
}
