use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use crate::object::KernelObject;
use spin::Mutex;

/// Resource quotas for industrial stability
#[derive(Debug, Clone, Copy)]
pub struct RoomQuota {
    pub max_tasks: usize,
    pub max_memory_pages: usize,
    pub max_fds: usize,
}

impl Default for RoomQuota {
    fn default() -> Self {
        Self {
            max_tasks: 16,
            max_memory_pages: 1024, // 4MB
            max_fds: 32,
        }
    }
}

/// The RoomContext defines a sovereign execution environment
use crate::process::task::Task;

#[derive(Debug)]
pub struct RoomContext {
    pub id: usize,
    pub name: String,
    pub quota: Mutex<RoomQuota>,
    pub parent: Option<Arc<RoomContext>>,
    pub root_inode: Mutex<Option<crate::fs::vfs::ArcInode>>,
    pub vnet_stack: Mutex<Option<crate::net::NetStack>>,
    pub rx_queue: Mutex<alloc::collections::VecDeque<Arc<Vec<u8>>>>,
    pub strategy: Mutex<Arc<dyn SchedulerStrategy>>,
}

pub trait SchedulerStrategy: Send + Sync + core::fmt::Debug {
    fn pick_next(&self, tasks: &[Option<Task>; crate::process::scheduler::MAX_TASKS], current: usize) -> Option<usize>;
}

#[derive(Debug)]
pub struct FairShareStrategy;
impl SchedulerStrategy for FairShareStrategy {
    fn pick_next(&self, tasks: &[Option<Task>; crate::process::scheduler::MAX_TASKS], current: usize) -> Option<usize> {
        for i in 1..crate::process::scheduler::MAX_TASKS {
            let idx = (current + i) % crate::process::scheduler::MAX_TASKS;
            if let Some(task) = &tasks[idx] {
                if task.state == crate::process::task::TaskState::Ready {
                    return Some(idx);
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct RealTimeStrategy;
impl SchedulerStrategy for RealTimeStrategy {
    fn pick_next(&self, tasks: &[Option<Task>; crate::process::scheduler::MAX_TASKS], _current: usize) -> Option<usize> {
        let mut best_pid = None;
        let mut best_pri = 0;
        for i in 0..crate::process::scheduler::MAX_TASKS {
            if let Some(task) = &tasks[i] {
                if task.state == crate::process::task::TaskState::Ready {
                    if task.priority > best_pri {
                        best_pri = task.priority;
                        best_pid = Some(i);
                    }
                }
            }
        }
        best_pid
    }
}

pub struct RoomManager {
    pub rooms: Vec<Arc<RoomContext>>,
    pub next_id: usize,
}

pub static ROOM_MANAGER: Mutex<RoomManager> = Mutex::new(RoomManager {
    rooms: Vec::new(),
    next_id: 1,
});

impl RoomContext {
    pub fn new(id: usize, name: &str, parent: Option<Arc<RoomContext>>, strategy: Arc<dyn SchedulerStrategy>) -> Self {
        Self {
            id,
            name: String::from(name),
            quota: Mutex::new(RoomQuota::default()),
            parent,
            root_inode: Mutex::new(None),
            vnet_stack: Mutex::new(None),
            rx_queue: Mutex::new(alloc::collections::VecDeque::new()),
            strategy: Mutex::new(strategy),
        }
    }

    pub fn get_root_inode(&self) -> crate::fs::vfs::ArcInode {
        if let Some(root) = self.root_inode.lock().as_ref() {
            root.clone()
        } else {
            crate::fs::vfs::root()
        }
    }
}

impl KernelObject for RoomContext {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn id(&self) -> usize {
        self.id
    }
    fn object_type(&self) -> &'static str {
        "IsolationRoom"
    }

    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        let mut data = alloc::vec::Vec::new();
        data.extend_from_slice(self.name.as_bytes());
        
        // Find tasks in this room and capture their IDs
        let mut related_handles = alloc::vec::Vec::new();
        // Trigger capture of all child tasks (using logical association)
        // In a full implementation, we would recursively call task.snapshot()
        Ok(crate::object::ObjectSnapshot { data, related_handles })
    }

    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> {
        Ok(())
    }
}

/// Initialize the Root Room (Room 0)
pub fn init() {
    let mut manager = ROOM_MANAGER.lock();
    let root_room = Arc::new(RoomContext::new(0, "root", None, Arc::new(FairShareStrategy)));
    manager.rooms.push(root_room.clone());

    // Register in Semantic Core
    use crate::semantic::core::{Sense, REGISTRY};
    REGISTRY.lock().register(root_room, alloc::vec![
        Sense { key: alloc::string::String::from("Role"), value: alloc::string::String::from("PrimaryRoom") },
        Sense { key: alloc::string::String::from("Security"), value: alloc::string::String::from("Full") },
    ]);
}

pub fn create_room(name: &str, parent_id: usize) -> Option<Arc<RoomContext>> {
    let mut manager = ROOM_MANAGER.lock();
    
    let parent = manager.rooms.iter().find(|r| r.id == parent_id)?.clone();
    let strat = parent.strategy.lock().clone(); // Inherit strategy
    
    let id = manager.next_id;
    manager.next_id += 1;
    
    let room = Arc::new(RoomContext::new(id, name, Some(parent), strat));
    manager.rooms.push(room.clone());
    Some(room)
}

pub fn remorph_room(id: usize, strategy_name: &str) -> Result<(), &'static str> {
    let manager = ROOM_MANAGER.lock();
    if let Some(room) = manager.rooms.iter().find(|r| r.id == id) {
        let new_strat: Arc<dyn SchedulerStrategy> = match strategy_name {
            "rt" => Arc::new(RealTimeStrategy),
            "fair" => Arc::new(FairShareStrategy),
            _ => return Err("Invalid strategy"),
        };
        *room.strategy.lock() = new_strat;
        Ok(())
    } else {
        Err("Room not found")
    }
}
