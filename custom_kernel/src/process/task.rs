#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Waiting,
    Zombie,
    Free,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Context {
    pub rsp: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rip: u64,
}

use crate::security::cap::CapTable;

#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Task {
    pub id: usize,
    pub context: Context,
    pub state: TaskState,
    pub stack: [u8; 4096], 
    pub cr3: u64, // Page Table Physical Address (0 if kernel task)
    pub userspace_stack_top: u64,
    pub caps: CapTable,
    pub sleep_ticks: u64, // Wake up time (0 = active)
    pub fds: crate::process::fd::FileDescriptorTable,
    pub parent_id: Option<usize>,
    pub exit_code: isize,
    pub wait_queue: alloc::vec::Vec<usize>, // PIDs waiting on this task
    pub cpu_time_ticks: u64,
    pub priority: u8, // 0=High, 255=Low
}

impl Task {
    pub fn new_free() -> Self {
        Self {
            id: 0,
            context: Context { rsp:0, r15:0, r14:0, r13:0, r12:0, rbx:0, rbp:0, rip:0 },
            state: TaskState::Free,
            stack: [0; 4096],
            cr3: 0,
            userspace_stack_top: 0,
            caps: CapTable::new(),
            sleep_ticks: 0,
            fds: crate::process::fd::FileDescriptorTable::new(),
            parent_id: None,
            exit_code: 0,
            wait_queue: alloc::vec::Vec::new(),
            cpu_time_ticks: 0,
            priority: 128, // Default
        }
    }
}
