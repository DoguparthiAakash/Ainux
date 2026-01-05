use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;

pub struct Driver {
    pub name: String,
    pub status: DriverStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DriverStatus {
    Loaded,
    Running,
    Failed,
}

pub static DRIVERS: Mutex<Vec<Driver>> = Mutex::new(Vec::new());

pub fn register_driver(name: &str) {
    let mut drivers = DRIVERS.lock();
    drivers.push(Driver {
        name: String::from(name),
        status: DriverStatus::Loaded,
    });
}

pub fn driver_health_check() {
    let mut drivers = DRIVERS.lock();
    for driver in drivers.iter_mut() {
        if driver.status == DriverStatus::Failed {
            // Log warning
             unsafe { crate::drivers::video::put_str("Health Check: Driver Failed: "); }
             // name printing hard without generic print
        }
    }
}
