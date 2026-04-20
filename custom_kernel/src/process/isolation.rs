use crate::mm::user::{USER_HEAP, UserError};
use crate::drivers::video;
use crate::process::scheduler;

/// Represents an isolated execution environment (Sandbox).
/// Inspired by "Clear Linux" isolation methodologies.
pub struct Sandbox {
    pub id: usize,
    pub quota_cycles: u64,
    pub used_cycles: u64,
    pub memory_base: u64,
    pub memory_size: usize,
}

impl Sandbox {
    pub fn new(id: usize, memory_size: usize) -> Result<Self, UserError> {
        let mut heap = USER_HEAP.lock();
        let base = heap.allocate(memory_size)?;
        
        Ok(Self {
            id,
            quota_cycles: 1_000_000_000, // 1 billion cycles default quota
            used_cycles: 0,
            memory_base: base,
            memory_size,
        })
    }

    /// Runs a function in this sandbox using Ring 3 isolation.
    pub fn run(&mut self, entry_point: u64) {
        video::put_str(&format!("[Isolation] Entering Sandbox {} (Base: {:#x})\n", self.id, self.memory_base));
        
        // Setup User Stack at the end of the allocated memory
        let user_stack = self.memory_base + self.memory_size as u64 - 16;

        unsafe {
            // We use the existing scheduler infrastructure to "spawn" this as a user task
            // but for immediate execution in this context, we'd use enter_userspace.
            // To be truly non-bricking, it MUST be a separate task.
            
            let pid = scheduler::spawn_user(entry_point, user_stack, 0); // 0 = Current Kernel CR3
            video::put_str(&format!("[Isolation] Spawned PID {} for isolated execution.\n", pid));
            
            // The load balancer (WFS) will now handle this task fairly.
        }
    }
}

/// Methodology helper to run a closure or bytecode safely.
pub fn run_isolated_task(id: usize, size: usize, entry: u64) {
    if let Ok(mut sb) = Sandbox::new(id, size) {
        sb.run(entry);
    } else {
        video::put_str("[Isolation] Failed to allocate isolated space.\n");
    }
}
