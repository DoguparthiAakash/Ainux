// =============================================================================
// Ainux DCustom — Sovereign TUI Customizer
// All colors are derived at render-time from video::THEME so that
// every dialog automatically adapts when the user changes the theme.
// =============================================================================

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use crate::drivers::video::{self, THEME};
use crate::drivers::keyboard;
use crate::drivers::colors::Color;

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
    let box_bg = t.dialog_bg;
    let shadow  = blend_dark(t.bg, 0x08);
    let title   = t.accent;
    let sel     = t.sel;
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

// Color target labels — maps a target index to a friendly name
const COLOR_TARGETS: [(&str, &str); 6] = [
    ("1", "Desktop Background   Screen fill color"),
    ("2", "Text Foreground      Body and shell text"),
    ("3", "Header / Accent      Titles and highlights"),
    ("4", "Root Prompt Color    root@ainux: color"),
    ("5", "Dialog Background    Settings box fill"),
    ("6", "Selection Highlight  Active item glow"),
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
            String::from("1 Theme & Colors       Select presets or edit colors"),
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
                '\u{2191}' => {
                    if dialog.active_btn != 0 { dialog.active_btn = 0; }
                    else if dialog.selected > 0 { dialog.selected -= 1; }
                },
                '\u{2193}' => {
                    if dialog.active_btn == 0 {
                        if dialog.selected < dialog.options.len() - 1 { dialog.selected += 1; }
                        else { dialog.active_btn = 1; }
                    }
                },
                '\u{2190}' => if dialog.active_btn == 2 { dialog.active_btn = 1; } else if dialog.active_btn == 1 { dialog.active_btn = 2; },
                '\u{2192}' => if dialog.active_btn == 1 { dialog.active_btn = 2; } else if dialog.active_btn == 2 { dialog.active_btn = 1; },
                '\t' => dialog.active_btn = (dialog.active_btn + 1) % 3,
                '\n' => {
                    if dialog.active_btn == 2 { video::clear(); return; } // Finish/Exit
                    if dialog.active_btn == 1 || dialog.active_btn == 0 {
                        match dialog.selected {
                            0 => theme_and_custom_menu(),
                            1 => appearance_menu(),
                            2 => reset_theme(),
                            3 => about_dialog(),
                            _ => {}
                        }
                        dirty = true;
                    }
                }
                '\x1B' => { video::clear(); return; },
                _ => { dirty = false; }
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn theme_and_custom_menu() {
    let mut dialog = Dialog {
        title: " Theme & Color Settings ",
        options: alloc::vec![
            String::from("1 Choose Theme Preset   Select from TokyoNight, Gruvbox..."),
            String::from("2 Custom Color Editor   Modify individual theme slots")
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
                    if dialog.active_btn == 2 { return; }
                    match dialog.selected {
                        0 => theme_preset_menu(),
                        1 => color_menu(),
                        _ => {}
                    }
                }
                '\x1B' => return,
                _ => { dirty = false; }
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn theme_preset_menu() {
    let mut dialog = Dialog {
        title: " Select Theme Preset ",
        options: alloc::vec![
            String::from("TokyoNight   (Default Dark Blue)"),
            String::from("Gruvbox      (Warm Retro Tones)"),
            String::from("Nord         (Arctic Frost Blue)")
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
                    if dialog.active_btn == 2 { return; }
                    match dialog.selected {
                        0 => apply_preset_tokyo(),
                        1 => apply_preset_gruvbox(),
                        2 => apply_preset_nord(),
                        _ => {}
                    }
                    return;
                }
                '\x1B' => return,
                _ => { dirty = false; }
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

pub fn color_menu() {
    // Build option list from COLOR_TARGETS, showing current value for each
    let mut selected = 0usize;
    let mut active_btn = 0usize;
    let mut dirty = true;

    loop {
        if dirty {
            draw_background();
            draw_color_editor(selected, active_btn);
            dirty = false;
        }
        if let Some(ch) = keyboard::pop_char() {
            dirty = true;
            match ch {
                '\u{2191}' => if active_btn == 0 && selected > 0 { selected -= 1; },
                '\u{2193}' => if active_btn == 0 && selected < COLOR_TARGETS.len() - 1 { selected += 1; },
                '\u{2190}' => if active_btn == 2 { active_btn = 1; },
                '\u{2192}' => if active_btn == 1 { active_btn = 2; },
                '\t' => active_btn = (active_btn + 1) % 3,
                '\n' => {
                    if active_btn == 2 { return; }
                    pick_color(selected);
                    dirty = true;
                }
                '\x1B' => return,
                _ => { dirty = false; }
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

/// Draw the color editor panel with live swatches for each theme slot.
fn draw_color_editor(selected: usize, active_btn: usize) {
    let p = theme_colors();
    let w = 66usize;
    let h = COLOR_TARGETS.len() + 9;
    let x = (80 - w) / 2;
    let y = (25 - h) / 2;

    video::draw_rect_grid(x + 1, y + 1, w, h, p.shadow, p.shadow);
    video::draw_rect_grid(x, y, w, h, p.bg, p.bg);
    draw_box_lines(x, y, w, h, p.text, p.bg);

    let title = " Sovereign Color Editor ";
    video::put_str_at(x + (w - title.len()) / 2, y, title, p.white, p.title);

    // Column headers
    video::put_str_at(x + 3,  y + 2, "  # Target               Current        Swatch", p.white, p.bg);

    // Read current theme values
    let t = THEME.lock();
    let curr = [t.bg, t.fg, t.accent, t.root, p.bg, p.sel];
    drop(t);

    for (i, &(num, label)) in COLOR_TARGETS.iter().enumerate() {
        let is_sel = active_btn == 0 && i == selected;
        let (row_fg, row_bg) = if is_sel { (0x00FFFFFF, p.sel) } else { (p.text, p.bg) };

        let prefix = if is_sel { " -> " } else { "    " };
        let row = format!("{}{} {:<22}", prefix, num, label);
        video::put_str_at(x + 2, y + 3 + i, &row, row_fg, row_bg);

        // Draw the swatch (3 chars wide, filled with the actual color)
        let swatch_x = x + w - 11;
        let swatch_y = y + 3 + i;
        video::draw_rect_grid(swatch_x, swatch_y, 7, 1, curr[i], curr[i]);
        // Overlay the hex value in contrasting text
        let hex = format!("{:06X}", curr[i] & 0xFFFFFF);
        // pick a contrasting fg for the hex label
        let brightness = ((curr[i] >> 16) & 0xFF) as u32
            + ((curr[i] >> 8) & 0xFF) as u32
            + (curr[i] & 0xFF) as u32;
        let hex_fg = if brightness > 384 { 0x00000000 } else { 0x00FFFFFF };
        video::put_str_at(swatch_x, swatch_y, &hex, hex_fg, curr[i]);
    }

    // Preset theme buttons (below the list)
    let preset_y = y + h - 4;
    video::put_str_at(x + 3, preset_y - 1, "--- Quick Presets: Press 1-3 to apply ---", p.text, p.bg);
    video::put_str_at(x + 3, preset_y,     "[1] TokyoNight", 0x007AA2F7, p.bg);
    video::put_str_at(x + 20, preset_y,    "[2] Gruvbox",    0x00D65D0E, p.bg);
    video::put_str_at(x + 34, preset_y,    "[3] Nord",       0x0081A1C1, p.bg);

    // Navigation buttons
    let btn_y = y + h - 2;
    let (p_fg, p_bg) = if active_btn == 1 { (0x00000000, 0x00FFFFFF) } else { (p.text, p.bg) };
    let (s_fg, s_bg) = if active_btn == 2 { (0x00000000, 0x00FFFFFF) } else { (p.text, p.bg) };

    let p_arr = if active_btn == 1 { "-> " } else { "   " };
    let s_arr = if active_btn == 2 { "-> " } else { "   " };

    let left_x  = x + (w / 2) - 16;
    let right_x = x + (w / 2) + 2;

    video::put_str_at(left_x,     btn_y, p_arr,  p_fg, p_bg);
    video::put_str_at(left_x + 3, btn_y, "[ Select ]", p_fg, p_bg);
    video::put_str_at(right_x,     btn_y, s_arr,  s_fg, s_bg);
    video::put_str_at(right_x + 3, btn_y, "[  Back  ]", s_fg, s_bg);
}

/// Color picker: shows all 18 palette entries with swatches
fn pick_color(target: usize) {
    let target_name = match target {
        0 => "Background",
        1 => "Foreground",
        2 => "Accent / Header",
        3 => "Root Prompt",
        4 => "Dialog BG",
        5 => "Selection",
        _ => "Color",
    };

    let mut selected = 0usize;
    let mut active_btn = 0usize;
    let mut dirty = true;

    loop {
        if dirty {
            draw_background();
            draw_color_picker(target_name, selected, active_btn);
            dirty = false;
        }
        if let Some(ch) = keyboard::pop_char() {
            dirty = true;
            match ch {
                '\u{2191}' => if active_btn == 0 && selected > 0 { selected -= 1; },
                '\u{2193}' => if active_btn == 0 && selected < Color::LIST.len() - 1 { selected += 1; },
                '\u{2190}' => if active_btn == 2 { active_btn = 1; },
                '\u{2192}' => if active_btn == 1 { active_btn = 2; },
                '\t' => active_btn = (active_btn + 1) % 3,
                '\n' => {
                    if active_btn == 2 { return; }
                    // Apply the selected color to the correct theme slot
                    let color = Color::LIST[selected].1;
                    apply_color(target, color);
                    return;
                }
                '1' => { apply_preset_tokyo(); return; }
                '2' => { apply_preset_gruvbox(); return; }
                '3' => { apply_preset_nord(); return; }
                '\x1B' => return,
                _ => { dirty = false; }
            }
        }
        unsafe { core::arch::asm!("hlt"); }
    }
}

/// Draw the 18-color picker with swatches
fn draw_color_picker(target_name: &str, selected: usize, active_btn: usize) {
    let p = theme_colors();
    let w = 60usize;
    let h = Color::LIST.len() + 8;
    let x = (80 - w) / 2;
    let y = (25usize).saturating_sub(h) / 2;

    video::draw_rect_grid(x + 1, y + 1, w, h, p.shadow, p.shadow);
    video::draw_rect_grid(x, y, w, h, p.bg, p.bg);
    draw_box_lines(x, y, w, h, p.text, p.bg);

    let title_str = format!(" Set: {} ", target_name);
    video::put_str_at(x + (w - title_str.len()) / 2, y, &title_str, p.white, p.title);
    video::put_str_at(x + 3, y + 2, "  Name          Swatch   Hex Code", p.white, p.bg);

    for (i, &(name, color)) in Color::LIST.iter().enumerate() {
        let is_sel = active_btn == 0 && i == selected;
        let (row_fg, row_bg) = if is_sel { (0x00FFFFFF, p.sel) } else { (p.text, p.bg) };

        let prefix = if is_sel { " -> " } else { "    " };
        let name_col = format!("{}{:<12}", prefix, name);
        video::put_str_at(x + 2, y + 3 + i, &name_col, row_fg, row_bg);

        // Swatch (5 chars)
        let sw_x = x + 20;
        video::draw_rect_grid(sw_x, y + 3 + i, 5, 1, color, color);

        // Hex label over swatch
        let hex = format!("{:06X}", color & 0xFFFFFF);
        let brightness = ((color >> 16) & 0xFF) + ((color >> 8) & 0xFF) + (color & 0xFF);
        let hex_fg = if brightness > 384 { 0x00000000 } else { 0x00FFFFFF };
        video::put_str_at(sw_x, y + 3 + i, "     ", hex_fg, color); // fill

        // Hex value to the right of swatch
        video::put_str_at(x + 27, y + 3 + i, &format!("#{}", hex), p.text, row_bg);
    }

    // Preset shortcuts
    let btn_y = y + h - 2;
    let (p_fg, p_bg) = if active_btn == 1 { (0x00000000, 0x00FFFFFF) } else { (p.text, p.bg) };
    let (s_fg, s_bg) = if active_btn == 2 { (0x00000000, 0x00FFFFFF) } else { (p.text, p.bg) };

    let p_arr = if active_btn == 1 { "-> " } else { "   " };
    let s_arr = if active_btn == 2 { "-> " } else { "   " };

    let left_x  = x + (w / 2) - 16;
    let right_x = x + (w / 2) + 2;

    video::put_str_at(left_x,     btn_y, p_arr,  p_fg, p_bg);
    video::put_str_at(left_x + 3, btn_y, "[ Apply  ]", p_fg, p_bg);
    video::put_str_at(right_x,     btn_y, s_arr,  s_fg, s_bg);
    video::put_str_at(right_x + 3, btn_y, "[  Back  ]", s_fg, s_bg);
}

/// Apply color to the correct theme slot by target index
fn apply_color(target: usize, color: u32) {
    let mut t = THEME.lock();
    match target {
        0 => t.bg = color,
        1 => t.fg = color,
        2 => t.accent = color,
        3 => t.root = color,
        4 => t.dialog_bg = color,
        5 => t.sel = color,
        _ => {}
    }
    crate::config::sync_from_theme();
    crate::config::save();
}

/// Preset: apply full TokyoNight palette in one shot
pub fn apply_preset_tokyo() {
    let mut t = THEME.lock();
    t.bg     = Color::TOKYO_BG;
    t.fg     = Color::TOKYO_FG;
    t.accent = Color::TOKYO_ACCENT;
    t.root   = Color::TOKYO_SUCCESS;
    t.dialog_bg = 0x0016161E;
    t.sel    = 0x002F334D;
    t.font_size = 1;
    drop(t);
    crate::config::sync_from_theme();
    crate::config::save();
}

/// Preset: apply full Gruvbox Dark palette
pub fn apply_preset_gruvbox() {
    let mut t = THEME.lock();
    t.bg     = Color::GRUV_BG;
    t.fg     = Color::GRUV_FG;
    t.accent = Color::GRUV_ACCENT;
    t.root   = Color::GRUV_GREEN;
    t.dialog_bg = 0x00282828;
    t.sel    = 0x00504945;
    t.font_size = 1;
    drop(t);
    crate::config::sync_from_theme();
    crate::config::save();
}

/// Preset: apply full Nord palette
pub fn apply_preset_nord() {
    let mut t = THEME.lock();
    t.bg     = Color::NORD_BG;
    t.fg     = Color::NORD_FG;
    t.accent = Color::NORD_BLUE;
    t.root   = 0x00A3BE8C; // Nord green
    t.dialog_bg = 0x002E3440;
    t.sel    = 0x004C566A;
    t.font_size = 1;
    drop(t);
    crate::config::sync_from_theme();
    crate::config::save();
}

fn appearance_menu() {
    let mut dialog = Dialog {
        title: " Appearance Settings ",
        options: alloc::vec![
            String::from("Font Scale (Current +1)"),
            String::from("Toggle High Contrast"),
            String::from("Set Display Resolution")
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
                    if dialog.active_btn == 2 { return; } // Back
                    if dialog.selected == 0 {
                        let mut t = THEME.lock();
                        t.font_size = (t.font_size % 3) + 1;
                    } else if dialog.selected == 2 {
                        resolution_menu();
                    }
                }
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
            String::from("800 x 600   (Standard)"),
            String::from("1024 x 768  (Legacy)"),
            String::from("1280 x 720  (HD 720p)"),
            String::from("1280 x 1024 (Standard Plus)")
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
                    if dialog.active_btn == 2 { return; } // Back
                    match dialog.selected {
                        0 => video::set_resolution(800, 600, 32),
                        1 => video::set_resolution(1024, 768, 32),
                        2 => video::set_resolution(1280, 720, 32),
                        3 => video::set_resolution(1280, 1024, 32),
                        _ => {}
                    }
                    return;
                }
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
    // Full screen fill
    video::draw_rect_grid(0, 0, cols, rows, p.white, p.screen);
    // Title bar
    video::draw_rect_grid(0, 0, cols, 1, p.white, p.title);
    video::put_str_at(2, 0, "Ainux OS  |  Configuration Hub", 0x00FFFFFF, p.title);
    // Status bar at bottom
    video::draw_rect_grid(0, rows - 1, cols, 1, p.white, p.bg);
    video::put_str_at(1, rows - 1,
        "Up/Down:Move  Tab:Switch Buttons  Enter:Select  Esc:Back",
        p.text, p.bg);
}

pub fn draw_dialog(d: &Dialog) {
    let p = theme_colors();
    let w = 60;
    let h = (d.options.len() + 8).max(12);
    let x = (80 - w) / 2;
    let y = (25 - h) / 2;

    // Shadow + box fill + border
    video::draw_rect_grid(x + 1, y + 1, w, h, p.shadow, p.shadow);
    video::draw_rect_grid(x, y, w, h, p.bg, p.bg);
    draw_box_lines(x, y, w, h, p.title, p.bg);

    // Title
    let title_x = x + (w - d.title.len()) / 2;
    video::put_str_at(title_x, y, d.title, p.white, p.title);

    // --- Menu items ---
    for (i, opt) in d.options.iter().enumerate() {
        let is_selected = d.active_btn == 0 && i == d.selected;
        let (fg, bg) = if is_selected {
            (0x00FFFFFF, p.sel)
        } else {
            (p.text, p.bg)
        };
        let prefix = if is_selected { "-> " } else { "   " };
        let display = if opt.len() > w - 9 { &opt[..w - 9] } else { opt };
        let line = format!("{}{:<width$}", prefix, display, width = w - 9);
        video::put_str_at(x + 2, y + 3 + i, &line, fg, bg);
    }

    // --- Button row ---
    // Determine labels
    let is_sovereign = d.title.contains("Sovereign") || d.title.contains("Color") || d.title.contains("Set:");
    let p_label = if is_sovereign { "[ Select ]" } else { "[   Ok   ]" };
    let s_label = if is_sovereign { "[ Finish ]" } else { "[  Back  ]" };

    let btn_y = y + h - 3;

    // Positions: left button at ~1/3, right at ~2/3
    let left_x  = x + (w / 2) - 16;
    let right_x = x + (w / 2) + 2;

    // Left button (action: Select/Ok)
    let (p_fg, p_bg) = if d.active_btn == 1 {
        (0x00000000, 0x00FFFFFF)  // Inverse = clearly selected
    } else {
        (p.text, p.bg)
    };
    let p_arrow = if d.active_btn == 1 { "-> " } else { "   " };
    video::put_str_at(left_x,     btn_y, p_arrow,  p_fg, p_bg);
    video::put_str_at(left_x + 3, btn_y, p_label,  p_fg, p_bg);

    // Right button (action: Finish/Back)
    let (s_fg, s_bg) = if d.active_btn == 2 {
        (0x00000000, 0x00FFFFFF)  // Inverse = clearly selected
    } else {
        (p.text, p.bg)
    };
    let s_arrow = if d.active_btn == 2 { "-> " } else { "   " };
    video::put_str_at(right_x,     btn_y, s_arrow,  s_fg, s_bg);
    video::put_str_at(right_x + 3, btn_y, s_label,  s_fg, s_bg);
}

pub fn draw_box_lines(x: usize, y: usize, w: usize, h: usize, fg: u32, bg: u32) {
    video::put_char_at(x, y, '\u{2554}', fg, bg); // ╔
    video::put_char_at(x + w - 1, y, '\u{2557}', fg, bg); // ╗
    for i in 1..(w - 1) { video::put_char_at(x + i, y, '\u{2550}', fg, bg); } // ═
    for i in 1..(h - 1) { video::put_char_at(x, y + i, '\u{2551}', fg, bg); } // ║
    for i in 1..(h - 1) { video::put_char_at(x + w - 1, y + i, '\u{2551}', fg, bg); } // ║
    for i in 1..(w - 1) { video::put_char_at(x + i, y + h - 1, '\u{2550}', fg, bg); } // ═
    video::put_char_at(x, y + h - 1, '\u{255A}', fg, bg); // ╚
    video::put_char_at(x + w - 1, y + h - 1, '\u{255D}', fg, bg); // ╝
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
    apply_preset_tokyo();
    crate::config::sync_from_theme();
    crate::config::save();
}
