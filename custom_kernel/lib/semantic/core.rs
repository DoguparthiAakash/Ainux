use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::object::KernelObject;

#[derive(Debug, Clone)]
pub struct Sense {
    pub key: String,
    pub value: String,
}

pub struct RegistryEntry {
    pub object: Arc<dyn KernelObject>,
    pub senses: Vec<Sense>,
}

pub struct Registry {
    pub entries: Vec<RegistryEntry>,
}

impl Registry {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn register(&mut self, object: Arc<dyn KernelObject>, senses: Vec<Sense>) {
        self.entries.push(RegistryEntry { object, senses });
    }

    pub fn discover(&self, key: &str, value: &str) -> Vec<Arc<dyn KernelObject>> {
        let mut results = Vec::new();
        for entry in &self.entries {
            if entry.senses.iter().any(|s| s.key == key && s.value == value) {
                results.push(entry.object.clone());
            }
        }
        results
    }
}

lazy_static! {
    pub static ref REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());
}
