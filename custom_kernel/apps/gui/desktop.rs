use crate::drivers::video;
use super::compositor::Compositor;
use super::window::Window;
use super::bmp::BmpImage;
use core::fmt::Write;

pub fn run() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let w = *video::FRAMEBUFFER_WIDTH.lock();
    let h = *video::FRAMEBUFFER_HEIGHT.lock();
    
    let _ = write!(serial, "Desktop: run() called. FB={}x{}\n", w, h);
    
    if w == 0 || h == 0 {
        let _ = write!(serial, "Desktop: Framebuffer not ready (0x0), aborting desktop run.\n");
        return;
    }

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
    video::put_str_at(
        ((w as i32) / 2 - (loading_txt.len() as i32 * 8) / 2) as usize,
        ((h as i32) / 2 + 60) as usize,
        loading_txt, 0xFFFFFF, 0x000000,
    );
    
    let _ = write!(serial, "Desktop: Loading screen displayed. Creating compositor...\n");

    let mut comp = Compositor::new(w, h);
    
    let _ = write!(serial, "Desktop: Compositor created. Launching initial apps...\n");

    // Launch default apps — they run as kernel tasks and post window creation
    // messages via the WM IPC queue. The compositor will pick them up on its
    // first process_input() iteration.
    crate::apps::gui_apps::launch_terminal(&mut comp);
    crate::apps::gui_apps::launch_file_manager(&mut comp);
    crate::apps::gui_apps::launch_settings(&mut comp);
    crate::apps::gui_apps::launch_sysmon(&mut comp);
    crate::apps::gui_apps::launch_task_manager(&mut comp);
    crate::apps::gui_apps::launch_clock(&mut comp);

    // Drain any spurious mouse events accumulated during boot so the first
    // click is not a phantom event from the PS/2 controller reset.
    while let Some(_) = crate::drivers::mouse::pop_event() {}

    // === FIX: Force an initial full-screen render immediately ===
    // Without this the desktop stays black until the first mouse/keyboard event
    // because process_input() only calls draw() when needs_redraw is true.
    // Setting full_redraw = true ensures draw() fires on the very first tick.
    comp.full_redraw = true;
    comp.draw(); // Paint the desktop wallpaper & taskbar now
    
    let _ = write!(serial, "Desktop: Initial frame rendered. Entering event loop.\n");

    // Periodic redraw counter: force a full redraw every N ticks even if no
    // input arrives. This ensures windows that get their buffer_ptr set
    // asynchronously (via WM IPC) will appear on screen without needing a
    // mouse/keyboard event to trigger the redraw.
    let mut ticks_since_forced_redraw: u32 = 0;
    const FORCED_REDRAW_INTERVAL: u32 = 20; // Every 20 scheduler ticks (~200ms)

    loop {
        ticks_since_forced_redraw += 1;
        if ticks_since_forced_redraw >= FORCED_REDRAW_INTERVAL {
            ticks_since_forced_redraw = 0;
            // Force a redraw so pending WM IPC window creations/updates are visible
            comp.full_redraw = true;
        }

        if !comp.process_input() {
            let _ = write!(serial, "Desktop: process_input() returned false — shutting down.\n");
            break;
        }
        
        // Sleep one scheduler tick to yield CPU and allow kernel tasks
        // (app threads, network, etc.) to run.
        crate::process::scheduler::sleep(1);
    }
    
    let _ = write!(serial, "Desktop: Exiting. Clearing screen.\n");
    // Clear screen on exit
    video::fill_rect(0, 0, w as i64, h as i64, 0x000000);
}
