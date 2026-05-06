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
    pub canary: u64,
    pub room: Arc<crate::process::room::RoomContext>,
    pub handle_table: Mutex<HandleTable>,
    
    // --- Phase 7: Wasm Orchestration ---
    pub wasm_fuel: u64,
    pub wasm_state: Option<alloc::sync::Arc<Mutex<crate::wasm::WasmProcessState>>>,
}

impl KernelObject for Task {
    fn name(&self) -> alloc::string::String {
        alloc::format!("task-{}", self.id)
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
    pub fn new_free(room: Arc<crate::process::room::RoomContext>) -> Self {
        Self {
            id: 0,
            context: Context { rsp:0, r15:0, r14:0, r13:0, r12:0, rbx:0, rbp:0, rip:0 },
            state: TaskState::Free,
            stack: alloc::vec![0; 16384].into_boxed_slice(),
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
            total_cycles: 0,
            last_tsc: 0,
            page_count: 0,
            signals: 0,
            syscall_count: 0,
            canary: STACK_CANARY_MAGIC,
            room,
            handle_table: Mutex::new(HandleTable::new()),
            wasm_fuel: 0,
            wasm_state: None,
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        if self.cr3 != 0 {
            unsafe {
                crate::mm::vmm::destroy_address_space(self.cr3);
            }
        }
    }
}
