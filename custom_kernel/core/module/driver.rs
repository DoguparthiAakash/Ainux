use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

pub trait Driver: Send + Sync {
    fn init(&self) -> Result<(), &'static str>;
    fn name(&self) -> &str;
}

pub static DRIVERS: Mutex<Vec<alloc::boxed::Box<dyn Driver>>> = Mutex::new(Vec::new());

pub fn register_driver(driver: alloc::boxed::Box<dyn Driver>) {
    crate::klog_serial!("Registering driver: {}\n", driver.name());
    let mut drivers = DRIVERS.lock();
    drivers.push(driver);
}
