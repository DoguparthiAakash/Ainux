#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SigAction {
    pub sa_handler: u64,
    pub sa_flags: u64,
    pub sa_restorer: u64,
    pub sa_mask: u64,
}

impl Default for SigAction {
    fn default() -> Self {
        Self {
            sa_handler: 0,
            sa_flags: 0,
            sa_restorer: 0,
            sa_mask: 0,
        }
    }
}

pub const SIGKILL: u8 = 9;
pub const SIGSTOP: u8 = 19;
pub const SIGCONT: u8 = 18;

// Called from syscall handler or interrupt handler before returning to userspace
#[no_mangle]
pub extern "C" fn handle_pending_signals(state: *mut crate::process::scheduler::SyscallState) {
    // For now, just handle SIGKILL directly as an example.
    let current_pid = crate::process::scheduler::get_current_pid();
    let mut kill_task = false;
    
    crate::cpu::without_interrupts(|| {
        let mut tasks = crate::process::scheduler::TASKS.lock();
        if let Some(task) = &mut tasks[current_pid] {
            let pending = task.pending_signals & !task.signal_mask;
            if pending != 0 {
                // If SIGKILL is pending, kill immediately
                if (pending & (1 << SIGKILL)) != 0 {
                    kill_task = true;
                }
            }
        }
    });
    
    if kill_task {
        crate::process::scheduler::exit_current_task(-9);
    }
}
