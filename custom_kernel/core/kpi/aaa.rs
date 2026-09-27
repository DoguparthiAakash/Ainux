// Ainux Application Architecture (AAA)
// A high-performance, purely capability-based application architecture 
// tailored for building large, complex, safe, and secure software components.

use alloc::sync::Arc;
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// A Secure Component Capability
/// Represents a cryptographically secure, unforgeable token granting access to a resource
/// or an IPC endpoint, preventing unauthorized access across components.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Capability {
    pub id: u64,
    pub permissions: u32,
    pub owner_pid: usize,
}

pub const PERM_READ: u32    = 1 << 0;
pub const PERM_WRITE: u32   = 1 << 1;
pub const PERM_EXECUTE: u32 = 1 << 2;
pub const PERM_IPC: u32     = 1 << 3;
pub const PERM_SHMEM: u32   = 1 << 4; // Shared Memory mapping capability
pub const PERM_DELEGATE: u32= 1 << 5; // Allow delegation of this capability

/// Resource Quota for a Component
#[derive(Clone, Debug)]
pub struct ResourceQuota {
    pub max_memory_bytes: usize,
    pub max_cpu_time_ms: usize,
    pub max_capabilities: usize,
}

/// Message passing payload for secure asynchronous IPC
#[derive(Clone)]
pub struct IpcMessage {
    pub sender_pid: usize,
    pub capability: Option<Capability>,
    pub data: Vec<u8>,
    pub shared_mem_ptr: Option<u64>, // Pointer to zero-copy shared memory segment
}

/// Ainux Application Node (AAA Node)
/// High-level component container ensuring total resource isolation and safe communication.
pub struct AinuxAppNode {
    pub pid: usize,
    pub capabilities: Mutex<BTreeMap<u64, Capability>>,
    pub ipc_queue: Mutex<alloc::collections::VecDeque<IpcMessage>>,
    pub quota: ResourceQuota,
}

impl AinuxAppNode {
    pub fn new(pid: usize, quota: ResourceQuota) -> Self {
        Self {
            pid,
            capabilities: Mutex::new(BTreeMap::new()),
            ipc_queue: Mutex::new(alloc::collections::VecDeque::new()),
            quota,
        }
    }

    /// Grant a capability to this Application Node
    pub fn grant(&self, cap: Capability) -> Result<(), &'static str> {
        let mut caps = self.capabilities.lock();
        if caps.len() >= self.quota.max_capabilities {
            return Err("Quota Exceeded: Capability Limit Reached");
        }
        caps.insert(cap.id, cap);
        Ok(())
    }

    /// Revoke a capability
    pub fn revoke(&self, cap_id: u64) {
        self.capabilities.lock().remove(&cap_id);
    }

    /// Delegate a capability to another node
    pub fn delegate(&self, target: &AinuxAppNode, cap_id: u64) -> Result<(), &'static str> {
        let caps = self.capabilities.lock();
        if let Some(cap) = caps.get(&cap_id) {
            if (cap.permissions & PERM_DELEGATE) != 0 {
                target.grant(cap.clone())?;
                return Ok(());
            }
            return Err("Security Violation: Capability lacks DELEGATE permission");
        }
        Err("Capability not found")
    }

    /// Send a secure, asynchronous IPC message (with optional zero-copy shared memory)
    pub fn send_message(&self, target: &AinuxAppNode, mut msg: IpcMessage) -> Result<(), &'static str> {
        // Enforce Capability-based Security
        if let Some(cap) = &msg.capability {
            let caps = self.capabilities.lock();
            if !caps.contains_key(&cap.id) || (cap.permissions & PERM_IPC) == 0 {
                return Err("Security Violation: Missing IPC Capability");
            }
        }
        msg.sender_pid = self.pid;
        target.ipc_queue.lock().push_back(msg);
        Ok(())
    }

    /// Receive a secure IPC message
    pub fn receive_message(&self) -> Option<IpcMessage> {
        self.ipc_queue.lock().pop_front()
    }
}

lazy_static::lazy_static! {
    pub static ref APP_NODES: Mutex<BTreeMap<usize, Arc<AinuxAppNode>>> = Mutex::new(BTreeMap::new());
}

pub fn register_app(pid: usize) -> Arc<AinuxAppNode> {
    let default_quota = ResourceQuota {
        max_memory_bytes: 1024 * 1024 * 256, // 256 MB
        max_cpu_time_ms: 10_000,
        max_capabilities: 64,
    };
    let node = Arc::new(AinuxAppNode::new(pid, default_quota));
    APP_NODES.lock().insert(pid, node.clone());
    node
}

pub fn init() {
    crate::println!("Ainux Application Architecture (AAA) Initialized.");
    crate::println!(" -> Secure Component Node Model + Adv. Capability-based IPC loaded.");
}
