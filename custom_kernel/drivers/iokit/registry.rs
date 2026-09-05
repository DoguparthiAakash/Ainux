use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use spin::Mutex;
use super::service::IOService;
use super::types::{IOValue, IOResult};

pub struct IORegistryEntry {
    pub service: Arc<dyn IOService>,
    pub parent: Mutex<Option<Weak<IORegistryEntry>>>,
    pub children: Mutex<Vec<Arc<IORegistryEntry>>>,
}

impl IORegistryEntry {
    pub fn new(service: Arc<dyn IOService>) -> Arc<Self> {
        Arc::new(Self {
            service,
            parent: Mutex::new(None),
            children: Mutex::new(Vec::new()),
        })
    }
    
    pub fn add_child(parent: &Arc<IORegistryEntry>, child: &Arc<IORegistryEntry>) {
        // Set parent pointer
        *child.parent.lock() = Some(Arc::downgrade(parent));
        // Add to children
        parent.children.lock().push(child.clone());
    }
}

pub struct IORegistry {
    pub root: Option<Arc<IORegistryEntry>>,
}

pub static REGISTRY: Mutex<IORegistry> = Mutex::new(IORegistry { root: None });

// Basic Platform Driver (Root of the tree)
#[derive(Debug)]
pub struct PlatformDevice {
    name: alloc::string::String,
}

impl PlatformDevice {
    pub fn new(name: &str) -> Self {
        Self { name: alloc::string::String::from(name) }
    }
}

impl IOService for PlatformDevice {
    fn get_name(&self) -> &str { &self.name }
    fn get_property(&self, _key: &str) -> Option<IOValue> { None }
    fn probe(&self, _provider: &Arc<dyn IOService>) -> i32 { 0 }
    fn start(&self, _provider: &Arc<dyn IOService>) -> IOResult<()> { Ok(()) }
    fn stop(&self) {}
}

pub fn init() {
    let root_service = Arc::new(PlatformDevice::new("MacRISC4")); // Arbitrary Root Name
    let root_entry = IORegistryEntry::new(root_service);
    REGISTRY.lock().root = Some(root_entry);
}
