use crate::drivers::video;
use super::compositor::Compositor;
use super::window::Window;
use super::bmp::BmpImage;

pub fn run() {
    let w = *video::FRAMEBUFFER_WIDTH.lock();
    let h = *video::FRAMEBUFFER_HEIGHT.lock();
    
    if w == 0 || h == 0 { return; }

    // --- Loading Screen ---
    video::fill_rect(0, 0, w as i64, h as i64, 0x000000);
    if let Some(logo) = BmpImage::parse(include_bytes!("../../OSLogo.bmp")) {
        let lx = (w as i32 - logo.width as i32) / 2;
        let ly = (h as i32 - logo.height as i32) / 2 - 40;
        for y in 0..logo.height {
            for x in 0..logo.width {
                let color = logo.data[y * logo.width + x];
                if color & 0xFF000000 != 0 {
                    video::draw_pixel((lx + x as i32) as i64, (ly + y as i32) as i64, color);
                }
            }
        }
    }
    let loading_txt = "Loading Mithl OS powered by Ainux kernel...";
    video::put_str_at(((w as i32) / 2 - (loading_txt.len() as i32 * 8) / 2) as usize, ((h as i32) / 2 + 60) as usize, loading_txt, 0xFFFFFF, 0x000000);
    
    // Simulate loading time removed to prevent QEMU timeouts
    // -----------------------

    let mut comp = Compositor::new(w, h);

    crate::apps::gui_apps::launch_terminal(&mut comp);
    crate::apps::gui_apps::launch_file_manager(&mut comp);
    crate::apps::gui_apps::launch_settings(&mut comp);
    crate::apps::gui_apps::launch_sysmon(&mut comp);
    crate::apps::gui_apps::launch_task_manager(&mut comp);
    crate::apps::gui_apps::launch_clock(&mut comp);

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
