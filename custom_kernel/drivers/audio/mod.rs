pub mod ac97;
pub mod speaker;
pub mod sb16;

pub fn init() {
    ac97::init();
    sb16::init();
}
