use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CapType {
    Endpoint(usize), // Points to a global Endpoint ID
    Notification,
    Memory,
    Interrupt(u8),
    Empty,
}

#[derive(Debug, Clone, Copy)]
pub struct Capability {
    pub cap_type: CapType,
    pub permissions: u8, // R, W, X, Grant, etc.
}

#[derive(Debug, Clone)]
pub struct CapTable {
    caps: Vec<Option<Capability>>,
}

impl CapTable {
    pub fn new() -> Self {
        // Pre-allocate some slots
        let mut caps = Vec::with_capacity(16);
        for _ in 0..16 {
            caps.push(None);
        }
        Self { caps }
    }

    pub fn get(&self, handle: usize) -> Option<Capability> {
        if handle < self.caps.len() {
            self.caps[handle]
        } else {
            None
        }
    }

    pub fn insert(&mut self, cap: Capability) -> Option<usize> {
        // Find empty slot
        for (i, slot) in self.caps.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(cap);
                return Some(i);
            }
        }
        // Expand
        let idx = self.caps.len();
        self.caps.push(Some(cap));
        Some(idx)
    }

    pub fn remove(&mut self, handle: usize) {
        if handle < self.caps.len() {
            self.caps[handle] = None;
        }
    }
    pub fn check_permission(&self, handle: usize, perm_mask: u8) -> bool {
        if let Some(cap) = self.get(handle) {
            if (cap.permissions & perm_mask) == perm_mask {
                return true;
            }
        }
        self.audit_failure(handle, perm_mask);
        false
    }
    
    fn audit_failure(&self, handle: usize, required: u8) {
        // Log failure
        // In real OS, send to audit daemon
        // unsafe { crate::drivers::video::put_str("Audit: Cap violation\n"); }
    }
}

pub fn cap_audit(msg: &str) {
    // General audit hook
}
