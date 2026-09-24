use crate::drivers::video;
use crate::drivers::keyboard;

// Simple fixed-point math for 3D rotation
const SCALE: i64 = 1024; // 1.0 = 1024

fn sin(angle: usize) -> i64 {
    // 360 degrees = 256 units
    let angle = angle % 256;
    let s = [
        0, 25, 50, 75, 100, 125, 150, 174, 198, 221, 243, 265, 287, 307, 327, 347,
        366, 384, 401, 417, 433, 448, 461, 474, 486, 497, 507, 516, 524, 531, 537, 542,
        546, 549, 550, 551, 550, 549, 546, 542, 537, 531, 524, 516, 507, 497, 486, 474,
        461, 448, 433, 417, 401, 384, 366, 347, 327, 307, 287, 265, 243, 221, 198, 174,
        150, 125, 100, 75, 50, 25, 0, -25, -50, -75, -100, -125, -150, -174, -198, -221,
        -243, -265, -287, -307, -327, -347, -366, -384, -401, -417, -433, -448, -461, -474,
        -486, -497, -507, -516, -524, -531, -537, -542, -546, -549, -550, -551, -550, -549,
        -546, -542, -537, -531, -524, -516, -507, -497, -486, -474, -461, -448, -433, -417,
        -401, -384, -366, -347, -327, -307, -287, -265, -243, -221, -198, -174, -150, -125,
        -100, -75, -50, -25
    ];
    let v = if angle < 128 { s[angle] } else { -s[angle - 128] };
    (v * SCALE) / 551 // normalize to SCALE
}

fn cos(angle: usize) -> i64 {
    sin(angle + 64)
}

pub fn run() {
    video::clear();
    let (w, h) = video::get_resolution();
    if w == 0 || h == 0 {
        video::put_str("gputest requires graphical mode!\n");
        return;
    }
    
    let cx = w as i64 / 2;
    let cy = h as i64 / 2;
    let size = (w.min(h) as i64) / 4;
    
    // Cube vertices
    let vertices = [
        (-1, -1, -1), (1, -1, -1), (1, 1, -1), (-1, 1, -1),
        (-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1)
    ];
    
    // Cube edges
    let edges = [
        (0, 1), (1, 2), (2, 3), (3, 0),
        (4, 5), (5, 6), (6, 7), (7, 4),
        (0, 4), (1, 5), (2, 6), (3, 7)
    ];

    let mut angle_x = 0;
    let mut angle_y = 0;
    let mut angle_z = 0;

    // We will draw the cube directly onto the screen.
    // To clear the previous frame, we redraw the old edges in black before drawing new ones.
    let mut old_points = [(0i64, 0i64); 8];

    video::put_str_at(2, 1, "GPU Stress Test: Spinning Cube (Press Q to quit)", 0xFFFFFFFF, 0x000000);

    loop {
        let sx = sin(angle_x);
        let cx_angle = cos(angle_x);
        let sy = sin(angle_y);
        let cy_angle = cos(angle_y);
        let sz = sin(angle_z);
        let cz_angle = cos(angle_z);

        let mut new_points = [(0i64, 0i64); 8];

        for i in 0..8 {
            let (vx, vy, vz) = vertices[i];
            
            // Scale and apply fixed-point rotation
            let mut x = vx * size;
            let mut y = vy * size;
            let mut z = vz * size;
            
            // Rot X
            let y1 = (y * cx_angle - z * sx) / SCALE;
            let z1 = (y * sx + z * cx_angle) / SCALE;
            y = y1;
            z = z1;
            
            // Rot Y
            let x1 = (x * cy_angle + z * sy) / SCALE;
            let z2 = (-x * sy + z * cy_angle) / SCALE;
            x = x1;
            z = z2;
            
            // Rot Z
            let x2 = (x * cz_angle - y * sz) / SCALE;
            let y2 = (x * sz + y * cz_angle) / SCALE;
            x = x2;
            y = y2;
            
            // Pseudo-perspective
            let z_offset = size * 3;
            let z_dist = z + z_offset;
            
            let px = cx + (x * z_offset / z_dist);
            let py = cy + (y * z_offset / z_dist);
            
            new_points[i] = (px, py);
        }

        // Erase old lines
        if angle_x > 0 || angle_y > 0 || angle_z > 0 {
            for &(p1, p2) in &edges {
                let pt1 = old_points[p1];
                let pt2 = old_points[p2];
                video::draw_line(pt1.0, pt1.1, pt2.0, pt2.1, 0x000000);
            }
        }

        // Color based on angles
        let r = ((sin(angle_x) + SCALE) * 255 / (2 * SCALE)) as u32;
        let g = ((sin(angle_y) + SCALE) * 255 / (2 * SCALE)) as u32;
        let b = ((sin(angle_z) + SCALE) * 255 / (2 * SCALE)) as u32;
        let color = (r << 16) | (g << 8) | b;

        // Draw new lines
        for &(p1, p2) in &edges {
            let pt1 = new_points[p1];
            let pt2 = new_points[p2];
            video::draw_line(pt1.0, pt1.1, pt2.0, pt2.1, color);
        }

        old_points = new_points;

        angle_x = (angle_x + 2) % 256;
        angle_y = (angle_y + 3) % 256;
        angle_z = (angle_z + 1) % 256;

        // Small delay
        for _ in 0..50000 {
            unsafe { core::arch::asm!("nop"); }
        }

        if let Some(c) = keyboard::pop_char() {
            if c == 'q' || c == 'Q' || c == '\x1B' || c == '\x03' {
                break;
            }
        }
    }

    video::clear();
}
