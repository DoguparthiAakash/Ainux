#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.w && self.x + self.w > other.x &&
        self.y < other.y + other.h && self.y + self.h > other.y
    }

    pub fn subtract(&self, other: &Rect) -> alloc::vec::Vec<Rect> {
        if !self.intersects(other) {
            return alloc::vec![*self];
        }
        
        let mut result = alloc::vec::Vec::new();
        
        // Top rect
        if self.y < other.y {
            result.push(Rect {
                x: self.x,
                y: self.y,
                w: self.w,
                h: other.y - self.y,
            });
        }
        
        // Bottom rect
        if self.y + self.h > other.y + other.h {
            result.push(Rect {
                x: self.x,
                y: other.y + other.h,
                w: self.w,
                h: (self.y + self.h) - (other.y + other.h),
            });
        }
        
        // Left rect
        if self.x < other.x {
            let y1 = self.y.max(other.y);
            let y2 = (self.y + self.h).min(other.y + other.h);
            if y1 < y2 {
                result.push(Rect {
                    x: self.x,
                    y: y1,
                    w: other.x - self.x,
                    h: y2 - y1,
                });
            }
        }
        
        // Right rect
        if self.x + self.w > other.x + other.w {
            let y1 = self.y.max(other.y);
            let y2 = (self.y + self.h).min(other.y + other.h);
            if y1 < y2 {
                result.push(Rect {
                    x: other.x + other.w,
                    y: y1,
                    w: (self.x + self.w) - (other.x + other.w),
                    h: y2 - y1,
                });
            }
        }
        
        result
    }
}
