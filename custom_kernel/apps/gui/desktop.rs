use crate::drivers::video;
use super::compositor::Compositor;
use super::window::Window;

pub fn run() {
    let w = *video::FRAMEBUFFER_WIDTH.lock();
    let h = *video::FRAMEBUFFER_HEIGHT.lock();
    
    if w == 0 || h == 0 { return; }

    let mut comp = Compositor::new(w, h);

    // Terminal — top-left
    let mut win_term = Window::new(1, "Terminal", 20, 30, 480, 320);
    win_term.app = crate::gui::app::AppType::Terminal(crate::gui::terminal_app::TerminalApp::new());
    comp.add_window(win_term);

    // File Manager — alongside terminal
    let mut win_fm = Window::new(2, "File Manager", 520, 30, 340, 380);
    win_fm.app = crate::gui::app::AppType::FileManager(crate::gui::file_manager_app::FileManagerApp::new());
    comp.add_window(win_fm);

    // Settings — bottom-left
    let mut win_set = Window::new(3, "Settings", 20, 380, 340, 280);
    win_set.app = crate::gui::app::AppType::Settings(crate::gui::settings_app::SettingsApp::new());
    comp.add_window(win_set);

    // System Monitor — bottom middle
    let mut win_mon = Window::new(4, "Sys Monitor", 380, 400, 300, 260);
    win_mon.app = crate::gui::app::AppType::SysMon(crate::gui::sysmon_app::SysMonApp::new());
    comp.add_window(win_mon);

    // Task Manager — bottom right
    let mut win_tm = Window::new(5, "Task Manager", 700, 30, 320, 350);
    win_tm.app = crate::gui::app::AppType::TaskManager(crate::gui::task_manager_app::TaskManagerApp::new());
    comp.add_window(win_tm);

    // Clock — corner
    let mut win_clock = Window::new(6, "Clock", 700, 400, 320, 160);
    win_clock.app = crate::gui::app::AppType::Clock(crate::gui::clock_app::ClockApp::new());
    comp.add_window(win_clock);

    // Drain any spurious mouse events from boot
    while let Some(_) = crate::drivers::mouse::pop_event() {}
    
    loop {
        if !comp.process_input() {
            break;
        }
        
        crate::hlt();
    }
    
    // Clear screen on exit
    video::fill_rect(0, 0, w as i64, h as i64, 0x000000);
}
