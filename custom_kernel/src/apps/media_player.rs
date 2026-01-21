
use alloc::vec::Vec;
use crate::drivers::video;
use crate::drivers::rtc;


use alloc::string::String;

pub enum VideoTheme {
    Default,
    Vikram, // Fire/Sparks Theme
}

pub struct YouTubePlayer {
    width: usize,
    height: usize,
    buffer: Vec<u32>,
    frame_count: usize,
    theme: VideoTheme,
    title: String,
}

impl YouTubePlayer {
    pub fn new(url_or_title: &str) -> Self {
        let width = *video::FRAMEBUFFER_WIDTH.lock();
        let height = *video::FRAMEBUFFER_HEIGHT.lock();
        
        let (theme, title) = if url_or_title.contains("X_kI2j1_YZo") || url_or_title.contains("Vikram") {
            (VideoTheme::Vikram, String::from("Vikram Title Track | Kamal Haasan | Anirudh"))
        } else {
            (VideoTheme::Default, String::from("Rick Astley - Never Gonna Give You Up"))
        };

        Self {
            width,
            height,
            buffer: vec![0; width * height],
            frame_count: 0,
            theme,
            title,
        }
    }

    pub fn run(&mut self) {
        let start_time = rtc::read_time().seconds;
        
        loop {
            // Check for exit (ESC or 'q')
            if let Some(key) = crate::drivers::keyboard::pop_char() {
                if key == 'q' || key == '\x1B' {
                    break;
                }
            }
            
            self.render_frame();
            video::copy_buffer(&self.buffer);
            self.frame_count += 1;
        }
        
        // Clean up / Clear screen
        video::clear();
    }

    fn render_frame(&mut self) {
        match self.theme {
            VideoTheme::Vikram => self.render_vikram(),
            VideoTheme::Default => self.render_plasma(),
        }
        self.draw_ui();
    }

    fn draw_ui(&mut self) {
        let w = self.width;
        let h = self.height;
        // Draw UI Header (fake browser/youtube)
        let header_h = 60;
        for y in 0..header_h {
            for x in 0..w {
                self.buffer[y * w + x] = 0xFF202020; 
            }
        }
        // Draw "YouTube" Logo text simulation (Red box)
        for y in 10..50 {
            for x in 20..80 {
                self.buffer[y * w + x] = 0xFFFF0000; 
            }
        }
        
        // Progress Bar (Simple Red Line)
        let t = self.frame_count as f32 * 0.05;
        let video_y_start = 80;
        let video_h = h - 200;
        let progress_y = video_y_start + video_h + 10;
        let progress_w = (t * 20.0) as usize % w; 
        
        for y in progress_y..progress_y+5 {
            let idx = y * w;
            if idx + w < self.buffer.len() {
                for x in 0..w {
                    if x < progress_w {
                        self.buffer[idx + x] = 0xFFFF0000; 
                    } else {
                        self.buffer[idx + x] = 0xFF888888; 
                    }
                }
            }
        }

        // Title Text Simulation (White blocks for now, until we have real text rendering on FB)
        // Ideally we'd use a font renderer here if available.
        // For now, simple blocks for "Text"
        let text_y = progress_y + 20;
        for y in text_y..text_y+20 {
            let idx = y * w;
            if idx + 400 < self.buffer.len() {
                for x in 20..400 {
                     self.buffer[idx + x] = 0xFFFFFFFF;
                }
            }
        }
    }

    fn render_vikram(&mut self) {
        let w = self.width;
        let h = self.height;
        let t = self.frame_count;
        let video_y_start = 80;
        let video_h = h - 200; 
        
        // Seed pseudo random
        let mut seed = t as u32;
        let mut rand = || -> u32 {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            (seed / 65536) % 32768
        };

        // Fire Effect Simulation
        // Base Color: Black/Dark Red
        // rising sparks
        
        for y in 0..video_h {
            // Gradient Background (Dark to slightly lighter red at bottom)
            let base_r = (y * 50 / video_h) as u32; 
            let bg_color = (base_r << 16) | 0x000000;
            
            for x in 0..w {
               // Noise/Fire
               let r_rnd = rand() % 100;
               let color = if r_rnd < 2 {
                   // Spark
                   0xFFFF0000 // Bright Red
               } else if r_rnd < 5 {
                   0xFFFFA500 // Orange
               } else {
                   bg_color
               };
               
               let idx = (video_y_start + y) * w + x;
               if idx < self.buffer.len() {
                    self.buffer[idx] = color;
               }
            }
        }
        
        // "EAGLE" Text Overlay (Big Block Letters)
        // Center of screen
        if (t / 60) % 2 == 0 { // Flash every second
             let cx = w / 2;
             let cy = video_y_start + video_h / 2;
             // Draw a Box for now
             for y in (cy - 50)..(cy + 50) {
                 for x in (cx - 100)..(cx + 100) {
                      let idx = y * w + x;
                      if idx < self.buffer.len() {
                          self.buffer[idx] = 0xFFFFFFFF;
                      }
                 }
             }
        }
    }

    fn render_plasma(&mut self) {
        let w = self.width;
        let h = self.height;
        let video_y_start = 80;
        let video_h = h - 200;
        let t = self.frame_count as f32 * 0.05;

        // Helper for sin/cos using Taylor series or simple hack
        fn f_sin(mut x: f32) -> f32 {
            while x > 3.14159 { x -= 6.28318; }
            while x < -3.14159 { x += 6.28318; }
            if x < 0.0 { return -f_sin(-x); }
            (4.0 * x * (3.14159 - x)) / 9.8696
        }
        fn f_cos(x: f32) -> f32 { f_sin(x + 1.5708) }
        fn f_sqrt(x: f32) -> f32 {
            if x <= 0.0 { return 0.0; }
            let mut z = x;
            for _ in 0..10 { z = z - (z*z - x) / (2.0 * z); }
            z
        }
        
        for y in (0..video_h).step_by(2) {
            let vy = y as f32;
             for x in (0..w).step_by(2) {
                let vx = x as f32;
                
                let v1 = f_sin(vx * 0.01 + t);
                let v2 = f_sin((vy * 0.01) + t);
                let v3 = f_sin((vx * 0.01) + (vy * 0.01) + t);
                let v4 = f_sin((f_sqrt(vx * vx + vy * vy)) * 0.01 + t);
                
                let v = (v1 + v2 + v3 + v4) / 4.0;
                
                let r = ((f_sin(v) * 0.5 + 0.5) * 255.0) as u32;
                let g = ((f_cos(v) * 0.5 + 0.5) * 255.0) as u32;
                let b = ((f_sin(v + 3.14) * 0.5 + 0.5) * 255.0) as u32;

                let color = (r << 16) | (g << 8) | b;
                
                let base_idx = (video_y_start + y) * w + x;
                if base_idx + w + 1 < self.buffer.len() {
                    self.buffer[base_idx] = color;
                    self.buffer[base_idx+1] = color;
                    self.buffer[base_idx+w] = color;
                    self.buffer[base_idx+w+1] = color;
                }
            }
        }
    }
}

