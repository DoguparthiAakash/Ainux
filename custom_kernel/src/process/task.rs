#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Waiting,
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

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct Task {
    pub id: usize,
    pub context: Context,
    pub state: TaskState,
    pub stack: [u8; 4096], 
    pub cr3: u64, // Page Table Physical Address (0 if kernel task)
    pub user_stack_top: u64,
}

impl Task {
    pub const fn new_free() -> Self {
        Self {
            id: 0,
            context: Context { rsp:0, r15:0, r14:0, r13:0, r12:0, rbx:0, rbp:0, rip:0 },
            state: TaskState::Free,
            stack: [0; 4096],
            cr3: 0,
            user_stack_top: 0,
        }
    }
}
