#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GdtDescriptor {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtDescriptor {
    pub const fn new(base: u32, limit: u32, access: u8, flags: u8) -> Self {
        Self {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            access,
            granularity: ((limit >> 16) & 0x0F) as u8 | (flags & 0xF0),
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }
}
fn main() {
    let desc = GdtDescriptor::new(0, 0xFFFFF, 0xF3, 0xC0);
    let ptr = &desc as *const _ as *const u64;
    unsafe {
        println!("{:#018x}", *ptr);
    }
}
