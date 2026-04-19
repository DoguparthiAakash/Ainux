#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: isize,
    pub y: isize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: isize,
    pub y: isize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    pub fn new(x: isize, y: isize, w: usize, h: usize) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.x + self.w as isize &&
        p.y >= self.y && p.y < self.y + self.h as isize
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.w as isize &&
        self.x + self.w as isize > other.x &&
        self.y < other.y + other.h as isize &&
        self.y + self.h as isize > other.y
    }
    
    pub fn right(&self) -> isize { self.x + self.w as isize }
    pub fn bottom(&self) -> isize { self.y + self.h as isize }
}
