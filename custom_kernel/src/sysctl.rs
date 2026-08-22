// =============================================================================
// Advanced Unix/BSD Management & Security (Sysctl)
// =============================================================================

use spin::Mutex;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt::Write;

pub struct SysctlManager {
    pub params: BTreeMap<String, String>,
}

pub static SYSCTL: Mutex<SysctlManager> = Mutex::new(SysctlManager {
    params: BTreeMap::new(),
});

pub fn init() {
    let mut sys = SYSCTL.lock();
    sys.params.insert("kern.securelevel".to_string(), "1".to_string());
    sys.params.insert("hw.acpi.thermal".to_string(), "enabled".to_string());
    sys.params.insert("vm.swappiness".to_string(), "60".to_string());
    sys.params.insert("net.ipv4.ip_forward".to_string(), "0".to_string());
    sys.params.insert("kern.version".to_string(), "Ainux 2.0 (Sovereign)".to_string());
}

pub fn get(key: &str) -> Option<String> {
    SYSCTL.lock().params.get(key).cloned()
}

pub fn set(key: &str, value: &str) -> Result<(), &'static str> {
    let mut sys = SYSCTL.lock();
    if !sys.params.contains_key(key) {
        return Err("unknown oid");
    }
    
    // Security policy simulation: Cannot lower securelevel once set
    if key == "kern.securelevel" {
        let current: i32 = sys.params.get(key).unwrap().parse().unwrap_or(0);
        let new_val: i32 = value.parse().unwrap_or(0);
        if new_val < current {
            return Err("Operation not permitted (securelevel cannot be lowered)");
        }
    }
    
    sys.params.insert(key.to_string(), value.to_string());
    Ok(())
}
