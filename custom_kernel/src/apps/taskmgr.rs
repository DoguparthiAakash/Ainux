extern crate alloc;
use crate::drivers::{video, keyboard};
use crate::process::scheduler::{TASKS, MAX_TASKS, get_ticks};
use crate::process::task::TaskState;
use crate::mm::pmm::PMM;

// ── Palette (Windows NT 3.51 / Win98 TM inspired) ──────────────────────────
const C_WINDOW_BG:    u32 = 0x00C0C0C0; // Classic silver-grey
const C_TITLE_BG:     u32 = 0x00000080; // Win98 active title bar blue
const C_TITLE_FG:     u32 = 0x00FFFFFF; // White title text
const C_HEADER_BG:    u32 = 0x00808080; // Column header grey
const C_HEADER_FG:    u32 = 0x00FFFFFF; // Header text
const C_ROW_BG:       u32 = 0x00DCDCDC; // Row background
const C_ROW_FG:       u32 = 0x00000000; // Row text black
const C_SEL_BG:       u32 = 0x00000080; // Selected row blue
const C_SEL_FG:       u32 = 0x00FFFFFF; // Selected row text
const C_TAB_ACTIVE:   u32 = 0x00FFFFFF; // Active tab white
const C_TAB_INACTIVE: u32 = 0x00B0B0B0; // Inactive tab grey
const C_TAB_TEXT:     u32 = 0x00000000; // Tab text
const C_BAR_FILLED:   u32 = 0x000000AA; // CPU bar filled
const C_BAR_EMPTY:    u32 = 0x00808080; // CPU bar empty
const C_STATUS_BG:    u32 = 0x00C0C0C0; // Status bar bg
const C_STATUS_FG:    u32 = 0x00000000; // Status bar text
const C_WARN_FG:      u32 = 0x00AA0000; // Warning red
const C_OK_FG:        u32 = 0x00006600; // OK green
const C_BORDER_DARK:  u32 = 0x00606060; // Dark border (3D effect)
const C_BORDER_LIGHT: u32 = 0x00FFFFFF; // Light border (3D effect)

#[derive(Clone, Copy, PartialEq)]
enum Tab { Processes, Performance, Kernel }

/// Snapshot of one task — all stack-allocated, no alloc needed
#[derive(Clone, Copy)]
struct TaskSnap {
    id:          usize,
    state:       TaskState,
    cpu_ticks:   u64,
    total_cycles: u64,
    page_count:  usize,
    priority:    u8,
    syscall_cnt: u64,
    canary_ok:   bool,
    room_id:     usize,
}

impl TaskSnap {
    const fn empty() -> Self {
        Self {
            id: 0, state: TaskState::Free, cpu_ticks: 0,
            total_cycles: 0, page_count: 0, priority: 0,
            syscall_cnt: 0, canary_ok: true, room_id: 0,
        }
    }
}

/// Collect task snapshots without any heap allocation.
/// Returns (count, array). Uses a fixed 32-entry stack array.
fn collect_snapshots() -> (usize, [TaskSnap; MAX_TASKS]) {
    let mut snaps = [TaskSnap::empty(); MAX_TASKS];
    let mut count = 0;

    // We disable interrupts only for the duration of the lock acquisition
    crate::cpu::without_interrupts(|| {
        let tasks = TASKS.lock();
        for i in 0..MAX_TASKS {
            if let Some(t) = &tasks[i] {
                if t.state == TaskState::Free { continue; }
                snaps[count] = TaskSnap {
                    id:           t.id,
                    state:        t.state,
                    cpu_ticks:    t.cpu_time_ticks,
                    total_cycles: t.total_cycles,
                    page_count:   t.page_count,
                    priority:     t.priority,
                    syscall_cnt:  t.syscall_count,
                    canary_ok:    t.canary == crate::process::task::STACK_CANARY_MAGIC,
                    room_id:      t.room.id,
                };
                count += 1;
            }
        }
    });

    (count, snaps)
}

// ── Small number formatting — no alloc, writes into a stack buffer ───────────
fn fmt_u64(n: u64, buf: &mut [u8; 20]) -> &str {
    if n == 0 {
        buf[0] = b'0';
        return core::str::from_utf8(&buf[..1]).unwrap_or("0");
    }
    let mut i = 20usize;
    let mut v = n;
    while v > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    core::str::from_utf8(&buf[i..]).unwrap_or("?")
}

fn fmt_kb(pages: usize, buf: &mut [u8; 20]) -> &str {
    fmt_u64((pages as u64) * 4, buf) // 4 KB per page
}

// ── Draw helpers ─────────────────────────────────────────────────────────────

fn fill(x: usize, y: usize, w: usize, h: usize, bg: u32) {
    video::draw_rect_grid(x, y, w, h, bg, bg);
}

fn text(x: usize, y: usize, s: &str, fg: u32, bg: u32) {
    video::put_str_at(x, y, s, fg, bg);
}

/// Draw a Win9x style raised 3D border around a rect
fn border_3d(x: usize, y: usize, w: usize, h: usize) {
    // Top + Left: light
    video::put_str_at(x, y, &" ".repeat(w), C_BORDER_LIGHT, C_BORDER_LIGHT);
    for row in y..y+h { video::put_char_at(x, row, ' ', C_BORDER_LIGHT, C_BORDER_LIGHT); }
    // Bottom + Right: dark
    video::put_str_at(x, y + h - 1, &" ".repeat(w), C_BORDER_DARK, C_BORDER_DARK);
    for row in y..y+h { video::put_char_at(x + w - 1, row, ' ', C_BORDER_DARK, C_BORDER_DARK); }
}

/// Draw a horizontal bar (0..=100 percent)
fn cpu_bar(x: usize, y: usize, w: usize, pct: u64) {
    let filled = ((pct.min(100) * w as u64) / 100) as usize;
    for i in 0..w {
        let ch = if i < filled { '\u{2588}' } else { '\u{2591}' };
        let col = if i < filled { C_BAR_FILLED } else { C_BAR_EMPTY };
        video::put_char_at(x + i, y, ch, col, C_WINDOW_BG);
    }
}

// ── Title bar ────────────────────────────────────────────────────────────────
fn draw_title(w: usize) {
    fill(0, 0, w, 1, C_TITLE_BG);
    text(2, 0, "Ainux Task Manager", C_TITLE_FG, C_TITLE_BG);
    text(w - 4, 0, "[X]", C_TITLE_FG, C_TITLE_BG);
}

// ── Tab bar ──────────────────────────────────────────────────────────────────
fn draw_tabs(w: usize, active: Tab) {
    fill(0, 1, w, 1, C_HEADER_BG);
    let tabs = [
        (0usize,  " Processes  ", Tab::Processes),
        (14usize, " Performance", Tab::Performance),
        (27usize, " Kernel     ", Tab::Kernel),
    ];
    for (off, label, tab) in &tabs {
        let (fg, bg) = if *tab == active {
            (C_TAB_TEXT, C_TAB_ACTIVE)
        } else {
            (C_TAB_TEXT, C_TAB_INACTIVE)
        };
        text(1 + off, 1, label, fg, bg);
    }
    text(40, 1, " F5:Refresh | Del:Kill | ESC:Exit ", C_HEADER_FG, C_HEADER_BG);
}

// ── Status bar ───────────────────────────────────────────────────────────────
fn draw_status(w: usize, proc_count: usize, free_pages: usize, total_pages: usize) {
    fill(0, 24, w, 1, C_STATUS_BG);
    let mut buf = [0u8; 20];
    // Processes count
    text(0, 24, " Processes: ", C_STATUS_FG, C_STATUS_BG);
    text(12, 24, fmt_u64(proc_count as u64, &mut buf), C_STATUS_FG, C_STATUS_BG);
    // Free memory
    let free_kb = (free_pages as u64) * 4;
    let total_kb = (total_pages as u64) * 4;
    text(17, 24, " | Free Mem:", C_STATUS_FG, C_STATUS_BG);
    text(29, 24, fmt_u64(free_kb, &mut buf), C_STATUS_FG, C_STATUS_BG);
    text(35, 24, "KB/", C_STATUS_FG, C_STATUS_BG);
    text(38, 24, fmt_u64(total_kb, &mut buf), C_STATUS_FG, C_STATUS_BG);
    text(45, 24, "KB", C_STATUS_FG, C_STATUS_BG);
}

// ── PROCESSES tab ─────────────────────────────────────────────────────────────
fn draw_processes(snaps: &[TaskSnap; MAX_TASKS], count: usize, selected: usize) {
    const W: usize = 80;

    // Column headers
    fill(0, 2, W, 1, C_HEADER_BG);
    text( 0, 2, " PID ", C_HEADER_FG, C_HEADER_BG);
    text( 5, 2, " State   ", C_HEADER_FG, C_HEADER_BG);
    text(14, 2, " Room ", C_HEADER_FG, C_HEADER_BG);
    text(20, 2, " Pri ", C_HEADER_FG, C_HEADER_BG);
    text(25, 2, " CPU Ticks     ", C_HEADER_FG, C_HEADER_BG);
    text(40, 2, " Mem (KB)  ", C_HEADER_FG, C_HEADER_BG);
    text(51, 2, " Syscalls  ", C_HEADER_FG, C_HEADER_BG);
    text(62, 2, " Canary  ", C_HEADER_FG, C_HEADER_BG);

    // Row area (rows 3..23)
    fill(0, 3, W, 21, C_ROW_BG, );

    let max_visible = 21usize;
    let scroll_off = if selected >= max_visible { selected - max_visible + 1 } else { 0 };

    for row in 0..max_visible {
        let idx = scroll_off + row;
        let y = 3 + row;
        if idx >= count {
            fill(0, y, W, 1, C_ROW_BG);
            continue;
        }
        let s = &snaps[idx];
        let (fg, bg) = if idx == selected { (C_SEL_FG, C_SEL_BG) } else { (C_ROW_FG, C_ROW_BG) };

        fill(0, y, W, 1, bg);

        let mut buf = [0u8; 20];

        // PID
        text(1, y, fmt_u64(s.id as u64, &mut buf), fg, bg);

        // State
        let state_str = match s.state {
            TaskState::Running => "Running",
            TaskState::Ready   => "Ready  ",
            TaskState::Waiting => "Waiting",
            TaskState::Zombie  => "Zombie ",
            TaskState::Free    => "Free   ",
        };
        let state_fg = match s.state {
            TaskState::Running => if idx == selected { C_SEL_FG } else { C_OK_FG },
            TaskState::Zombie  => if idx == selected { C_SEL_FG } else { C_WARN_FG },
            _ => fg,
        };
        text(5, y, state_str, state_fg, bg);

        // Room
        text(14, y, fmt_u64(s.room_id as u64, &mut buf), fg, bg);

        // Priority
        text(20, y, fmt_u64(s.priority as u64, &mut buf), fg, bg);

        // CPU Ticks
        text(25, y, fmt_u64(s.cpu_ticks, &mut buf), fg, bg);

        // Memory KB
        text(40, y, fmt_kb(s.page_count, &mut buf), fg, bg);

        // Syscall count
        text(51, y, fmt_u64(s.syscall_cnt, &mut buf), fg, bg);

        // Canary health
        let (canary_str, canary_fg) = if s.canary_ok {
            ("OK     ", if idx == selected { C_SEL_FG } else { C_OK_FG })
        } else {
            ("CORRUPT", C_WARN_FG)
        };
        text(62, y, canary_str, canary_fg, bg);
    }
}

// ── PERFORMANCE tab ───────────────────────────────────────────────────────────
fn draw_performance(snaps: &[TaskSnap; MAX_TASKS], count: usize,
                    free_pages: usize, total_pages: usize)
{
    const W: usize = 80;
    fill(0, 2, W, 22, C_WINDOW_BG);

    let mut buf = [0u8; 20];
    let ticks = get_ticks();

    // --- CPU Usage graph ---
    text(2, 3, "CPU Usage History", C_ROW_FG, C_WINDOW_BG);

    // Compute total CPU ticks from all running/ready tasks
    let mut total_cpu: u64 = 0;
    for i in 0..count {
        total_cpu += snaps[i].cpu_ticks;
    }
    let cpu_pct = if ticks > 0 { (total_cpu * 100 / ticks.max(1)).min(100) } else { 0 };

    // Bar graph — 60 chars wide
    text(2, 4, "[", C_ROW_FG, C_WINDOW_BG);
    cpu_bar(3, 4, 60, cpu_pct);
    text(63, 4, "]", C_ROW_FG, C_WINDOW_BG);
    text(65, 4, fmt_u64(cpu_pct, &mut buf), C_ROW_FG, C_WINDOW_BG);
    text(68, 4, "%", C_ROW_FG, C_WINDOW_BG);

    // --- Memory stats ---
    text(2, 6, "Physical Memory (KB):", C_ROW_FG, C_WINDOW_BG);
    let total_kb = (total_pages as u64) * 4;
    let free_kb  = (free_pages  as u64) * 4;
    let used_kb  = total_kb.saturating_sub(free_kb);

    text(2,  7, "  Total:  ", C_ROW_FG, C_WINDOW_BG); text(12, 7, fmt_u64(total_kb, &mut buf), C_ROW_FG, C_WINDOW_BG);
    text(2,  8, "  Used:   ", C_WARN_FG, C_WINDOW_BG); text(12, 8, fmt_u64(used_kb, &mut buf),  C_WARN_FG, C_WINDOW_BG);
    text(2,  9, "  Free:   ", C_OK_FG,   C_WINDOW_BG); text(12, 9, fmt_u64(free_kb, &mut buf),  C_OK_FG,   C_WINDOW_BG);

    // Memory usage bar
    let mem_pct = if total_pages > 0 { ((total_pages - free_pages) * 100 / total_pages) as u64 } else { 0 };
    text(2, 10, "Mem [", C_ROW_FG, C_WINDOW_BG);
    cpu_bar(7, 10, 58, mem_pct);
    text(65, 10, "]", C_ROW_FG, C_WINDOW_BG);
    text(67, 10, fmt_u64(mem_pct, &mut buf), C_ROW_FG, C_WINDOW_BG);
    text(70, 10, "%", C_ROW_FG, C_WINDOW_BG);

    // --- Process counts by state ---
    text(2, 12, "Process States:", C_ROW_FG, C_WINDOW_BG);
    let (mut n_run, mut n_rdy, mut n_wait, mut n_zomb) = (0u32, 0u32, 0u32, 0u32);
    for i in 0..count {
        match snaps[i].state {
            TaskState::Running => n_run  += 1,
            TaskState::Ready   => n_rdy  += 1,
            TaskState::Waiting => n_wait += 1,
            TaskState::Zombie  => n_zomb += 1,
            _ => {}
        }
    }
    text(2, 13, "  Running: ", C_OK_FG,   C_WINDOW_BG); text(13, 13, fmt_u64(n_run  as u64, &mut buf), C_OK_FG,   C_WINDOW_BG);
    text(2, 14, "  Ready:   ", C_ROW_FG,  C_WINDOW_BG); text(13, 14, fmt_u64(n_rdy  as u64, &mut buf), C_ROW_FG,  C_WINDOW_BG);
    text(2, 15, "  Waiting: ", C_ROW_FG,  C_WINDOW_BG); text(13, 15, fmt_u64(n_wait as u64, &mut buf), C_ROW_FG,  C_WINDOW_BG);
    text(2, 16, "  Zombie:  ", C_WARN_FG, C_WINDOW_BG); text(13, 16, fmt_u64(n_zomb as u64, &mut buf), C_WARN_FG, C_WINDOW_BG);

    // --- System uptime (ticks → seconds at ~100 Hz) ---
    text(2, 18, "Uptime (ticks):", C_ROW_FG, C_WINDOW_BG);
    text(18, 18, fmt_u64(ticks, &mut buf), C_ROW_FG, C_WINDOW_BG);

    text(2, 19, "Total Procs:   ", C_ROW_FG, C_WINDOW_BG);
    text(18, 19, fmt_u64(count as u64, &mut buf), C_ROW_FG, C_WINDOW_BG);
}

// ── KERNEL tab ────────────────────────────────────────────────────────────────
fn draw_kernel(snaps: &[TaskSnap; MAX_TASKS], count: usize) {
    const W: usize = 80;
    fill(0, 2, W, 22, C_WINDOW_BG);

    let mut buf = [0u8; 20];

    text(2, 3,  "=== AINUX KERNEL DIAGNOSTICS ===", C_TITLE_BG, C_WINDOW_BG);
    text(2, 5,  "Stack Canary Status (per task):", C_ROW_FG, C_WINDOW_BG);

    let mut y = 6usize;
    let mut corrupted = 0u32;
    for i in 0..count.min(16) {
        let s = &snaps[i];
        let (status, fg) = if s.canary_ok { ("OK      ", C_OK_FG) } else { ("CORRUPT!", C_WARN_FG) };
        if !s.canary_ok { corrupted += 1; }
        text(2, y, "  PID ", C_ROW_FG, C_WINDOW_BG);
        text(8, y, fmt_u64(s.id as u64, &mut buf), C_ROW_FG, C_WINDOW_BG);
        text(12, y, "  Canary: ", C_ROW_FG, C_WINDOW_BG);
        text(22, y, status, fg, C_WINDOW_BG);
        y += 1;
    }

    y += 1;
    if corrupted > 0 {
        text(2, y, "!!! KERNEL STACK CORRUPTION DETECTED !!!", C_WARN_FG, C_WINDOW_BG);
    } else {
        text(2, y, "All kernel stacks healthy.", C_OK_FG, C_WINDOW_BG);
    }

    y += 2;
    text(2, y, "Memory Allocator:", C_ROW_FG, C_WINDOW_BG);
    y += 1;

    // Total cycles across all tasks
    let mut total_cycles: u64 = 0;
    for i in 0..count { total_cycles = total_cycles.wrapping_add(snaps[i].total_cycles); }
    text(2, y, "  Total TSC Cycles: ", C_ROW_FG, C_WINDOW_BG);
    text(22, y, fmt_u64(total_cycles, &mut buf), C_ROW_FG, C_WINDOW_BG);

    y += 2;
    text(2, y, "Scheduler: Ainux Bitmap Round-Robin + ULE", C_ROW_FG, C_WINDOW_BG);
    y += 1;
    text(2, y, "Max Tasks: ", C_ROW_FG, C_WINDOW_BG);
    text(13, y, fmt_u64(MAX_TASKS as u64, &mut buf), C_ROW_FG, C_WINDOW_BG);
}

// ── Main entry ────────────────────────────────────────────────────────────────
pub fn cmd_taskmgr(_args: &[&str]) {
    const W: usize = 80;
    const H: usize = 25;

    let mut tab      = Tab::Processes;
    let mut selected = 0usize;
    let mut dirty    = true;

    // Snapshot — refreshed on key press or F5
    let mut count = 0usize;
    let mut snaps = [TaskSnap::empty(); MAX_TASKS];

    let (used_pages, total_pages) = PMM.lock()
        .as_ref()
        .map(|p| p.get_stats_fast())
        .unwrap_or((0, 0));
    let free_pages = total_pages.saturating_sub(used_pages);

    loop {
        // Refresh snapshot when dirty
        if dirty {
            let (c, s) = collect_snapshots();
            count = c;
            snaps = s;

            // ── Render ──────────────────────────────────────────────────────
            draw_title(W);
            draw_tabs(W, tab);

            match tab {
                Tab::Processes   => draw_processes(&snaps, count, selected),
                Tab::Performance => draw_performance(&snaps, count, free_pages, total_pages),
                Tab::Kernel      => draw_kernel(&snaps, count),
            }

            draw_status(W, count, free_pages, total_pages);
            dirty = false;
        }

        // ── Input ────────────────────────────────────────────────────────────
        if let Some(ch) = keyboard::pop_char() {
            dirty = true;
            match ch {
                // Navigation
                '\u{2191}' => { if selected > 0 { selected -= 1; } }
                '\u{2193}' => { if selected + 1 < count { selected += 1; } }

                // Tab switching
                '\t' => {
                    tab = match tab {
                        Tab::Processes   => Tab::Performance,
                        Tab::Performance => Tab::Kernel,
                        Tab::Kernel      => Tab::Processes,
                    };
                    selected = 0;
                }

                // Kill selected process (Del key)
                '\x7F' | '\x08' => {
                    if tab == Tab::Processes && count > 0 {
                        let pid = snaps[selected].id;
                        if pid != 0 { // Never kill PID 0 (kernel)
                            crate::process::scheduler::kill_task(pid);
                        }
                    }
                }

                // F5 – force refresh (already dirty=true)
                '\x15' => {}

                // ESC or Q – exit
                '\x1B' | 'q' | 'Q' => {
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
