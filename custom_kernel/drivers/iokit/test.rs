use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::iokit::service::IOService;
use crate::drivers::iokit::registry::{self, IORegistryEntry};
use crate::drivers::iokit::types::{IOValue, IOResult};

#[derive(Debug)]
pub struct TestDriver {
    name: String,
}

impl TestDriver {
    pub fn new() -> Self {
        Self { name: String::from("TestDriver") }
    }
}

impl IOService for TestDriver {
    fn get_name(&self) -> &str { &self.name }
    
    fn get_property(&self, _key: &str) -> Option<IOValue> {
        None
    }
    
    fn probe(&self, _provider: &Arc<dyn IOService>) -> i32 {
        crate::drivers::video::put_str("TestDriver: Probe called.\n");
        100 // High score
    }
    
    fn start(&self, _provider: &Arc<dyn IOService>) -> IOResult<()> {
        crate::drivers::video::put_str("TestDriver: Start called! Driver Active.\n");
        Ok(())
    }
    
    fn stop(&self) {
        crate::drivers::video::put_str("TestDriver: Stop called.\n");
    }
}

pub fn run_test() {
    crate::drivers::video::put_str("IO Kit Test: Registering TestDriver...\n");
    
    // Create Driver Instance
    let driver = Arc::new(TestDriver::new());
    
    // Create Registry Entry (usually done by matching, but manual here for test)
    let entry = IORegistryEntry::new(driver.clone());
    
    // Attach to Root (Platform Device)
    // We need to access Registry Root.
    // REGISTRY.lock().root is Option<Arc<Entry>>.
    let root = registry::REGISTRY.lock().root.clone();
    
    if let Some(r) = root {
        IORegistryEntry::add_child(&r, &entry);
        crate::drivers::video::put_str("IO Kit Test: Attached to Root.\n");
        
        // Manually trigger start for test (Matching logic usually does this)
        let _ = driver.start(&r.service);
    } else {
        crate::drivers::video::put_str("IO Kit Test: Root not found!\n");
    }
}
