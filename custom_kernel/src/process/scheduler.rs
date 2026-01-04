use spin::Mutex;
use crate::process::task::{Task, TaskState, Context};
use crate::process::switch::__switch;

const MAX_TASKS: usize = 32;

static TASKS: Mutex<[Task; MAX_TASKS]> = Mutex::new([Task {
    id: 0,
    context: Context { rsp:0, r15:0, r14:0, r13:0, r12:0, rbx:0, rbp:0, rip:0 },
    state: TaskState::Free,
    stack: [0; 4096],
    cr3: 0,
    user_stack_top: 0,
}; MAX_TASKS]);

static mut CURRENT_PID: usize = 0;

pub fn init() {
    let mut tasks = TASKS.lock();
    // Initialize Task 0 as the current running kernel task
    tasks[0].id = 0;
    tasks[0].state = TaskState::Running;
    // Context is irrelevant for running task, it gets saved on switch
}

pub fn spawn(func: extern "C" fn()) {
    let mut tasks = TASKS.lock();
    for (i, task) in tasks.iter_mut().enumerate() {
        if task.state == TaskState::Free {
            task.id = i;
            task.state = TaskState::Ready;
            
            // Setup Stack
            // Pointer to end of stack (stack grows down)
            let stack_top = task.stack.as_ptr() as u64 + 4096;
            // Align 16
            let mut sp = stack_top & !0xF;
            
            // We need to push 'func' address as the return address for 'ret' in __switch
            // __switch does: mov rsp, [ctx]; ret
            // So [rsp] must be RIP.
            unsafe {
                sp -= 8;
                *(sp as *mut u64) = func as u64;
            }
            
            task.context.rsp = sp;
            // rip in context is unused by __switch, it relies on stack
            
            return;
        }
    }
    // panic!("No free slots");
}

pub fn schedule() {
    let mut tasks = TASKS.lock();
    let current_pid = unsafe { CURRENT_PID };
    
    // Simple Round Robin
    let mut next_pid = current_pid;
    loop {
        next_pid = (next_pid + 1) % MAX_TASKS;
        if next_pid == current_pid {
            return; // No other tasks
        }
        
        if tasks[next_pid].state == TaskState::Ready {
            break;
        }
    }
    
    // Switch
    let old_pid = current_pid;
    unsafe { CURRENT_PID = next_pid; }
    
    let old_task_ptr = &mut tasks[old_pid].context as *mut Context;
    let next_task_ptr = &tasks[next_pid].context as *const Context;
    
    tasks[next_pid].state = TaskState::Running;
    if tasks[old_pid].state == TaskState::Running {
        tasks[old_pid].state = TaskState::Ready;
    }
    
    // Check CR3 / Address Space
    let next_cr3 = tasks[next_pid].cr3;
    
    // Switch CR3 if user task (cr3 != 0)
    // Note: We should technically switch even for kernel tasks if they have distinct CR3s (not implemented yet)
    if next_cr3 != 0 {
        unsafe {
             core::arch::asm!("mov cr3, {}", in(reg) next_cr3);
        }
    } else {
        // Switch to Kernel CR3 (we need to know it, or just assume we are in it?)
        // For now, assume we stay in whatever CR3 if next is kernel task (usually running in previous user map is risky but ok for simple kernel)
        // Better: Switch to base kernel CR3.
        // TODO: Get kernel CR3. Assuming we don't need to switch back for simple kthreads for now.
    }

    drop(tasks); // Unlock before switch!
    
    unsafe {
        __switch(old_task_ptr, next_task_ptr);
    }
}

pub fn yield_now() {
    schedule();
}

pub fn spawn_user(rip: u64, rsp: u64, cr3: u64) {
    let mut tasks = TASKS.lock();
    for (i, task) in tasks.iter_mut().enumerate() {
        if task.state == TaskState::Free {
            task.id = i;
            task.state = TaskState::Ready;
            task.cr3 = cr3;
            task.user_stack_top = rsp;
            
            // Kernel Stack Top
            let kstack_top = task.stack.as_ptr() as u64 + 4096;
            
            // Set shim entry
            let mut sp = kstack_top & !0xF;
            
            unsafe {
                sp -= 8;
                *(sp as *mut u64) = kernel_shim_entry as u64; // Return address
            }
            
            task.context.rsp = sp;
            // Pass User RIP and RSP via R12, R13
            task.context.r12 = rip;
            task.context.r13 = rsp;
            
            return;
        }
    }
}

pub fn kernel_shim_entry() {
    // We are now active kernel task.
    // Registers R12, R13 hold User RIP, RSP.
    let rip: u64;
    let rsp: u64;
    unsafe {
        core::arch::asm!("mov {}, r12", out(reg) rip);
        core::arch::asm!("mov {}, r13", out(reg) rsp);
        
        crate::cpu::userspace::enter_userspace(rip, rsp);
    }
}
