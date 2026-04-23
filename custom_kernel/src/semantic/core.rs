use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::string::String;
use crate::object::KernelObject;
use spin::Mutex;

/// A Semantic attribute or "Sense" for an object.
/// Example: "Hardware", "NetworkAdapter", "Storage", "PrimaryDevice"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sense {
    pub key: String,
    pub value: String,
}

/// The Knowledge Graph of all Sovereign Kernel Objects.
/// This replaces the static /dev and /sys discovery models.
pub struct SemanticRegistry {
    pub entries: Vec<SemanticEntry>,
}

pub struct SemanticEntry {
    pub object: Arc<dyn KernelObject>,
    pub senses: Vec<Sense>,
}

pub static REGISTRY: Mutex<SemanticRegistry> = Mutex::new(SemanticRegistry {
    entries: Vec::new(),
});

impl SemanticRegistry {
    /// Register an object with its semantic attributes
    pub fn register(&mut self, object: Arc<dyn KernelObject>, senses: Vec<Sense>) {
        self.entries.push(SemanticEntry { object, senses });
    }

    /// Discover objects matching a specific Sense
    pub fn discover(&self, key: &str, value: &str) -> Vec<Arc<dyn KernelObject>> {
        self.entries.iter()
            .filter(|e| e.senses.iter().any(|s| s.key == key && s.value == value))
            .map(|e| e.object.clone())
            .collect()
    }
}
