use alloc::sync::Arc;
use super::types::{IOValue, IOResult};

pub trait IOService: Send + Sync + core::fmt::Debug {
    /// Returns the name of the device/driver
    fn get_name(&self) -> &str;
    
    /// Get a property by key
    fn get_property(&self, key: &str) -> Option<IOValue>;
    
    /// Probe if this driver supports the given provider
    /// Returns a score: 0 = No match, >0 = Match priority
    fn probe(&self, provider: &Arc<dyn IOService>) -> i32;
    
    /// Start the driver
    fn start(&self, provider: &Arc<dyn IOService>) -> IOResult<()>;
    
    /// Stop the driver
    fn stop(&self);
}
