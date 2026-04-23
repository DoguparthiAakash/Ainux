use alloc::sync::Arc;
use alloc::string::String;
use spin::Mutex;

pub mod handle;
pub mod chronos;

/// The blueprint for all sovereign resources in the Ainux Kernel.
/// Every resource (Tasks, Memory, VNet) must implement this trait.
pub trait KernelObject: Send + Sync + core::fmt::Debug {
    fn name(&self) -> String;
    fn id(&self) -> usize;
    fn object_type(&self) -> &'static str;

    /// Capture the absolute current state of the object.
    fn snapshot(&self) -> Result<ObjectSnapshot, &str>;

    /// Restore the object to a previous state.
    fn restore(&self, snapshot: ObjectSnapshot) -> Result<(), &str>;
}

pub struct ObjectSnapshot {
     pub data: alloc::vec::Vec<u8>,
     pub related_handles: alloc::vec::Vec<usize>, // Indicies of handles to re-bind
}

/// A Reference-Counted Handle to a KernelObject.
/// This acts as the secure "token" for capability-based access.
#[derive(Debug, Clone)]
pub struct ObjectHandle {
    pub object: Arc<dyn KernelObject>,
    pub capabilities: CapabilitySet,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct CapabilitySet: u32 {
        const READ    = 0b00000001;
        const WRITE   = 0b00000010;
        const EXECUTE = 0b00000100;
        const GRANT   = 0b00001000; // Ability to share this object
        const DESTROY = 0b00010000; // Ability to dispose of the object
    }
}

impl ObjectHandle {
    pub fn new(object: Arc<dyn KernelObject>, capabilities: CapabilitySet) -> Self {
        Self { object, capabilities }
    }
}
