const std = @import("std");

// External video driver symbols (to be linked)
extern fn c_draw_char(x: i32, y: i32, c: u32, fg_color: u32, bg_color: u32) void;

export fn fast_grid_clear(width: u32, height: u32, fg: u32, bg: u32, char: u32) void {
    var y: u32 = 0;
    while (y < height) : (y += 1) {
        var x: u32 = 0;
        while (x < width) : (x += 1) {
            c_draw_char(@intCast(x), @intCast(y), char, fg, bg);
        }
    }
}

export fn draw_tui_shadow(x: u32, y: u32, w: u32, h: u32) void {
    // Shadow is usually a dark translucent rect or just black space
    // For TUI, we just draw ' ' with black BG on the right and bottom edges
    var i: u32 = 0;
    
    // Bottom shadow
    while (i < w) : (i += 1) {
        c_draw_char(@intCast(x + i + 1), @intCast(y + h), ' ', 0, 0x000000);
    }
    
    // Right shadow
    i = 0;
    while (i < h) : (i += 1) {
        c_draw_char(@intCast(x + w), @intCast(y + i + 1), ' ', 0, 0x000000);
    }
}

pub fn main() void {}
