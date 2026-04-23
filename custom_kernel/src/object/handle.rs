use alloc::vec::Vec;
use alloc::sync::Arc;
use crate::object::{ObjectHandle, KernelObject, CapabilitySet};
use spin::Mutex;

/// A local table of ObjectHandles.
/// This replaces the legacy "File Descriptor" table with a more powerful,
/// capability-based resource manager.
#[derive(Debug)]
pub struct HandleTable {
    handles: Vec<Option<ObjectHandle>>,
}

impl HandleTable {
    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    /// Insert a new handle and return its index
    pub fn insert(&mut self, handle: ObjectHandle) -> usize {
        for (i, entry) in self.handles.iter_mut().enumerate() {
            if entry.is_none() {
                *entry = Some(handle);
                return i;
            }
        }
        self.handles.push(Some(handle));
        self.handles.len() - 1
    }

    /// Retrieve a handle with a capability check
    pub fn get(&self, index: usize, required: CapabilitySet) -> Option<Arc<dyn KernelObject>> {
        let handle = self.handles.get(index)?.as_ref()?;
        if handle.capabilities.contains(required) {
            Some(handle.object.clone())
        } else {
            None // Capability breach
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<ObjectHandle> {
        if index < self.handles.len() {
            self.handles[index].take()
        } else {
            None
        }
    }

    /// Capture a summary of handle associations.
    pub fn snapshot(&self) -> Vec<(usize, usize, CapabilitySet)> {
         let mut snap = Vec::new();
         for (i, entry) in self.handles.iter().enumerate() {
             if let Some(h) = entry {
                 snap.push((i, h.object.id(), h.capabilities));
             }
         }
         snap
    }
}
