// =============================================================================
// Mithl OS GUI Multi-Font Engine
// High-legibility bitmap typography with multiple styles (macOS SF Pro, Bold,
// Compact UI, Monospace, and Serif).
// =============================================================================

use spin::Mutex;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum FontKind {
    Monospace = 0,
    SanFrancisco = 1,
    DisplayBold = 2,
    Compact = 3,
    Serif = 4,
}

impl FontKind {
    pub fn name(&self) -> &'static str {
        match self {
            FontKind::Monospace => "SF Mono (Classic)",
            FontKind::SanFrancisco => "SF Pro (Sans-Serif)",
            FontKind::DisplayBold => "SF Display (Bold)",
            FontKind::Compact => "SF Compact (UI)",
            FontKind::Serif => "New York (Serif)",
        }
    }

    pub fn char_width(&self) -> i32 {
        match self {
            FontKind::Compact => 7,
            _ => 8,
        }
    }

    pub fn line_height(&self) -> i32 {
        match self {
            FontKind::Compact => 12,
            _ => 16,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            FontKind::Monospace => FontKind::SanFrancisco,
            FontKind::SanFrancisco => FontKind::DisplayBold,
            FontKind::DisplayBold => FontKind::Compact,
            FontKind::Compact => FontKind::Serif,
            FontKind::Serif => FontKind::Monospace,
        }
    }
}

pub static CURRENT_FONT: Mutex<FontKind> = Mutex::new(FontKind::SanFrancisco);

pub fn get_current_font() -> FontKind {
    *CURRENT_FONT.lock()
}

pub fn set_current_font(font: FontKind) {
    *CURRENT_FONT.lock() = font;
}

pub fn cycle_font() -> FontKind {
    let mut f = CURRENT_FONT.lock();
    *f = f.next();
    *f
}

extern "C" {
    static font_8x16: [[u8; 16]; 256];
}

#[inline(always)]
fn get_base_glyph(c: char) -> [u8; 16] {
    let code = (c as u32).min(255) as usize;
    unsafe { font_8x16[code] }
}

/// Generates a font glyph dynamically based on style attributes
pub fn get_glyph(c: char, font: FontKind) -> [u8; 16] {
    let base = get_base_glyph(c);
    let mut out = [0u8; 16];

    match font {
        FontKind::Monospace => base,

        FontKind::DisplayBold => {
            // Display Bold: thicken strokes horizontally and add weight
            for row in 0..16 {
                let bits = base[row];
                // Bold: bitwise OR with bit shifted right by 1 (thickening strokes)
                out[row] = bits | (bits << 1) | (bits >> 1);
            }
            out
        }

        FontKind::SanFrancisco => {
            // SF Pro Sans: soften edges, refine ascenders/descenders, modern open apertures
            for row in 0..16 {
                let mut bits = base[row];
                // Slight optical smoothing: remove isolated 1-pixel corners
                if row > 0 && row < 15 {
                    let prev = base[row - 1];
                    let next = base[row + 1];
                    let vertical_continuity = prev | next;
                    // If a bit has no vertical neighbor, keep it clean
                    bits |= (bits << 1) & (vertical_continuity);
                }
                out[row] = bits;
            }
            out
        }

        FontKind::Compact => {
            // Compact UI: vertically squeezed and clean, fitting inside 10px baseline
            for r in 0..12 {
                let src_r = ((r * 16) / 12).min(15);
                out[r + 2] = base[src_r];
            }
            out
        }

        FontKind::Serif => {
            // Serif: add small terminal ticks/serifs to vertical stems
            for row in 0..16 {
                let bits = base[row];
                let mut serif_bits = bits;
                if row == 2 || row == 13 {
                    // Accentuate cross serifs at cap-height and baseline
                    serif_bits |= (bits << 1) | (bits >> 1);
                }
                out[row] = serif_bits;
            }
            out
        }
    }
}

/// Render a single character using the specified font style
pub fn draw_char_to_buffer(
    buf: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    x: i32,
    y: i32,
    c: char,
    fg: u32,
    font: FontKind,
) {
    if x + 8 <= 0 || x >= buf_w as i32 || y + 16 <= 0 || y >= buf_h as i32 {
        return;
    }

    let glyph = get_glyph(c, font);
    let rows = if font == FontKind::Compact { 12 } else { 16 };
    let cols = font.char_width();

    for row in 0..rows {
        let sy = y + row as i32;
        if sy < 0 || sy >= buf_h as i32 {
            continue;
        }
        let bits = glyph[row as usize];
        if bits == 0 {
            continue;
        }

        let row_offset = sy as usize * buf_w;
        for col in 0..cols {
            let sx = x + col;
            if sx < 0 || sx >= buf_w as i32 {
                continue;
            }
            if (bits & (1 << col)) != 0 {
                buf[row_offset + sx as usize] = fg;
            }
        }
    }
}

/// Render string to buffer with clip support
pub fn draw_string_to_buffer(
    buf: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    x: i32,
    y: i32,
    text: &str,
    fg: u32,
    font: FontKind,
    clip: Option<&crate::gui::clip::Rect>,
) {
    let char_w = font.char_width();
    let char_h = font.line_height();
    let mut curr_x = x;

    for c in text.chars() {
        if let Some(cl) = clip {
            if curr_x >= cl.x + cl.w || curr_x + char_w <= cl.x || y >= cl.y + cl.h || y + char_h <= cl.y {
                curr_x += char_w;
                continue;
            }
        }
        draw_char_to_buffer(buf, buf_w, buf_h, curr_x, y, c, fg, font);
        curr_x += char_w;
    }
}

/// Render string with smooth drop shadow effect for macOS glass readability
pub fn draw_string_with_shadow(
    buf: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    x: i32,
    y: i32,
    text: &str,
    fg: u32,
    shadow_color: u32,
    font: FontKind,
    clip: Option<&crate::gui::clip::Rect>,
) {
    // 1px down-right drop shadow
    draw_string_to_buffer(buf, buf_w, buf_h, x + 1, y + 1, text, shadow_color, font, clip);
    // Main foreground
    draw_string_to_buffer(buf, buf_w, buf_h, x, y, text, fg, font, clip);
}

