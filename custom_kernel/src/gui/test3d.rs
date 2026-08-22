use alloc::format;
use crate::drivers::video;
use alloc::vec::Vec;

const SIN_LUT: [i32; 256] = [
    0, 6, 12, 18, 25, 31, 37, 43, 49, 56, 62, 68, 74, 80, 86, 92, 97, 103, 109, 115,
    120, 126, 131, 136, 142, 147, 152, 157, 162, 167, 171, 176, 180, 185, 189, 193, 197, 201, 205, 208,
    212, 215, 219, 222, 225, 228, 231, 233, 236, 238, 240, 242, 244, 246, 247, 249, 250, 251, 252, 253,
    254, 255, 255, 256, 256, 256, 255, 255, 254, 253, 252, 251, 250, 249, 247, 246, 244, 242, 240, 238,
    236, 233, 231, 228, 225, 222, 219, 215, 212, 208, 205, 201, 197, 193, 189, 185, 180, 176, 171, 167,
    162, 157, 152, 147, 142, 136, 131, 126, 120, 115, 109, 103, 97, 92, 86, 80, 74, 68, 62, 56,
    49, 43, 37, 31, 25, 18, 12, 6, 0, -6, -12, -18, -25, -31, -37, -43, -49, -56, -62, -68,
    -74, -80, -86, -92, -97, -103, -109, -115, -120, -126, -131, -136, -142, -147, -152, -157, -162, -167, -171, -176,
    -180, -185, -189, -193, -197, -201, -205, -208, -212, -215, -219, -222, -225, -228, -231, -233, -236, -238, -240, -242,
    -244, -246, -247, -249, -250, -251, -252, -253, -254, -255, -255, -256, -256, -256, -255, -255, -254, -253, -252, -251,
    -250, -249, -247, -246, -244, -242, -240, -238, -236, -233, -231, -228, -225, -222, -219, -215, -212, -208, -205, -201,
    -197, -193, -189, -185, -180, -176, -171, -167, -162, -157, -152, -147, -142, -136, -131, -126, -120, -115, -109, -103,
    -97, -92, -86, -80, -74, -68, -62, -56, -49, -43, -37, -31, -25, -18, -12, -6
];

fn sin(angle: u8) -> i32 { SIN_LUT[angle as usize] }
fn cos(angle: u8) -> i32 { SIN_LUT[(angle.wrapping_add(64)) as usize] }

#[derive(Clone, Copy)]
struct Vec3 { x: i32, y: i32, z: i32 }

pub fn run(artifact_name: &str) {
    let (w, h) = video::get_resolution();
    
    // Check for NVIDIA GPU presence
    let has_nvidia = check_nvidia_presence();

    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    let mut scale = 3;

    match artifact_name {
        "pyramid" => {
            vertices = alloc::vec![
                Vec3 { x: 0, y: -60, z: 0 },
                Vec3 { x: -50, y: 50, z: -50 },
                Vec3 { x: 50, y: 50, z: -50 },
                Vec3 { x: 50, y: 50, z: 50 },
                Vec3 { x: -50, y: 50, z: 50 },
            ];
            edges = alloc::vec![
                (0, 1), (0, 2), (0, 3), (0, 4), // Lines from top
                (1, 2), (2, 3), (3, 4), (4, 1)  // Base
            ];
            scale = 3;
        },
        "torus" => {
            // Generate a simple torus point cloud (approx 8x16 = 128 points)
            let r1 = 40;
            let r2 = 15;
            for i in 0..16 {
                let u = (i * 16) as u8;
                for j in 0..8 {
                    let v = (j * 32) as u8;
                    let x = (r1 + (r2 * cos(v)) / 256) * cos(u) / 256;
                    let y = (r1 + (r2 * cos(v)) / 256) * sin(u) / 256;
                    let z = r2 * sin(v) / 256;
                    vertices.push(Vec3 { x, y, z });
                }
            }
            // Add edges connecting the grid
            for i in 0..16 {
                for j in 0..8 {
                    let idx = i * 8 + j;
                    let next_i = ((i + 1) % 16) * 8 + j;
                    let next_j = i * 8 + ((j + 1) % 8);
                    edges.push((idx, next_i));
                    edges.push((idx, next_j));
                }
            }
            scale = 4;
        },
        "sphere" => {
            let r = 35;
            for i in 0..16 {
                let phi = (i * 8) as u8; // 0 to 128 (pi)
                for j in 0..16 {
                    let theta = (j * 16) as u8; // 0 to 255 (2pi)
                    let x = (r * sin(phi) / 256) * cos(theta) / 256;
                    let y = (r * sin(phi) / 256) * sin(theta) / 256;
                    let z = r * cos(phi) / 256;
                    vertices.push(Vec3 { x, y, z });
                }
            }
            for i in 0..15 {
                for j in 0..16 {
                    let idx = i * 16 + j;
                    let next_i = (i + 1) * 16 + j;
                    let next_j = i * 16 + ((j + 1) % 16);
                    edges.push((idx, next_i));
                    edges.push((idx, next_j));
                }
            }
            scale = 4;
        },
        "cylinder" => {
            let r = 30;
            let h_half = 40;
            for j in 0..16 {
                let theta = (j * 16) as u8;
                let x = r * cos(theta) / 256;
                let y = r * sin(theta) / 256;
                vertices.push(Vec3 { x, y, z: h_half });
            }
            for j in 0..16 {
                let theta = (j * 16) as u8;
                let x = r * cos(theta) / 256;
                let y = r * sin(theta) / 256;
                vertices.push(Vec3 { x, y, z: -h_half });
            }
            for j in 0..16 {
                let next_j = (j + 1) % 16;
                edges.push((j, next_j));
                edges.push((j + 16, next_j + 16));
                edges.push((j, j + 16));
            }
            scale = 3;
        },
        "cone" => {
            let r = 35;
            let h_half = 40;
            vertices.push(Vec3 { x: 0, y: 0, z: h_half });
            for j in 0..16 {
                let theta = (j * 16) as u8;
                let x = r * cos(theta) / 256;
                let y = r * sin(theta) / 256;
                vertices.push(Vec3 { x, y, z: -h_half });
            }
            for j in 0..16 {
                let current = j + 1;
                let next = ((j + 1) % 16) + 1;
                edges.push((0, current));
                edges.push((current, next));
            }
            scale = 3;
        },
        "cube" | _ => {
            vertices = alloc::vec![
                Vec3 { x: -50, y: -50, z: -50 },
                Vec3 { x: 50, y: -50, z: -50 },
                Vec3 { x: 50, y: 50, z: -50 },
                Vec3 { x: -50, y: 50, z: -50 },
                Vec3 { x: -50, y: -50, z: 50 },
                Vec3 { x: 50, y: -50, z: 50 },
                Vec3 { x: 50, y: 50, z: 50 },
                Vec3 { x: -50, y: 50, z: 50 },
            ];
            edges = alloc::vec![
                (0, 1), (1, 2), (2, 3), (3, 0), // Front face
                (4, 5), (5, 6), (6, 7), (7, 4), // Back face
                (0, 4), (1, 5), (2, 6), (3, 7)  // Connectors
            ];
            scale = 3;
        }
    }

    let mut angle_x: u8 = 0;
    let mut angle_y: u8 = 0;
    let mut angle_z: u8 = 0;

    let mut frames = 0;
    let mut current_fps = 0;
    let mut last_sec = crate::drivers::rtc::read_time().seconds;
    
    // Clear whole screen initially
    video::fill_rect(0, 0, w as i64, h as i64, 0xFF0D0D1A);
    
    // Draw static header ONCE directly to framebuffer
    video::put_str_at(2, 1, &format!("AINUX 3D GPU TEST: {}", artifact_name.to_uppercase()), 0xFFFFFFFF, 0xFF0D0D1A);
    if has_nvidia {
        video::put_str_at(2, 2, "HARDWARE: NVIDIA GPU DETECTED (PCI)", 0xFF00FF00, 0xFF0D0D1A);
    } else {
        video::put_str_at(2, 2, "HARDWARE: GENERIC VGA", 0xFFFFFF00, 0xFF0D0D1A);
    }
    video::put_str_at(2, 4, "Press 'Q' to exit.", 0xFFAAAAAA, 0xFF0D0D1A);

    // Setup partial backbuffer (skipping top 120 pixels for text)
    let top_margin = 120;
    let render_h = if h > top_margin { h - top_margin } else { h };

    // Instead of a huge 1.5MB backbuffer which causes OOM, we will track the previous lines
    // and just erase them by drawing them with the background color before drawing the new ones.
    let mut old_projected: Vec<Vec3> = Vec::new();

    loop {
        let now = crate::drivers::rtc::read_time().seconds;
        if now != last_sec {
            current_fps = frames;
            frames = 0;
            last_sec = now;
            
            // Draw FPS only once per second directly to framebuffer
            let fps_str = format!("FPS: {}  ", current_fps);
            video::put_str_at(60, 1, &fps_str, 0xFF00FFFF, 0xFF0D0D1A);
        }

        let cx = w as i32 / 2;
        let cy = render_h as i32 / 2;

        // Erase old lines directly on framebuffer
        if !old_projected.is_empty() {
            for &(a, b) in edges.iter() {
                let p1 = &old_projected[a];
                let p2 = &old_projected[b];
                draw_line_fb(w, render_h, top_margin, p1.x, p1.y, p2.x, p2.y, 0xFF0D0D1A);
            }
        }

        // Project new vertices
        let mut projected = Vec::with_capacity(vertices.len());
        for i in 0..vertices.len() {
            let v = &vertices[i];
            
            // Rotate X
            let y1 = (v.y * cos(angle_x) - v.z * sin(angle_x)) / 256;
            let z1 = (v.y * sin(angle_x) + v.z * cos(angle_x)) / 256;
            let x1 = v.x;

            // Rotate Y
            let x2 = (x1 * cos(angle_y) - z1 * sin(angle_y)) / 256;
            let z2 = (x1 * sin(angle_y) + z1 * cos(angle_y)) / 256;
            let y2 = y1;

            // Rotate Z
            let x3 = (x2 * cos(angle_z) - y2 * sin(angle_z)) / 256;
            let y3 = (x2 * sin(angle_z) + y2 * cos(angle_z)) / 256;
            
            projected.push(Vec3 {
                x: cx + x3 * scale,
                y: cy + y3 * scale,
                z: 0
            });
        }

        // Draw new edges to framebuffer
        for (i, &(a, b)) in edges.iter().enumerate() {
            let p1 = &projected[a];
            let p2 = &projected[b];
            
            // Use different colors for different lines to make it complex
            let colors = [0xFFFF0055, 0xFF00FFCC, 0xFF5500FF, 0xFFFFFF00];
            let color = colors[i % 4];
            draw_line_fb(w, render_h, top_margin, p1.x, p1.y, p2.x, p2.y, color);
        }

        old_projected = projected;
        frames += 1;

        // Step animation
        angle_x = angle_x.wrapping_add(1);
        angle_y = angle_y.wrapping_add(2);
        angle_z = angle_z.wrapping_add(1);

        // Allow exiting if 'q', Ctrl+C, or Ctrl+Z is pressed
        if let Some(c) = crate::drivers::keyboard::pop_char() {
            if c == 'q' || c == 'Q' || c == '\x03' || c == '\x1A' {
                video::clear();
                break;
            }
        }
        
        // Also check for signals
        if crate::process::scheduler::check_current_signal(2) || crate::process::scheduler::check_current_signal(20) {
            video::clear();
            break;
        }
        
        unsafe { crate::process::scheduler::set_current_sleep(crate::process::scheduler::get_ticks() + 1); }
        crate::process::scheduler::yield_now();
    }
}

fn draw_line_fb(w: usize, h: usize, top_margin: usize, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut e2;

    loop {
        if x0 >= 0 && x0 < w as i32 && y0 >= 0 && y0 < h as i32 {
            crate::drivers::video::draw_pixel(x0 as i64, (y0 + top_margin as i32) as i64, color);
        }
        if x0 == x1 && y0 == y1 { break; }
        e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn check_nvidia_presence() -> bool {
    for bus in 0..=3 {
        for slot in 0..32 {
            let vendor = (crate::drivers::pci::pci_config_read(bus, slot, 0, 0) & 0xFFFF) as u16;
            if vendor == 0x10DE {
                return true;
            }
        }
    }
    false
}
