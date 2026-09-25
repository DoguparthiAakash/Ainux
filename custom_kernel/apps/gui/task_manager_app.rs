use alloc::string::String;
use alloc::vec::Vec;
use crate::gui::wm::{GuiMessage, GuiEvent, send_message, pop_events};

pub struct TaskManagerApp {
    entries: Vec<TaskEntry>,
    selected: usize,
    scroll: usize,
    last_refresh: u64,
    needs_redraw: bool,
    status: String,
}

#[derive(Clone)]
struct TaskEntry {
    pid: usize,
    name: String,
    state: String,
    cpu_ticks: u64,
}

impl TaskManagerApp {
    pub fn new() -> Self {
        let mut app = Self {
            entries: Vec::new(),
            selected: 0,
            scroll: 0,
            last_refresh: 0,
            needs_redraw: true,
            status: String::from("Press K to kill selected process"),
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.entries.clear();
        crate::cpu::without_interrupts(|| {
            let tasks = crate::process::scheduler::TASKS.lock();
            for i in 0..crate::process::scheduler::MAX_TASKS {
                if let Some(task) = &tasks[i] {
                    let state_str = match task.state {
                        crate::process::task::TaskState::Running => "Running",
                        crate::process::task::TaskState::Ready   => "Ready  ",
                        crate::process::task::TaskState::Waiting => "Waiting",
                        crate::process::task::TaskState::Zombie  => "Zombie ",
                        crate::process::task::TaskState::Free    => "Free   ",
                    };
                    self.entries.push(TaskEntry {
                        pid: task.id,
                        name: task.name.clone(),
                        state: String::from(state_str),
                        cpu_ticks: task.cpu_time_ticks,
                    });
                }
            }
        });
        self.entries.sort_by_key(|e| e.pid);
        if self.selected >= self.entries.len().max(1) {
            self.selected = self.entries.len().saturating_sub(1);
        }
        self.needs_redraw = true;
    }

    fn kill_selected(&mut self) {
        if self.entries.is_empty() { return; }
        let pid = self.entries[self.selected].pid;
        crate::process::scheduler::kill_task(pid);
        self.status = alloc::format!("Killed PID {}", pid);
        self.refresh();
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
}

pub fn task_manager_main() {
    let id = 3;
    let width = 360;
    let height = 360;
    
    send_message(GuiMessage::CreateWindow {
        id,
        title: String::from("Task Manager"),
        x: 100,
        y: 100,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0; width * height];
    let mut app = TaskManagerApp::new();
    
    loop {
        // Handle events
        for ev in pop_events(id) {
            match ev {
                GuiEvent::MouseClick { x: _mx, y: my, button } => {
                    if button & 1 != 0 {
                        let list_y = 62;
                        let row_h  = 18;
                        if my >= list_y {
                            let idx = ((my - list_y) / row_h) as usize + app.scroll;
                            if idx < app.entries.len() {
                                app.selected = idx;
                                app.needs_redraw = true;
                            }
                        }
                    }
                }
                GuiEvent::KeyPress { key: c } => {
                    match c {
                        crate::drivers::keyboard::KEY_UP => {
                            if app.selected > 0 { app.selected -= 1; }
                            if app.selected < app.scroll { app.scroll = app.selected; }
                            app.needs_redraw = true;
                        }
                        crate::drivers::keyboard::KEY_DOWN => {
                            if app.selected + 1 < app.entries.len() { app.selected += 1; }
                            app.needs_redraw = true;
                        }
                        'k' | 'K' => { app.kill_selected(); }
                        'r' | 'R' => { app.refresh(); }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        
        let t = crate::process::scheduler::get_ticks();
        if t.saturating_sub(app.last_refresh) > 50 {
            app.last_refresh = t;
            app.refresh();
        }
        
        if app.needs_redraw {
            let w = width;
            let h = height;
            let buf = &mut buffer;
            
            TaskManagerApp::fill(buf, w, h, 0, 0, w as i32, h as i32, 0xFF1E1E2E);
            TaskManagerApp::fill(buf, w, h, 0, 0, w as i32, 22, 0xFF313244);
            TaskManagerApp::text(buf, w, h, 4, 4, "Task Manager", 0xFFCBA6F7);
            
            let total_ticks: u64 = app.entries.iter().map(|e| e.cpu_ticks).sum();
            let summary = alloc::format!("Processes: {}   Total CPU ticks: {}", app.entries.len(), total_ticks);
            TaskManagerApp::text(buf, w, h, 4, 25, &summary, 0xFF6C7086);
            
            TaskManagerApp::fill(buf, w, h, 0, 42, w as i32, 18, 0xFF45475A);
            TaskManagerApp::text(buf, w, h, 4,   44, "PID",     0xFFBAC2E8);
            TaskManagerApp::text(buf, w, h, 50,  44, "Name",    0xFFBAC2E8);
            TaskManagerApp::text(buf, w, h, w as i32 - 120, 44, "State",    0xFFBAC2E8);
            TaskManagerApp::text(buf, w, h, w as i32 - 60,  44, "CPU%",     0xFFBAC2E8);
            
            let list_y = 62;
            let row_h  = 18;
            let visible = ((h as i32 - list_y - 24) / row_h).max(0) as usize;
            
            for (i, entry) in app.entries.iter().enumerate().skip(app.scroll) {
                if i - app.scroll >= visible { break; }
                let ry = list_y + ((i - app.scroll) as i32 * row_h);
                let bg = if i == app.selected { 0xFF585B70 } else if (i - app.scroll) % 2 == 0 { 0xFF1E1E2E } else { 0xFF24273A };
                TaskManagerApp::fill(buf, w, h, 0, ry, w as i32, row_h, bg);
                
                let pid_str = alloc::format!("{}", entry.pid);
                TaskManagerApp::text(buf, w, h, 4, ry + 2, &pid_str, 0xFFA6E3A1);
                
                let name_disp: String = entry.name.chars().take(18).collect();
                TaskManagerApp::text(buf, w, h, 50, ry + 2, &name_disp, 0xFFCDD6F4);
                
                let state_color = match entry.state.trim() {
                    "Running" => 0xFF89DCEB,
                    "Ready"   => 0xFFA6E3A1,
                    "Waiting" => 0xFFF9E2AF,
                    "Zombie"  => 0xFFF38BA8,
                    _         => 0xFF6C7086,
                };
                TaskManagerApp::text(buf, w, h, w as i32 - 120, ry + 2, entry.state.trim(), state_color);
                
                let cpu_pct = if total_ticks > 0 { (entry.cpu_ticks * 100 / total_ticks) as i32 } else { 0 };
                let cpu_str = alloc::format!("{}%", cpu_pct);
                let cpu_color = if cpu_pct > 50 { 0xFFF38BA8 } else { 0xFFCDD6F4 };
                TaskManagerApp::text(buf, w, h, w as i32 - 60, ry + 2, &cpu_str, cpu_color);
            }
            
            TaskManagerApp::fill(buf, w, h, 0, h as i32 - 20, w as i32, 20, 0xFF313244);
            TaskManagerApp::text(buf, w, h, 4, h as i32 - 16, &app.status, 0xFF6C7086);
            
            app.needs_redraw = false;
            
            send_message(GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::yield_now();
    }
}
