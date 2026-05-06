// =============================================================================
// Ainux Sovereign Color Library
// =============================================================================

pub struct Color;

impl Color {
    // Basic ANSI Colors (32-bit RGB)
    pub const BLACK:   u32 = 0x00000000;
    pub const WHITE:   u32 = 0x00FFFFFF;
    pub const RED:     u32 = 0x00FF0000;
    pub const GREEN:   u32 = 0x0000FF00;
    pub const BLUE:    u32 = 0x000000FF;
    pub const YELLOW:  u32 = 0x00FFFF00;
    pub const CYAN:    u32 = 0x0000FFFF;
    pub const MAGENTA: u32 = 0x00FF00FF;

    // TokyoNight Palette (Official Sovereign Default)
    pub const TOKYO_BG:      u32 = 0x001A1B26;
    pub const TOKYO_FG:      u32 = 0x00C0CAF5;
    pub const TOKYO_ACCENT:  u32 = 0x007AA2F7;
    pub const TOKYO_SUCCESS: u32 = 0x009ECE6A;
    pub const TOKYO_WARN:    u32 = 0x00E0AF68;
    pub const TOKYO_ERROR:   u32 = 0x00F7768E;
    pub const TOKYO_COMMENT: u32 = 0x00565F89;

    // Gruvbox Dark Palette
    pub const GRUV_BG:      u32 = 0x00282828;
    pub const GRUV_FG:      u32 = 0x00EBDBB2;
    pub const GRUV_ACCENT:  u32 = 0x00D65D0E; // Orange
    pub const GRUV_GREEN:   u32 = 0x0098971A;

    // Nord Palette
    pub const NORD_BG:      u32 = 0x002E3440;
    pub const NORD_FG:      u32 = 0x00D8DEE9;
    pub const NORD_BLUE:    u32 = 0x0081A1C1;

    pub const LIST: [(&'static str, u32); 18] = [
        ("Black",      Self::BLACK),
        ("White",      Self::WHITE),
        ("Tokyo BG",   Self::TOKYO_BG),
        ("Tokyo FG",   Self::TOKYO_FG),
        ("Tokyo Blue", Self::TOKYO_ACCENT),
        ("Tokyo Grn",  Self::TOKYO_SUCCESS),
        ("Tokyo Red",  Self::TOKYO_ERROR),
        ("Gruv BG",    Self::GRUV_BG),
        ("Gruv FG",    Self::GRUV_FG),
        ("Gruv Orange",Self::GRUV_ACCENT),
        ("Nord BG",    Self::NORD_BG),
        ("Nord FG",    Self::NORD_FG),
        ("Pure Red",   Self::RED),
        ("Pure Green", Self::GREEN),
        ("Pure Blue",  Self::BLUE),
        ("Yellow",     Self::YELLOW),
        ("Cyan",       Self::CYAN),
        ("Magenta",    Self::MAGENTA),
    ];

    pub fn blend(c1: u32, c2: u32, factor: u32) -> u32 {
        let r1 = (c1 >> 16) & 0xFF;
        let g1 = (c1 >> 8) & 0xFF;
        let b1 = c1 & 0xFF;
        let r2 = (c2 >> 16) & 0xFF;
        let g2 = (c2 >> 8) & 0xFF;
        let b2 = c2 & 0xFF;

        let r = (r1 * (100 - factor) + r2 * factor) / 100;
        let g = (g1 * (100 - factor) + g2 * factor) / 100;
        let b = (b1 * (100 - factor) + b2 * factor) / 100;

        (r << 16) | (g << 8) | b
    }

    pub fn brighten(c: u32, amount: u32) -> u32 {
        let r = (((c >> 16) & 0xFF) + amount).min(0xFF);
        let g = (((c >>  8) & 0xFF) + amount).min(0xFF);
        let b = (( c        & 0xFF) + amount).min(0xFF);
        (r << 16) | (g << 8) | b
    }

    pub fn darken(c: u32, amount: u32) -> u32 {
        let r = ((c >> 16) & 0xFF).saturating_sub(amount);
        let g = ((c >>  8) & 0xFF).saturating_sub(amount);
        let b = ( c        & 0xFF).saturating_sub(amount);
        (r << 16) | (g << 8) | b
    }
}
