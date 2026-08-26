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

use alloc::sync::Arc;
use spin::Mutex;
use crate::object::handle::HandleTable;
use crate::object::KernelObject;
use crate::security::cap::CapTable;

pub const SIGTERM: u32 = 15;
pub const SIGKILL: u32 = 9;
pub const SIGINT: u32 = 2;
pub const SIGTSTP: u32 = 20;

#[repr(C, align(16))]
#[derive(Debug)]
pub struct Task {
    pub id: usize,
    pub context: Context,
    pub state: TaskState,
    pub stack: alloc::boxed::Box<[u8]>, 
    /// Legacy cr3 integer for compatibility with existing scheduler.
    pub cr3: u64, 
    /// PHASE 2 TEMPORARY: Task owns AddressSpace until Process/Thread separation in Phase 4.
    pub address_space: Option<Arc<Mutex<crate::mm::address_space::AddressSpace>>>,
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
    pub canary: u64,
    pub pgid: usize,
    pub sid: usize,
    pub signal_mask: u64,
    pub pending_signals: u64,
    pub cwd: alloc::string::String,
    pub environ: alloc::vec::Vec<(alloc::string::String, alloc::string::String)>,
    pub cell: Arc<crate::process::cell::ExecutionCell>,
    pub handle_table: Mutex<HandleTable>,
    pub name: alloc::string::String,
    pub sigactions: [crate::process::signal::SigAction; 64],
    pub brk: u64,
    pub mmap_base: u64,
    // --- UNIX Process Credentials ---
    pub ruid: u32,
    pub euid: u32,
    pub rgid: u32,
    pub egid: u32,
    pub umask: u16,
}

impl KernelObject for Task {
    fn name(&self) -> alloc::string::String {
        self.name.clone()
    }
    fn id(&self) -> usize {
        self.id
    }
    fn object_type(&self) -> &'static str {
        "Process"
    }

    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        let mut data = alloc::vec::Vec::new();
        // Serialize core fields (Context, State, Cycles, etc.)
        data.extend_from_slice(unsafe { 
            core::slice::from_raw_parts(&self.context as *const _ as *const u8, core::mem::size_of::<Context>()) 
        });
        data.push(self.state as u8);
        data.extend_from_slice(&self.cpu_time_ticks.to_le_bytes());
        data.extend_from_slice(&self.total_cycles.to_le_bytes());
        data.extend_from_slice(self.stack.as_ref()); // Full Kernel Stack

        // Handles are serialized as their IDs in related_handles
        let mut related_handles = alloc::vec::Vec::new();
        let h_snap = self.handle_table.lock().snapshot();
        for (_, obj_id, _) in h_snap {
            related_handles.push(obj_id);
        }

        Ok(crate::object::ObjectSnapshot { data, related_handles })
    }

    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> {
        // Implementation for restoring (requires mutable access or cell)
        // For now, we will handle restore at the Scheduler level.
        Ok(())
    }
}

pub const STACK_CANARY_MAGIC: u64 = 0xDEADC0DE_FEEDFACE;

impl Task {
    pub fn new_free(cell: Arc<crate::process::cell::ExecutionCell>) -> Self {
        Self {
            id: 0,
            context: Context { rsp:0, r15:0, r14:0, r13:0, r12:0, rbx:0, rbp:0, rip:0 },
            state: TaskState::Free,
            stack: alloc::vec![0; 8192].into_boxed_slice(),
            cr3: 0,
            address_space: None, // Will be set by loader or scheduler for user tasks
            userspace_stack_top: 0,
            caps: CapTable::new(),
            sleep_ticks: 0,
            fds: crate::process::fd::FileDescriptorTable::new(),
            parent_id: None,
            exit_code: 0,
            wait_queue: alloc::vec::Vec::new(),
            cpu_time_ticks: 0,
            priority: 128, // Default
            total_cycles: 0,
            last_tsc: 0,
            page_count: 0,
            signals: 0,
            syscall_count: 0,
            canary: STACK_CANARY_MAGIC,
            pgid: 0,
            sid: 0,
            signal_mask: 0,
            pending_signals: 0,
            cwd: alloc::string::String::from("/"),
            environ: alloc::vec::Vec::new(),
            cell,
            handle_table: Mutex::new(HandleTable::new()),
            name: alloc::string::String::from("unknown"),
            sigactions: [crate::process::signal::SigAction::default(); 64],
            brk: 0x0000000040000000, // Reasonable start for heap
            mmap_base: 0x0000700000000000,
            ruid: 0,
            euid: 0,
            rgid: 0,
            egid: 0,
            umask: 0o022,
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        // AddressSpace is dropped automatically by Arc/Drop.
        // We removed the manual `vmm::destroy_address_space(self.cr3)` call.
    }
}
