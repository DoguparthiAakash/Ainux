// =============================================================================
// Ainux DCustom — Whiptail-Style TUI Customizer
// =============================================================================

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use crate::drivers::video::{self, THEME};
use crate::drivers::keyboard;

pub const WP_BG: u32 = 0x00111122;    // Midnight Blue-Black
pub const WP_BOX: u32 = 0x002D2D2D;   // Deep Charcoal Gray
pub const WP_SHADOW: u32 = 0x000F0F0F;// Near Black Shadow
pub const WP_TITLE: u32 = 0x000088AA; // Teal Header
pub const WP_SEL: u32 = 0x0000AAAA;   // Cyan Highlight
pub const WP_TEXT: u32 = 0x00DDDDDD;  // Soft White Text
pub const WP_WHITE: u32 = 0xFFFFFFFF; // Pure White

const COLOR_LIST: [(&str, u32); 12] = [
    ("Black",   0x00000000), ("White",   0xFFFFFFFF), ("Red",     0x00FF0000),
    ("Green",   0x0000FF00), ("Blue",    0x000000FF), ("Yellow",  0x00FFFF00),
    ("Cyan",    0x0000FFFF), ("Magenta", 0x00FF00FF), ("Silver",  0x00C0C0C0),
    ("Gray",    0x00808080), ("Orange",  0x00FFA500), ("SkyBlue", 0x0087CEEB),
];

pub struct Dialog {
    pub title: &'static str,
    pub options: Vec<String>,
    pub selected: usize,
    pub active_btn: usize, // 0 = menu, 1 = primary (Select/Ok), 2 = secondary (Back/Finish)
}

pub fn main() {
    main_personalization();
}

pub fn main_personalization() {
    let mut dialog = Dialog {
        title: " Ainux Sovereign Customization Tool ",
        options: alloc::vec![
            String::from("1 Change UI Colors     Configure desktop color themes"),
            String::from("2 System Appearance    Set font scaling and density"),
            String::from("3 Load Defaults        Reset all theme settings"),
            String::from("4 About dcustom        Information about this tool")
        ],
        selected: 0,
        active_btn: 0,
    };

    loop {
        draw_background();
        draw_dialog(&dialog);

        if let Some(ch) = keyboard::pop_char() {
            match ch {
                '\u{2191}' => if dialog.active_btn == 0 && dialog.selected > 0 { dialog.selected -= 1; },
                '\u{2193}' => if dialog.active_btn == 0 && dialog.selected < dialog.options.len() - 1 { dialog.selected += 1; },
                '\u{2190}' => if dialog.active_btn == 2 { dialog.active_btn = 1; },
                '\u{2192}' => if dialog.active_btn == 1 { dialog.active_btn = 2; },
                '\t' => dialog.active_btn = (dialog.active_btn + 1) % 3,
                '\n' => {
                    if dialog.active_btn == 2 { video::clear(); return; } // Finish/Exit
                    if dialog.active_btn == 1 || dialog.active_btn == 0 {
                        match dialog.selected {
                            0 => color_menu(),
                            1 => appearance_menu(),
                            2 => reset_theme(),
                            3 => about_dialog(),
                            _ => {}
                        }
                    }
                }
                '\x1B' => { video::clear(); return; }, // ESC
                _ => {}
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn color_menu() {
    let mut dialog = Dialog {
        title: " Color Options ",
        options: alloc::vec![
            String::from("Desktop Background"),
            String::from("Text Foreground"),
            String::from("Header Accent Color"),
            String::from("Root Prompt Color")
        ],
        selected: 0,
        active_btn: 0,
    };

    loop {
        draw_background();
        draw_dialog(&dialog);
        if let Some(ch) = keyboard::pop_char() {
            match ch {
                '\u{2191}' => if dialog.active_btn == 0 && dialog.selected > 0 { dialog.selected -= 1; },
                '\u{2193}' => if dialog.active_btn == 0 && dialog.selected < dialog.options.len() - 1 { dialog.selected += 1; },
                '\u{2190}' => if dialog.active_btn == 2 { dialog.active_btn = 1; },
                '\u{2192}' => if dialog.active_btn == 1 { dialog.active_btn = 2; },
                '\t' => dialog.active_btn = (dialog.active_btn + 1) % 3,
                '\n' => {
                    if dialog.active_btn == 2 { return; } // Back
                    pick_color(dialog.selected);
                }
                _ => {}
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn pick_color(target: usize) {
    let mut dialog = Dialog {
        title: " Choose a Color ",
        options: COLOR_LIST.iter().map(|(n, _)| String::from(*n)).collect(),
        selected: 0,
        active_btn: 0,
    };

    loop {
        draw_background();
        draw_dialog(&dialog);
        if let Some(ch) = keyboard::pop_char() {
            match ch {
                '\u{2191}' => if dialog.active_btn == 0 && dialog.selected > 0 { dialog.selected -= 1; },
                '\u{2193}' => if dialog.active_btn == 0 && dialog.selected < dialog.options.len() - 1 { dialog.selected += 1; },
                '\u{2190}' => if dialog.active_btn == 2 { dialog.active_btn = 1; },
                '\u{2192}' => if dialog.active_btn == 1 { dialog.active_btn = 2; },
                '\t' => dialog.active_btn = (dialog.active_btn + 1) % 3,
                '\n' => {
                    if dialog.active_btn == 2 { return; } // Back
                    let color = COLOR_LIST[dialog.selected].1;
                    let mut t = THEME.lock();
                    match target {
                        0 => t.bg = color,
                        1 => t.fg = color,
                        2 => t.accent = color,
                        3 => t.root = color,
                        _ => {}
                    }
                    return;
                }
                _ => {}
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn appearance_menu() {
    let mut dialog = Dialog {
        title: " Appearance Settings ",
        options: alloc::vec![
            String::from("Font Scale (Current +1)"),
            String::from("Toggle High Contrast")
        ],
        selected: 0,
        active_btn: 0,
    };

    loop {
        draw_background();
        draw_dialog(&dialog);
        if let Some(ch) = keyboard::pop_char() {
            match ch {
                '\u{2191}' => if dialog.active_btn == 0 && dialog.selected > 0 { dialog.selected -= 1; },
                '\u{2193}' => if dialog.active_btn == 0 && dialog.selected < dialog.options.len() - 1 { dialog.selected += 1; },
                '\u{2190}' => if dialog.active_btn == 2 { dialog.active_btn = 1; },
                '\u{2192}' => if dialog.active_btn == 1 { dialog.active_btn = 2; },
                '\t' => dialog.active_btn = (dialog.active_btn + 1) % 3,
                '\n' => {
                    if dialog.active_btn == 2 { return; } // Back
                    if dialog.selected == 0 {
                        let mut t = THEME.lock();
                        t.font_size = (t.font_size % 3) + 1;
                    }
                }
                _ => {}
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn about_dialog() {
    draw_background();
    let w = 40; let h = 10;
    let x = (80 - w) / 2; let y = (25 - h) / 2;
    video::draw_rect_grid(x + 1, y + 1, w, h, WP_SHADOW, WP_SHADOW);
    video::draw_rect_grid(x, y, w, h, WP_BOX, WP_BOX);
    video::put_str_at(x + 2, y + 2, "   Ainux DCustom v1.2", WP_TEXT, WP_BOX);
    video::put_str_at(x + 2, y + 4, "   Sovereign TUI System", WP_TEXT, WP_BOX);
    video::put_str_at(x + 2, y + 6, "   Press any key to return", WP_TITLE, WP_BOX);
    
    loop {
        if keyboard::pop_char().is_some() { return; }
        unsafe { core::arch::asm!("hlt"); }
    }
}

pub fn draw_background() {
    let (cols, rows) = (80, 25);
    video::draw_rect_grid(0, 0, cols, rows, WP_WHITE, WP_BG);
    video::put_str_at(1, 0, " Ainux Configuration Hub (Sovereign) ", WP_TEXT, WP_BOX);
    video::draw_rect_grid(0, rows - 1, cols, 1, WP_WHITE, WP_BOX);
    video::put_str_at(1, rows - 1, " <Tab>/<Arrows> Move | <Enter> Select ", WP_TEXT, WP_BOX);
}

pub fn draw_dialog(d: &Dialog) {
    let w = 60;
    let h = (d.options.len() + 8).max(12);
    let x = (80 - w) / 2;
    let y = (25 - h) / 2;

    video::draw_rect_grid(x + 1, y + 1, w, h, WP_SHADOW, WP_SHADOW);
    video::draw_rect_grid(x, y, w, h, WP_BOX, WP_BOX);
    draw_box_lines(x, y, w, h, WP_TEXT, WP_BOX);
    
    let title_x = x + (w - d.title.len()) / 2;
    video::put_str_at(title_x, y, d.title, WP_WHITE, WP_TITLE);

    for (i, opt) in d.options.iter().enumerate() {
        let (fg, bg) = if d.active_btn == 0 && i == d.selected {
            (WP_WHITE, WP_SEL)
        } else {
            (WP_TEXT, WP_BOX)
        };
        let display = if opt.len() > w - 6 { &opt[..w - 6] } else { opt };
        video::put_str_at(x + 3, y + 3 + i, &format!(" {:<width$} ", display, width = w - 8), fg, bg);
    }

    let btn_y = y + h - 3;
    let (p_fg, p_bg) = if d.active_btn == 1 { (WP_WHITE, WP_SEL) } else { (WP_TEXT, WP_BOX) };
    let (s_fg, s_bg) = if d.active_btn == 2 { (WP_WHITE, WP_SEL) } else { (WP_TEXT, WP_BOX) };

    // Dynamic button labels
    let p_txt = if d.title.contains("Sovereign") { " <Select> " } else { " <  Ok  > " };
    let s_txt = if d.title.contains("Sovereign") { " <Finish> " } else { " < Back > " };

    video::put_str_at(x + (w / 2) - 13, btn_y, p_txt, p_fg, p_bg);
    video::put_str_at(x + (w / 2) + 3, btn_y, s_txt, s_fg, s_bg);
}

pub fn draw_box_lines(x: usize, y: usize, w: usize, h: usize, fg: u32, bg: u32) {
    video::put_char_at(x, y, '╔', fg, bg);
    video::put_char_at(x + w - 1, y, '╗', fg, bg);
    for i in 1..(w - 1) { video::put_char_at(x + i, y, '═', fg, bg); }
    for i in 1..(h - 1) { video::put_char_at(x, y + i, '║', fg, bg); }
    for i in 1..(h - 1) { video::put_char_at(x + w - 1, y + i, '║', fg, bg); }
    for i in 1..(w - 1) { video::put_char_at(x + i, y + h - 1, '═', fg, bg); }
    video::put_char_at(x, y + h - 1, '╚', fg, bg);
    video::put_char_at(x + w - 1, y + h - 1, '╝', fg, bg);
}

pub fn draw_input_box(title: &'static str, prompt: &str, buffer: &str, masked: bool) {
    let w = 50;
    let h = 10;
    let x = (80 - w) / 2;
    let y = (25 - h) / 2;

    video::draw_rect_grid(x + 1, y + 1, w, h, WP_SHADOW, WP_SHADOW);
    video::draw_rect_grid(x, y, w, h, WP_BOX, WP_BOX);
    draw_box_lines(x, y, w, h, WP_TEXT, WP_BOX);
    
    let title_x = x + (w - title.len()) / 2;
    video::put_str_at(title_x, y, title, WP_WHITE, WP_TITLE);

    video::put_str_at(x + 3, y + 2, prompt, WP_TEXT, WP_BOX);
    
    // Draw Input Field
    video::draw_rect_grid(x + 3, y + 4, w - 6, 3, WP_SHADOW, WP_WHITE);
    draw_box_lines(x + 3, y + 4, w - 6, 3, WP_TEXT, WP_WHITE);
    
    let display = if masked {
        let mut s = String::new();
        for _ in 0..buffer.len() { s.push('*'); }
        s
    } else {
        String::from(buffer)
    };
    
    video::put_str_at(x + 5, y + 5, &display, WP_TEXT, WP_WHITE);
    
    // Buttons
    video::put_str_at(x + (w / 2) - 10, y + h - 2, " <  Ok  > ", WP_WHITE, WP_SEL);
    video::put_str_at(x + (w / 2) + 2, y + h - 2, " < Back > ", WP_TEXT, WP_BOX);
}

fn reset_theme() {
    let mut t = THEME.lock();
    t.bg = 0x00000000;
    t.fg = 0xFFFFFFFF;
    t.accent = 0x00AAAAFF;
    t.root = 0x00FF5555;
    t.font_size = 1;
}
