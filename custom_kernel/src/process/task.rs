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

pub const SIGTERM: u32 = 15;
pub const SIGKILL: u32 = 9;
pub const SIGINT: u32 = 2;
pub const SIGTSTP: u32 = 20;

#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Task {
    pub id: usize,
    pub context: Context,
    pub state: TaskState,
    pub stack: Option<alloc::boxed::Box<[u8; 32768]>>, 
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

    // --- Maturation & Metering ---
    pub total_cycles: u64,
    pub last_tsc: u64,
    pub page_count: usize,
    pub signals: u32,
    pub syscall_count: u64,
    pub vruntime: u64, // Virtual runtime (cycles / priority) for WFS
}

impl Task {
    /// Creates a minimal task structure without allocating a stack yet.
    pub fn new_empty() -> Self {
        Self {
            id: 0,
            context: Context::default(),
            state: TaskState::Free,
            stack: None,
            cr3: 0,
            userspace_stack_top: 0,
            caps: CapTable::new(),
            sleep_ticks: 0,
            fds: crate::process::fd::FileDescriptorTable::new(),
            parent_id: None,
            exit_code: 0,
            wait_queue: alloc::vec::Vec::new(),
            cpu_time_ticks: 0,
            priority: 128,
            total_cycles: 0,
            last_tsc: 0,
            page_count: 0,
            signals: 0,
            syscall_count: 0,
            vruntime: 0,
        }
    }

    /// Resets the task state. Allocates a new stack if none exists.
    pub fn reset(&mut self, id: usize) {
        self.id = id;
        self.context = Context::default();
        self.state = TaskState::Free;
        
        // Ensure a heap-allocated stack exists
        if self.stack.is_none() {
             self.stack = Some(alloc::boxed::Box::new([0; 32768]));
        }
        self.cr3 = 0;
        self.userspace_stack_top = 0;
        self.caps = CapTable::new();
        self.sleep_ticks = 0;
        self.fds = crate::process::fd::FileDescriptorTable::new();
        self.parent_id = None;
        self.exit_code = 0;
        self.wait_queue = alloc::vec::Vec::new();
        self.cpu_time_ticks = 0;
        self.priority = 128;
        self.total_cycles = 0;
        self.last_tsc = 0;
        self.page_count = 0;
        self.signals = 0;
        self.syscall_count = 0;
        self.vruntime = 0;
    }
}
