use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use crate::object::KernelObject;
use crate::security::cap::CapTable;
use crate::fs::vfs::{ArcInode, Mount};

/// Execution Cell Quota - Defines the hardware boundaries of a cell
#[derive(Debug, Clone, Copy)]
pub struct CellQuota {
    pub max_tasks: usize,
    pub max_memory_pages: usize,
    pub max_fds: usize,
    pub max_fuel: u64, // CPU cycle budget before mandatory yield/suspension
}

impl Default for CellQuota {
    fn default() -> Self {
        Self {
            max_tasks: 32,
            max_memory_pages: 4096, // 16MB
            max_fds: 64,
            max_fuel: 1_000_000,    // 1M cycles
        }
    }
}

/// Namespace Mediation - Per-cell virtual resource graph
#[derive(Debug, Clone)]
pub struct Namespace {
    pub root: ArcInode,
    pub mounts: Vec<Mount>,
}

/// Cell State - For instant suspend/resume logic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Active,
    Suspended, // Frozen in memory
    Dormant,   // Serialized to swap/disk
    Degraded,  // Surviving partial failure
}

/// The ExecutionCell defines a sovereign runtime environment.
/// Inspired by Inferno OS, it is the primary unit of authority and isolation.
#[derive(Debug)]
pub struct ExecutionCell {
    pub id: usize,
    pub name: String,
    pub quota: Mutex<CellQuota>,
    pub parent: Option<Arc<ExecutionCell>>,
    pub namespace: Mutex<Namespace>,
    pub caps: Mutex<CapTable>,
    pub vnet_stack: Mutex<Option<crate::net::NetStack>>,
    pub rx_queue: Mutex<alloc::collections::VecDeque<Arc<Vec<u8>>>>,
    pub state: Mutex<CellState>,
    pub fuel: Mutex<u64>,
    pub fuel_limit: u64,
    pub strategy: Mutex<Arc<dyn crate::process::room::SchedulerStrategy>>,
}

impl ExecutionCell {
    pub fn new(id: usize, name: &str, parent: Option<Arc<ExecutionCell>>, strategy: Arc<dyn crate::process::room::SchedulerStrategy>) -> Self {
        let quota = CellQuota::default();
        Self {
            id,
            name: String::from(name),
            fuel: Mutex::new(quota.max_fuel),
            fuel_limit: quota.max_fuel,
            quota: Mutex::new(quota),
            parent,
            namespace: Mutex::new(Namespace {
                root: crate::fs::vfs::root(),
                mounts: Vec::new(),
            }),
            caps: Mutex::new(CapTable::new()),
            vnet_stack: Mutex::new(None),
            rx_queue: Mutex::new(alloc::collections::VecDeque::new()),
            state: Mutex::new(CellState::Active),
            strategy: Mutex::new(strategy),
        }
    }

    pub fn consume_fuel(&self, amount: u64) {
        let mut fuel = self.fuel.lock();
        if *fuel > amount {
            *fuel -= amount;
        } else {
            *fuel = 0;
        }
    }

    pub fn replenish_fuel(&self) {
        let mut fuel = self.fuel.lock();
        *fuel = self.fuel_limit;
    }

    pub fn get_root_inode(&self) -> ArcInode {
        self.namespace.lock().root.clone()
    }

    pub fn attach_resource(&self, path: &str, fs: Arc<dyn crate::fs::vfs::FileSystem>) {
        let mut ns = self.namespace.lock();
        ns.mounts.push(Mount {
            path: String::from(path),
            fs,
        });
    }

    pub fn check_authority(&self, cap_handle: usize, perm: u8) -> bool {
        self.caps.lock().check_permission(cap_handle, perm)
    }
}

impl KernelObject for ExecutionCell {
    fn name(&self) -> String { self.name.clone() }
    fn id(&self) -> usize { self.id }
    fn object_type(&self) -> &'static str { "ExecutionCell" }

    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        let mut data = Vec::new();
        data.extend_from_slice(self.name.as_bytes());
        // Snapshot cell metadata, caps, and namespace config
        Ok(crate::object::ObjectSnapshot { data, related_handles: Vec::new() })
    }

    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> {
        Ok(())
    }
}

pub struct CellManager {
    pub cells: Vec<Arc<ExecutionCell>>,
    pub next_id: usize,
}

pub static CELL_MANAGER: Mutex<CellManager> = Mutex::new(CellManager {
    cells: Vec::new(),
    next_id: 1,
});

pub fn init() {
    let mut manager = CELL_MANAGER.lock();
    let root_cell = Arc::new(ExecutionCell::new(0, "root", None, Arc::new(crate::process::room::FairShareStrategy)));
    manager.cells.push(root_cell.clone());
}
pub fn create_cell(name: &str, parent_id: usize) -> Option<Arc<ExecutionCell>> {
    let mut manager = CELL_MANAGER.lock();
    let parent = manager.cells.iter().find(|c| c.id == parent_id)?.clone();
    let strat = parent.strategy.lock().clone();
    
    let id = manager.next_id;
    manager.next_id += 1;
    
    let cell = Arc::new(ExecutionCell::new(id, name, Some(parent), strat));
    manager.cells.push(cell.clone());
    Some(cell)
}

pub fn remorph_cell(id: usize, strategy_name: &str) -> Result<(), &'static str> {
    let manager = CELL_MANAGER.lock();
    if let Some(cell) = manager.cells.iter().find(|c| c.id == id) {
        let new_strat: Arc<dyn crate::process::room::SchedulerStrategy> = match strategy_name {
            "rt" => Arc::new(crate::process::room::RealTimeStrategy),
            "fair" => Arc::new(crate::process::room::FairShareStrategy),
            _ => return Err("Invalid strategy"),
        };
        *cell.strategy.lock() = new_strat;
        Ok(())
    } else {
        Err("Cell not found")
    }
}
