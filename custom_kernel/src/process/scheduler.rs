use core::arch::asm;
use spin::Mutex;
use crate::process::task::{Task, TaskState, Context};
use crate::process::switch::__switch;

pub const MAX_TASKS: usize = 32;

pub static TASKS: Mutex<[Option<Task>; MAX_TASKS]> = Mutex::new([const { None }; MAX_TASKS]);

static mut TICKS: u64 = 0;

pub fn init() {
    let mut tasks = TASKS.lock();
    // Initialize Task 0 as the current running kernel task
    let mut task0 = Task::new_free();
    task0.id = 0;
    task0.state = TaskState::Running;
    tasks[0] = Some(task0);
}

pub fn tick() {
    unsafe { 
        TICKS += 1; 
        
        // Heartbeat dot every 100 ticks (approx 1s)
        if TICKS % 100 == 0 {
            asm!("out dx, al", in("dx") 0x3F8, in("al") b'.' as u8, options(nomem, nostack, preserves_flags));
        }
    }
    
    // Account CPU Time (Mature Metering)
    let current_tsc = crate::cpu::cpuid::rdtsc();
    let mut tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
    if let Some(task) = &mut tasks[current_pid] {
        if task.state == TaskState::Running {
            task.cpu_time_ticks += 1;
            
            // Cycle Accounting
            if task.last_tsc != 0 {
                let delta = current_tsc.wrapping_sub(task.last_tsc);
                task.total_cycles += delta;
            }
            task.last_tsc = current_tsc;
        }
    }
    drop(tasks);
    
    // Find next task to run
    crate::net::poll();
    schedule();
}

pub fn post_signal(pid: usize, sig: u32) -> isize {
    let mut tasks = TASKS.lock();
    if let Some(task) = &mut tasks[pid] {
        task.signals |= 1 << sig;
        return 0;
    }
    -1
}

pub fn check_current_signal(sig: u32) -> bool {
    let mut tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
    if let Some(task) = &mut tasks[current_pid] {
        if (task.signals >> sig) & 1 == 1 {
            // Consume the signal if it's not a kill signal?
            // Usually SIGINT is consumed. SIGKILL is not.
            if sig != crate::process::task::SIGKILL {
                task.signals &= !(1 << sig);
            }
            return true;
        }
    }
    false
}

pub fn set_priority(pid: usize, priority: u8) -> isize {
    let mut tasks = TASKS.lock();
    if let Some(task) = &mut tasks[pid] {
        task.priority = priority;
        return 0;
    }
    -1
}

pub fn get_ticks() -> u64 {
    unsafe { TICKS }
}

pub fn get_current_pid() -> usize {
    crate::cpu::smp::get_current_pid()
}

pub fn spawn(func: extern "C" fn()) {
    let mut tasks = TASKS.lock();
    for i in 0..MAX_TASKS {
        if tasks[i].is_none() || tasks[i].as_ref().unwrap().state == TaskState::Free {
            let mut task = Task::new_free();
            task.id = i;
            task.state = TaskState::Ready;
            
            // Setup Stack
            let stack_top = task.stack.as_ptr() as u64 + 16384;
            let mut sp = stack_top & !0xF;
            unsafe {
                sp -= 8;
                *(sp as *mut u64) = kernel_thread_entry as u64; // Return to shim
            }
            task.context.rsp = sp;
            task.context.r12 = func as u64; // Pass function in R12 (callee saved, restored by switch)
            
            tasks[i] = Some(task);
            return;
        }
    }
}

pub extern "C" fn kernel_thread_entry() {
    unsafe {
        // Enable Interrupts!
        core::arch::asm!("sti");
        
        // Get function from R12
        let func: extern "C" fn();
        core::arch::asm!("mov {}, r12", out(reg) func);
        
        // Call it
        func();
    }
    
    // Exit
    exit_current_task(0);
}

pub fn clone_task(entry: u64, stack_ptr: u64) -> isize {
    let mut tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
    
    // 1. Get current task state to copy
    let (cr3, caps, fds) = if let Some(current) = &tasks[current_pid] {
        (current.cr3, current.caps.clone(), current.fds.clone())
    } else {
        return -1;
    };

    // 2. Find free slot
    for i in 0..MAX_TASKS {
        if tasks[i].is_none() || tasks[i].as_ref().unwrap().state == TaskState::Free {
             let mut task = Task::new_free();
            task.id = i;
            task.state = TaskState::Ready;
            task.cr3 = cr3; // Shared Memory
            task.caps = caps; // Inherit Caps
            task.fds = fds; // Clone FDs (Note: Separate table copy, not shared! Thread semantics vary)
            // For true threads, FDs should be shared.
            // But our FdTable is inline struct.
            // We'll treat this as "Process sharing memory" (like CLONE_VM).
            
            task.userspace_stack_top = stack_ptr;
            
            // Setup Kernel Stack for Return to User
            let kstack_top = task.stack.as_ptr() as u64 + 4096;
            let mut sp = kstack_top & !0xF;
            unsafe {
                sp -= 8;
                *(sp as *mut u64) = kernel_shim_entry as u64;
            }
            task.context.rsp = sp;
            task.context.r12 = entry;   // Shim R12 -> RIP
            task.context.r13 = stack_ptr; // Shim R13 -> RSP
            
            tasks[i] = Some(task);
            return i as isize;
        }
    }
    
    -1 // No free slots
}

pub fn spawn_user(rip: u64, rsp: u64, cr3: u64) -> usize {
    let mut tasks = TASKS.lock();
    for i in 0..MAX_TASKS {
        if tasks[i].is_none() || tasks[i].as_ref().unwrap().state == TaskState::Free {
             let mut task = Task::new_free();
            task.id = i;
            task.state = TaskState::Ready;
            task.cr3 = cr3;
            task.userspace_stack_top = rsp;
            
            let kstack_top = task.stack.as_ptr() as u64 + 4096;
            let mut sp = kstack_top & !0xF;
            unsafe {
                sp -= 8;
                *(sp as *mut u64) = kernel_shim_entry as u64;
            }
            task.context.rsp = sp;
            task.context.r12 = rip;
            task.context.r13 = rsp;
            
            tasks[i] = Some(task);
            return i;
        }
    }
    0 // Error PID
}

pub fn kernel_shim_entry() {
    let rip: u64;
    let rsp: u64;
    unsafe {
        crate::drivers::video::put_str("Shim Entry\n");
        core::arch::asm!("mov {}, r12", out(reg) rip);
        core::arch::asm!("mov {}, r13", out(reg) rsp);
        
        crate::cpu::userspace::enter_userspace(rip, rsp);
    }
}

pub fn schedule() {
    let mut tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
    
    // Simple Round Robin with Priority Bias (Fake)
    // Real implementation would look for higest priority Ready task.
    // Here we just skip low priority tasks occasionally? 
    // Or just simple Round Robin for now to keep it stable, but we store the priority.
    let mut next_pid = current_pid;
    loop {
        next_pid = (next_pid + 1) % MAX_TASKS;
        if next_pid == current_pid {
            return; // No other tasks
        }
        
        if let Some(task) = &mut tasks[next_pid] {
            // --- Maturity Check: Signals ---
            let signals = task.signals;
            if (signals >> crate::process::task::SIGKILL) & 1 == 1 {
                task.state = TaskState::Zombie;
                task.exit_code = -9;
                continue;
            }
            if (signals >> crate::process::task::SIGINT) & 1 == 1 {
                task.state = TaskState::Zombie;
                task.exit_code = -2;
                continue;
            }
            if (signals >> crate::process::task::SIGTSTP) & 1 == 1 {
                task.state = TaskState::Waiting;
                continue;
            }

            if task.state == TaskState::Ready {
               // Check Sleep
               let current_ticks = unsafe { TICKS };
               if task.sleep_ticks > current_ticks {
                   // Still sleeping
               } else {
                   // Wake up or Ready
                   break;
               }
            }
        }
    }
    
    // Switch
    let old_pid = current_pid;
    crate::cpu::smp::set_current_pid(next_pid);

    let current_tsc = crate::cpu::cpuid::rdtsc();
    
    // Safety: we know old_pid != next_pid.
    let tasks_ptr = tasks.as_mut_ptr();
    let old_task = unsafe { (*tasks_ptr.add(old_pid)).as_mut().unwrap() };
    
    // Account final cycles for old task
    if old_task.last_tsc != 0 {
        old_task.total_cycles += current_tsc.wrapping_sub(old_task.last_tsc);
    }
    old_task.last_tsc = 0; // Clear on deschedule

    let next_task = unsafe { (*tasks_ptr.add(next_pid)).as_mut().unwrap() };
    next_task.state = TaskState::Running;
    next_task.last_tsc = current_tsc; // Mark start on reschedule

    if old_task.state == TaskState::Running {
        old_task.state = TaskState::Ready;
    }
    
    // Switch CR3
    let next_cr3 = next_task.cr3;
    if next_cr3 != 0 {
        unsafe {
             core::arch::asm!("mov cr3, {}", in(reg) next_cr3);
        }
    }
    
    // Update TSS RSP0
    let kstack_top = next_task.stack.as_ptr() as u64 + next_task.stack.len() as u64;
    unsafe {
        crate::cpu::gdt::set_kernel_stack(kstack_top);
    }
    
    // Maturation: Final pointers for switch
    let old_task_ptr = &mut old_task.context as *mut Context;
    let next_task_ptr = &next_task.context as *const Context;
    
    drop(tasks); 
    
    unsafe {
        __switch(old_task_ptr, next_task_ptr);
    }
}

pub fn yield_now() {
    schedule();
}

pub unsafe fn set_current_sleep(target_ticks: u64) {
    let mut tasks = TASKS.lock();
    if let Some(task) = &mut tasks[crate::cpu::smp::get_current_pid()] {
        task.sleep_ticks = target_ticks;
    }
}

pub fn process_open(path: &str, _flags: u32) -> isize {
    // 1. Lookup file in VFS (TODO: Parse path, currently just verify ROOT exists)
    // For now, fail if not implemented.
    // In Phase 13b, we will walk ROOT.lookup(path).
    
    // Stub: Always fail for now until EXT4 attached.
    -1
}

pub fn process_read(fd: usize, buf: &mut [u8]) -> isize {
    let mut tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
    
    // 1. Get Handle and Offset
    let (handle, offset): (crate::fs::vfs::ArcHandle, u64) = if let Some(task) = &tasks[current_pid] {
         match task.fds.get_entry(fd) {
             Some(entry) => (entry.handle.clone(), entry.offset),
             None => return -1,
         }
    } else {
        return -1;
    };
    
    // 2. Drop lock to perform I/O
    drop(tasks);
    
    // 3. Perform Read
    match handle.read(buf, offset) {
        Ok(n) => {
            // 4. Update Offset (Re-acquire lock)
             let mut tasks = TASKS.lock();
             if let Some(task) = &mut tasks[current_pid] {
                 task.fds.update_offset(fd, offset + n as u64);
             }
             n as isize
        },
        Err(_) => -1
    }
}

pub fn process_write(fd: usize, buf: &[u8]) -> isize {
    let mut tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
    
    // 1. Get Handle and Offset
    let (handle, offset): (crate::fs::vfs::ArcHandle, u64) = if let Some(task) = &tasks[current_pid] {
         match task.fds.get_entry(fd) {
             Some(entry) => (entry.handle.clone(), entry.offset),
             None => return -1, // Invalid FD
         }
    } else {
        return -1;
    };
    
    // 2. Drop lock to perform I/O
    drop(tasks);
    
    // 3. Perform Write
    match handle.write(buf, offset) {
        Ok(n) => {
            // 4. Update Offset (Re-acquire lock)
             let mut tasks = TASKS.lock();
             if let Some(task) = &mut tasks[current_pid] {
                 task.fds.update_offset(fd, offset + n as u64);
             }
             n as isize
        },
        Err(_) => -1
    }
}

pub fn process_close(fd: usize) -> isize {
    let mut tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
     if let Some(task) = &mut tasks[current_pid] {
         task.fds.free_fd(fd);
         0
     } else {
         -1
     }
}

pub fn exit_current_task(exit_code: isize) {
    let mut tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
    
    if let Some(task) = &mut tasks[current_pid] {
         task.state = TaskState::Zombie;
         task.exit_code = exit_code;
    }
    
    drop(tasks);
    schedule();
}

pub fn wait_pid(pid: usize) -> isize {
    loop {
        let mut tasks = TASKS.lock();
        if let Some(child) = &tasks[pid] {
            if child.state == TaskState::Zombie {
                let code = child.exit_code;
                tasks[pid] = None; // Reap
                return code;
            }
        } else {
            return -1;
        }
        drop(tasks);
        yield_now();
    }
}

pub fn kill_task(pid: usize) -> isize {
    let mut tasks = TASKS.lock();
    if pid == 0 { return -1; } // Cannot kill kernel
    
    if let Some(task) = &mut tasks[pid] {
        if task.state != TaskState::Free && task.state != TaskState::Zombie {
             task.state = TaskState::Zombie;
             task.exit_code = -9; // SIGKILL equivalent
             return 0;
        }
    }
    -1
}

pub fn print_task_list() {
    // Critical Section: Disable Interrupts to prevent deadlock with Tick
    let flags = crate::cpu::control::save_cpu_flags();
    unsafe { core::arch::asm!("cli", options(nomem, nostack)); }

    {
        let tasks = TASKS.lock();
        crate::drivers::video::put_str("PID  State    CPU Ticks\n");
        for i in 0..MAX_TASKS {
            if let Some(task) = &tasks[i] {
                let state_str = match task.state {
                    TaskState::Running => "Running",
                    TaskState::Ready => "Ready  ",
                    TaskState::Waiting => "Waiting",
                    TaskState::Free => "Free   ",
                    TaskState::Zombie => "Zombie ",
                };
                
                // PID
                crate::drivers::video::put_char((b'0' + task.id as u8) as char); 
                crate::drivers::video::put_str("    ");
                crate::drivers::video::put_str(state_str);
                crate::drivers::video::put_str("  ");
                
                // CPU Time (Very basic for now)
                let ticks = task.cpu_time_ticks;
                if ticks > 0 {
                    crate::drivers::video::put_char(if ticks > 100 { '+' } else { '.' });
                }
                crate::drivers::video::put_char('\n');
            }
        }
    } // Unlock matches here

    unsafe { crate::cpu::control::restore_cpu_flags(flags); }
}

pub fn block_current_task() {
    let mut tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
    
    if let Some(task) = &mut tasks[current_pid] {
        task.state = TaskState::Waiting;
    }
    drop(tasks);
    schedule();
}

pub fn wake_task(pid: usize) {
    let mut tasks = TASKS.lock();
    if let Some(task) = &mut tasks[pid] {
         if task.state == TaskState::Waiting {
             task.state = TaskState::Ready;
         }
    }
}

pub fn increment_current_page_count() {
    let mut tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
    if current_pid < MAX_TASKS {
        if let Some(task) = &mut tasks[current_pid] {
            task.page_count += 1;
        }
    }
}

// --- Phase 30: Thread Management Extensions ---

pub fn suspend_thread(pid: usize) {
    let mut tasks = TASKS.lock();
    if let Some(task) = &mut tasks[pid] {
        if task.state == TaskState::Ready || task.state == TaskState::Running {
            task.state = TaskState::Waiting;
        }
    }
}

pub fn resume_thread(pid: usize) {
    let mut tasks = TASKS.lock();
    if let Some(task) = &mut tasks[pid] {
        if task.state == TaskState::Waiting {
             task.state = TaskState::Ready;
        }
    }
}

pub fn bind_thread_cpu(pid: usize, _cpu: usize) {
    // Stub for future SMP
    // Validate PID existence
    let tasks = TASKS.lock();
    if tasks[pid].is_some() {
        // Log or store affinity
    }
}

pub fn reap_any_zombie() {
    let mut tasks = TASKS.lock();
    for i in 1..MAX_TASKS { // Don't reap kernel (0)
        if let Some(task) = &tasks[i] {
            if task.state == TaskState::Zombie {
                // In real OS, only parent can reap.
                // But if parent died, Init inherits.
                // Here we just allow global cleanup for "orphan" like behavior
                tasks[i] = None;
            }
        }
    }
}

pub fn detect_starvation() {
    // Scan for Ready tasks that are at risk
    // Simple stub for now
    let tasks = TASKS.lock();
    for i in 0..MAX_TASKS {
        if let Some(task) = &tasks[i] {
            if task.state == TaskState::Ready {
               // If ready and not run for long time...
               // Needed: last_run_tick in Task struct
            }
        }
    }
}

pub fn detect_priority_inversion() {
     // Stub
}
