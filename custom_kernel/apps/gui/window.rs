use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::video;
use crate::gui::app::AppType;

#[derive(Clone, Copy, PartialEq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
}

pub struct Window {
    pub id: u32,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub restore_x: i32,
    pub restore_y: i32,
    pub restore_width: i32,
    pub restore_height: i32,
    pub buffer: Vec<u32>,
    pub state: WindowState,
    pub drag_offset_x: i32,
    pub drag_offset_y: i32,
    pub app: AppType,
}

impl Window {
    pub fn new(id: u32, title: &str, x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            id,
            title: String::from(title),
            x,
            y,
            width,
            height,
            restore_x: x,
            restore_y: y,
            restore_width: width,
            restore_height: height,
            buffer: alloc::vec![0xFFFFFF; (width * height) as usize], // White background
            state: WindowState::Normal,
            drag_offset_x: 0,
            drag_offset_y: 0,
            app: AppType::None,
        }
    }
    
    pub fn is_point_inside(&self, px: i32, py: i32) -> bool {
        if self.state == WindowState::Minimized { return false; }
        
        let border = 2;
        let title_h = 24;
        
        px >= self.x - border && px <= self.x + self.width + border &&
        py >= self.y - title_h - border && py <= self.y + self.height + border
    }

    pub fn is_point_in_titlebar(&self, mx: i32, my: i32) -> bool {
        // Title bar is 24px high above the window
        mx >= self.x && mx <= self.x + self.width && my >= self.y - 24 && my <= self.y
    }
    
    pub fn is_point_in_close_btn(&self, mx: i32, my: i32) -> bool {
        // Close button is top-right, 20x20
        let bx = self.x + self.width - 22;
        let by = self.y - 22;
        mx >= bx && mx <= bx + 20 && my >= by && my <= by + 20
    }

    pub fn is_point_in_max_btn(&self, mx: i32, my: i32) -> bool {
        // Maximize button is left of close button, 20x20
        let bx = self.x + self.width - 44;
        let by = self.y - 22;
        mx >= bx && mx <= bx + 20 && my >= by && my <= by + 20
    }

    pub fn is_point_in_min_btn(&self, mx: i32, my: i32) -> bool {
        // Minimize button is left of maximize button, 20x20
        let bx = self.x + self.width - 66;
        let by = self.y - 22;
        mx >= bx && mx <= bx + 20 && my >= by && my <= by + 20
    }

    pub fn is_point_in_resize_btn(&self, mx: i32, my: i32) -> bool {
        // Resize handle is bottom right, 12x12
        let bx = self.x + self.width - 12;
        let by = self.y + self.height - 12;
        mx >= bx && mx <= bx + 12 && my >= by && my <= by + 12
    }

    pub fn check_resize_zone(&self, mx: i32, my: i32) -> Option<u8> {
        let b = 4;
        // 0: Right, 1: Bottom, 2: BottomRight, 3: Left, 4: Top
        let right = mx >= self.x + self.width - b && mx <= self.x + self.width + b && my >= self.y - 24 && my <= self.y + self.height;
        let bottom = my >= self.y + self.height - b && my <= self.y + self.height + b && mx >= self.x && mx <= self.x + self.width;
        let left = mx >= self.x - b && mx <= self.x + b && my >= self.y - 24 && my <= self.y + self.height;
        let top = my >= self.y - 24 - b && my <= self.y - 24 + b && mx >= self.x && mx <= self.x + self.width;
        
        if right && bottom { return Some(2); }
        if right { return Some(0); }
        if bottom { return Some(1); }
        if left { return Some(3); }
        if top { return Some(4); }
        None
    }
}
