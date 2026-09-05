// =============================================================================
// Ainux DCustom — Whiptail-Style TUI Customizer
// All colors are derived at render-time from video::THEME so that
// every dialog automatically adapts when the user changes the theme.
// =============================================================================

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use crate::klog;
use crate::drivers::video::{self, THEME};
use crate::drivers::keyboard;

/// Computed palette — derived from THEME at render time.
/// Never store these globally; call theme_colors() fresh on each render.
pub struct Palette {
    pub bg:     u32, // Dialog box interior
    pub shadow: u32, // Drop shadow
    pub title:  u32, // Title bar
    pub sel:    u32, // Selected item highlight
    pub text:   u32, // Body text
    pub white:  u32, // Bright foreground
    pub screen: u32, // Full-screen background
}

/// Read THEME once and derive a full dialog palette from it.
/// This is the single source of truth for ALL dcustom/settings colors.
pub fn theme_colors() -> Palette {
    let t = THEME.lock();
    // Derive box background: darken the theme bg slightly
    let box_bg = blend_dark(t.bg, 0x18);
    // Derive shadow: almost pure black tinted with bg
    let shadow  = blend_dark(t.bg, 0x08);
    // Title bar: use accent color
    let title   = t.accent;
    // Selection: slightly brighter accent
    let sel     = brighten(t.accent, 0x20);
    // Text: theme fg
    let text    = t.fg;
    let screen  = t.bg;
    drop(t);
    Palette { bg: box_bg, shadow, title, sel, text, white: 0x00FFFFFF, screen }
}

/// Blend a color toward black by subtracting `amount` from each channel.
/// Also exported for use by other modules (hinfo, hfetch, taskman).
pub fn blend_dark_pub(c: u32, amount: u32) -> u32 {
    blend_dark(c, amount)
}

/// Blend a color toward black by subtracting `amount` from each channel.
fn blend_dark(c: u32, amount: u32) -> u32 {
    let r = ((c >> 16) & 0xFF).saturating_sub(amount);
    let g = ((c >>  8) & 0xFF).saturating_sub(amount);
    let b = ( c        & 0xFF).saturating_sub(amount);
    (r << 16) | (g << 8) | b
}

/// Brighten a color by adding `amount` to each channel (clamped at 0xFF).
fn brighten(c: u32, amount: u32) -> u32 {
    let r = (((c >> 16) & 0xFF) + amount).min(0xFF);
    let g = (((c >>  8) & 0xFF) + amount).min(0xFF);
    let b = (( c        & 0xFF) + amount).min(0xFF);
    (r << 16) | (g << 8) | b
}

// Legacy aliases so callers that use WP_* still compile.
// These now read from theme at the call site instead of being const.
#[inline] pub fn wp_bg()     -> u32 { theme_colors().screen }
#[inline] pub fn wp_box()    -> u32 { theme_colors().bg }
#[inline] pub fn wp_shadow() -> u32 { theme_colors().shadow }
#[inline] pub fn wp_title()  -> u32 { theme_colors().title }
#[inline] pub fn wp_sel()    -> u32 { theme_colors().sel }
#[inline] pub fn wp_text()   -> u32 { theme_colors().text }
#[inline] pub fn wp_white()  -> u32 { 0x00FFFFFF }

// Keep old const names working in files that use them directly.
// They resolve to the default theme startup values; components that render
// correctly MUST call theme_colors() at render time instead.
pub const WP_BG:     u32 = 0x001A1B26;
pub const WP_BOX:    u32 = 0x00111827;
pub const WP_SHADOW: u32 = 0x00080C12;
pub const WP_TITLE:  u32 = 0x007AA2F7; // matches default accent
pub const WP_SEL:    u32 = 0x009AC2FF;
pub const WP_TEXT:   u32 = 0x00C0CAF5; // matches default fg
pub const WP_WHITE:  u32 = 0x00FFFFFF;

const COLOR_LIST: [(&str, u32); 12] = [
    ("Black",   0x00000000), ("White",   0x00FFFFFF), ("Red",     0x00FF0000),
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
    klog!("dcustom: enter main_personalization\n");
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

    let mut dirty = true;
    loop {
        if dirty {
            draw_background();
            draw_dialog(&dialog);
            dirty = false;
        }

        if let Some(ch) = keyboard::pop_char() {
            dirty = true;
            match ch {
                '\u{2191}' => if dialog.active_btn == 0 && dialog.selected > 0 { dialog.selected -= 1; },
                '\u{2193}' => if dialog.active_btn == 0 && dialog.selected < dialog.options.len() - 1 { dialog.selected += 1; },
                '\u{2190}' => if dialog.active_btn == 2 { dialog.active_btn = 1; },
                '\u{2192}' => if dialog.active_btn == 1 { dialog.active_btn = 2; },
                '\t' => dialog.active_btn = (dialog.active_btn + 1) % 3,
                '\n' => {
                    if dialog.active_btn == 2 { klog!("dcustom: exit main_personalization\n"); video::clear(); return; } // Finish/Exit
                    if dialog.active_btn == 1 || dialog.active_btn == 0 {
                        dirty = true; // re-render after sub-menu returns
                        match dialog.selected {
                            0 => color_menu(),
                            1 => appearance_menu(),
                            2 => reset_theme(),
                            3 => about_dialog(),
                            _ => {}
                        }
                    }
                }
                '\x1B' | '\x08' | '\x7F' => { klog!("dcustom: exit main_personalization (esc)\n"); video::clear(); return; }, // ESC/Backspace/Delete
                _ => { dirty = false; } // unknown key — don't flicker
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

    let mut dirty = true;
    loop {
        if dirty {
            draw_background();
            draw_dialog(&dialog);
            dirty = false;
        }
        if let Some(ch) = keyboard::pop_char() {
            dirty = true;
            match ch {
                '\u{2191}' => if dialog.active_btn == 0 && dialog.selected > 0 { dialog.selected -= 1; },
                '\u{2193}' => if dialog.active_btn == 0 && dialog.selected < dialog.options.len() - 1 { dialog.selected += 1; },
                '\u{2190}' => if dialog.active_btn == 2 { dialog.active_btn = 1; },
                '\u{2192}' => if dialog.active_btn == 1 { dialog.active_btn = 2; },
                '\t' => dialog.active_btn = (dialog.active_btn + 1) % 3,
                '\n' => {
                    if dialog.active_btn == 2 { 
                        klog!("dcustom: exit color_menu (save)\n");
                        crate::drivers::video::save_theme();
                        return; 
                    } // Back
                    dirty = true;
                    pick_color(dialog.selected);
                }
                '\x1B' | '\x08' | '\x7F' => { klog!("dcustom: exit color_menu (esc/back)\n"); return; }, // ESC/Backspace/Delete
                _ => { dirty = false; }
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

    let mut dirty = true;
    loop {
        if dirty {
            draw_background();
            draw_dialog(&dialog);
            dirty = false;
        }
        if let Some(ch) = keyboard::pop_char() {
            dirty = true;
            match ch {
                '\u{2191}' => if dialog.active_btn == 0 && dialog.selected > 0 { dialog.selected -= 1; },
                '\u{2193}' => if dialog.active_btn == 0 && dialog.selected < dialog.options.len() - 1 { dialog.selected += 1; },
                '\u{2190}' => if dialog.active_btn == 2 { dialog.active_btn = 1; },
                '\u{2192}' => if dialog.active_btn == 1 { dialog.active_btn = 2; },
                '\t' => dialog.active_btn = (dialog.active_btn + 1) % 3,
                '\n' => {
                    if dialog.active_btn == 2 { klog!("dcustom: pick_color back\n"); return; } // Back
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
                '\x1B' | '\x08' | '\x7F' => { klog!("dcustom: exit pick_color (esc/back)\n"); return; }, // ESC/Backspace/Delete
                _ => { dirty = false; }
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
            String::from("Toggle High Contrast"),
            String::from("Set Display Resolution"),
            String::from("Toggle Scrollbar"),
            String::from("Toggle Page-Wise Scrolling")
        ],
        selected: 0,
        active_btn: 0,
    };

    let mut dirty = true;
    loop {
        let theme = THEME.lock();
        let scrollbar_enabled = theme.scrollbar_enabled;
        let page_wise = theme.scroll_page_wise;
        drop(theme);

        dialog.options[3] = format!("Scrollbar: {}", if scrollbar_enabled { "Enabled" } else { "Disabled" });
        dialog.options[4] = format!("Scroll Mode: {}", if page_wise { "Page-Wise" } else { "Smooth" });

        if dirty {
            draw_background();
            draw_dialog(&dialog);
            dirty = false;
        }
        if let Some(ch) = keyboard::pop_char() {
            dirty = true;
            match ch {
                '\u{2191}' => if dialog.active_btn == 0 && dialog.selected > 0 { dialog.selected -= 1; },
                '\u{2193}' => if dialog.active_btn == 0 && dialog.selected < dialog.options.len() - 1 { dialog.selected += 1; },
                '\u{2190}' => if dialog.active_btn == 2 { dialog.active_btn = 1; },
                '\u{2192}' => if dialog.active_btn == 1 { dialog.active_btn = 2; },
                '\t' => dialog.active_btn = (dialog.active_btn + 1) % 3,
                '\n' => {
                    if dialog.active_btn == 2 { 
                        klog!("dcustom: exit appearance_menu (save)\n");
                        crate::drivers::video::save_theme();
                        return; 
                    } // Back
                    if dialog.selected == 0 {
                        let mut t = THEME.lock();
                        t.font_size = (t.font_size % 3) + 1;
                    } else if dialog.selected == 2 {
                        dirty = true;
                        resolution_menu();
                    } else if dialog.selected == 3 {
                        let mut t = THEME.lock();
                        t.scrollbar_enabled = !t.scrollbar_enabled;
                    } else if dialog.selected == 4 {
                        let mut t = THEME.lock();
                        t.scroll_page_wise = !t.scroll_page_wise;
                    }
                }
                '\x1B' | '\x08' | '\x7F' => { klog!("dcustom: exit appearance_menu (esc/back)\n"); return; }, // ESC/Backspace/Delete
                _ => { dirty = false; }
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn resolution_menu() {
    let mut dialog = Dialog {
        title: " Display Resolution ",
        options: alloc::vec![
            String::from("Auto (Highest Supported)"),
            String::from("Default (Bootloader)"),
            String::from("1920 x 1080 (FHD 1080p)"),
            String::from("1600 x 900  (HD+)"),
            String::from("1440 x 900  (WXGA+)"),
            String::from("1366 x 768  (HD)"),
            String::from("1280 x 1024 (SXGA)"),
            String::from("1280 x 720  (HD 720p)"),
            String::from("1024 x 768  (XGA)"),
            String::from("800 x 600   (SVGA)")
        ],
        selected: 0,
        active_btn: 0,
    };

    let mut dirty = true;
    loop {
        if dirty {
            draw_background();
            draw_dialog(&dialog);
            dirty = false;
        }
        if let Some(ch) = keyboard::pop_char() {
            dirty = true;
            match ch {
                '\u{2191}' => if dialog.active_btn == 0 && dialog.selected > 0 { dialog.selected -= 1; },
                '\u{2193}' => if dialog.active_btn == 0 && dialog.selected < dialog.options.len() - 1 { dialog.selected += 1; },
                '\u{2190}' => if dialog.active_btn == 2 { dialog.active_btn = 1; },
                '\u{2192}' => if dialog.active_btn == 1 { dialog.active_btn = 2; },
                '\t' => dialog.active_btn = (dialog.active_btn + 1) % 3,
                '\n' => {
                    if dialog.active_btn == 2 { klog!("dcustom: exit resolution_menu (back)\n"); return; } // Back
                    let success = match dialog.selected {
                        0 => { video::auto_resolution(); true }
                        1 => { video::restore_default_resolution(); true }
                        2 => video::set_resolution(1920, 1080, 32),
                        3 => video::set_resolution(1600, 900, 32),
                        4 => video::set_resolution(1440, 900, 32),
                        5 => video::set_resolution(1366, 768, 32),
                        6 => video::set_resolution(1280, 1024, 32),
                        7 => video::set_resolution(1280, 720, 32),
                        8 => video::set_resolution(1024, 768, 32),
                        9 => video::set_resolution(800, 600, 32),
                        _ => true,
                    };
                    
                    if !success {
                        let p = theme_colors();
                        video::draw_tui_title_box(10, 10, 60, 5, " Error ", 0xFF0000);
                        video::put_str_at(12, 12, "Hardware rejected resolution (Unsupported / VMSVGA)", p.white, p.bg);
                        video::put_str_at(25, 13, "[ Press Any Key ]", p.title, p.bg);
                        loop { if keyboard::pop_char().is_some() { break; } }
                    }
                    return;
                }
                '\x1B' | '\x08' | '\x7F' => { klog!("dcustom: exit resolution_menu (esc/back)\n"); return; }, // ESC/Backspace/Delete
                _ => { dirty = false; }
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn about_dialog() {
    draw_background();
    let p = theme_colors();
    let w = 40; let h = 10;
    let x = (80 - w) / 2; let y = (25 - h) / 2;
    video::draw_rect_grid(x + 1, y + 1, w, h, p.shadow, p.shadow);
    video::draw_rect_grid(x, y, w, h, p.bg, p.bg);
    video::put_str_at(x + 2, y + 2, "   Ainux DCustom v1.2",   p.text,  p.bg);
    video::put_str_at(x + 2, y + 4, "   Sovereign TUI System", p.text,  p.bg);
    video::put_str_at(x + 2, y + 6, "   Press any key to return", p.title, p.bg);

    loop {
        if keyboard::pop_char().is_some() { return; }
        unsafe { core::arch::asm!("hlt"); }
    }
}

pub fn draw_background() {
    let p = theme_colors();
    let (cols, rows) = (80, 25);
    video::draw_rect_grid(0, 0, cols, rows, p.white, p.screen);
    video::put_str_at(1, 0, " Ainux Configuration Hub (Sovereign) ", p.text, p.bg);
    video::draw_rect_grid(0, rows - 1, cols, 1, p.white, p.bg);
    video::put_str_at(1, rows - 1, " <Tab>/<Arrows> Move | <Enter> Select ", p.text, p.bg);
}

pub fn draw_dialog(d: &Dialog) {
    let p = theme_colors();
    let w = 60;
    let h = (d.options.len() + 8).max(12);
    let x = (80 - w) / 2;
    let y = (25 - h) / 2;

    video::draw_rect_grid(x + 1, y + 1, w, h, p.shadow, p.shadow);
    video::draw_rect_grid(x, y, w, h, p.bg, p.bg);
    draw_box_lines(x, y, w, h, p.text, p.bg);

    let title_x = x + (w - d.title.len()) / 2;
    video::put_str_at(title_x, y, d.title, p.white, p.title);

    for (i, opt) in d.options.iter().enumerate() {
        let (fg, bg) = if d.active_btn == 0 && i == d.selected {
            (p.white, p.sel)
        } else {
            (p.text, p.bg)
        };
        let display = if opt.len() > w - 6 { &opt[..w - 6] } else { opt };
        video::put_str_at(x + 3, y + 3 + i, &format!(" {:<width$} ", display, width = w - 8), fg, bg);
    }

    let btn_y = y + h - 3;
    let (p_fg, p_bg) = if d.active_btn == 1 { (p.white, p.sel) } else { (p.text, p.bg) };
    let (s_fg, s_bg) = if d.active_btn == 2 { (p.white, p.sel) } else { (p.text, p.bg) };

    let p_txt = if d.title.contains("Sovereign") { " <Select> " } else { " <  Ok  > " };
    let s_txt = if d.title.contains("Sovereign") { " <Finish> " } else { " < Back > " };

    video::put_str_at(x + (w / 2) - 13, btn_y, p_txt, p_fg, p_bg);
    video::put_str_at(x + (w / 2) + 3,  btn_y, s_txt, s_fg, s_bg);
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
    let p = theme_colors();
    let w = 50;
    let h = 10;
    let x = (80 - w) / 2;
    let y = (25 - h) / 2;

    video::draw_rect_grid(x + 1, y + 1, w, h, p.shadow, p.shadow);
    video::draw_rect_grid(x, y, w, h, p.bg, p.bg);
    draw_box_lines(x, y, w, h, p.text, p.bg);

    let title_x = x + (w - title.len()) / 2;
    video::put_str_at(title_x, y, title, p.white, p.title);

    video::put_str_at(x + 3, y + 2, prompt, p.text, p.bg);

    // Input field — white bg for contrast regardless of theme
    video::draw_rect_grid(x + 3, y + 4, w - 6, 3, p.shadow, 0x00FFFFFF);
    draw_box_lines(x + 3, y + 4, w - 6, 3, p.text, 0x00FFFFFF);

    let display = if masked {
        let mut s = alloc::string::String::new();
        for _ in 0..buffer.len() { s.push('*'); }
        s
    } else {
        alloc::string::String::from(buffer)
    };
    video::put_str_at(x + 5, y + 5, &display, 0x00000000, 0x00FFFFFF);

    video::put_str_at(x + (w / 2) - 10, y + h - 2, " <  Ok  > ", p.white, p.sel);
    video::put_str_at(x + (w / 2) + 2,  y + h - 2, " < Back > ", p.text,  p.bg);
}

fn reset_theme() {
    let mut t = THEME.lock();
    t.bg     = 0x001A1B26; // Nvix dark (matches global theme default)
    t.fg     = 0x00C0CAF5; // Soft blue-white
    t.accent = 0x007AA2F7; // Neovim accent blue
    t.root   = 0x009ECE6A; // Neovim accent green
    t.font_size = 1;
}
