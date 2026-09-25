use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::gui::window::Window;
use crate::gui::compositor::Compositor;

use crate::gui::terminal_app::terminal_main;
use crate::gui::notepad_app::notepad_main;
use crate::drivers::video;

// Helper to draw text into a window's buffer
fn draw_text_to_window(win: &mut Window, text: &str, x: i32, y: i32) {
    let mut curr_x = x;
    for c in text.chars() {
        if c as u32 >= 32 && c as u32 <= 126 {
            let fg = 0x000000; // Black text
            // We use video::draw_char_to_buffer using the window's buffer dimensions
            video::draw_char_to_buffer(&mut win.buffer, win.width as usize, win.height as usize, curr_x as usize, y as usize, c, fg);
        }
        curr_x += 8;
    }
}

use crate::gui::task_manager_app::task_manager_main;
use crate::gui::sysmon_app::sysmon_main;

pub fn launch_task_manager(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(task_manager_main as u64, "task_manager");
}

pub fn launch_sysmon(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(sysmon_main as u64, "sysmon");
}

use crate::gui::settings_app::settings_main;
pub fn launch_settings(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(settings_main as u64, "settings");
}

use crate::gui::paint_app::paint_main;
pub fn launch_paint(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(paint_main as u64, "paint");
}

use crate::gui::browser_app::browser_main;
pub fn launch_browser(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(browser_main as u64, "browser");
}

use crate::gui::calculator_app::calculator_main;
pub fn launch_calculator(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(calculator_main as u64, "calculator");
}

use crate::gui::minesweeper_app::minesweeper_main;
pub fn launch_minesweeper(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(minesweeper_main as u64, "minesweeper");
}

use crate::gui::clock_app::clock_main;
pub fn launch_clock(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(clock_main as u64, "clock");
}

use crate::gui::calendar_app::calendar_main;
pub fn launch_calendar(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(calendar_main as u64, "calendar");
}

use crate::gui::tetris_app::tetris_main;
pub fn launch_tetris(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(tetris_main as u64, "tetris");
}

use crate::gui::pong_app::pong_main;
pub fn launch_pong(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(pong_main as u64, "pong");
}

use crate::gui::game2048_app::game2048_main;
pub fn launch_2048(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(game2048_main as u64, "game2048");
}

use crate::gui::chess_app::chess_main;
pub fn launch_chess(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(chess_main as u64, "chess");
}

use crate::gui::sudoku_app::sudoku_main;
pub fn launch_sudoku(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(sudoku_main as u64, "sudoku");
}

use crate::gui::snake_app::snake_main;
pub fn launch_snake(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(snake_main as u64, "snake");
}

use crate::gui::file_manager_app::file_manager_main;

pub fn launch_file_manager(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(file_manager_main as u64, "file_manager");
}

pub fn launch_terminal(comp: &mut Compositor) {
    let id = comp.windows.len() as u64 + 1;
    crate::process::scheduler::spawn_kernel_task(terminal_main as u64, "terminal_app");
}

pub fn launch_notepad(comp: &mut Compositor) {
    crate::process::scheduler::spawn_kernel_task(notepad_main as u64, "notepad_app");
}
