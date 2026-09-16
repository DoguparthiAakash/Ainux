pub mod desktop;
pub mod compositor {
    pub struct Compositor;
    impl Compositor {
        pub fn init() -> Self { Self }
        pub fn add_window(_w: super::window::Window) {}
        pub fn render() {}
    }
}
pub mod window {
    pub struct Window {
        pub text_lines: alloc::vec::Vec<alloc::string::String>,
    }
    impl Window {
        pub fn new(_id: usize, _x: usize, _y: usize, _w: usize, _h: usize, _title: &str) -> Self { 
            Self { text_lines: alloc::vec::Vec::new() } 
        }
        pub fn fill_content(&mut self, _color: u32) {}
    }
}
pub mod graphics {
    pub struct Graphics;
    impl Graphics {
        pub fn fill_rect(_x: usize, _y: usize, _w: usize, _h: usize, _color: super::graphics::Color) {}
    }
    pub struct Color;
    impl Color {
        pub fn from_u32(_val: u32) -> Self { Self }
        pub fn from(_val: u32) -> Self { Self }
    }
}
pub mod test3d {
    pub fn run(_arg: &str) {}
}
