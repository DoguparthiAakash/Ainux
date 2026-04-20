use core::arch::asm;
use spin::Mutex;
use crate::process::task::{Task, TaskState, Context};
use crate::process::switch::__switch;

pub const MAX_TASKS: usize = 32;

pub static TASKS: Mutex<Option<alloc::boxed::Box<[Task; MAX_TASKS]>>> = Mutex::new(None);

static mut TICKS: u64 = 0;

pub fn get_ticks() -> u64 {
    unsafe { TICKS }
}

pub fn init() {
    let mut tasks_lock = TASKS.lock();
    
    // 1. Allocate the task table on the heap to avoid BSS/Stack bloat
    // We use a vector and convert to boxed slice to ensure heap allocation
    let mut v = alloc::vec::Vec::with_capacity(MAX_TASKS);
    for i in 0..MAX_TASKS {
        let mut t = Task::new_empty(); // Minimal stack impact
        t.reset(i);
        v.push(t);
    }
    let mut tasks = v.into_boxed_slice();
    
    // 2. Initialize Task 0 as the current running kernel task
    tasks[0].state = TaskState::Running;
    
    let ptr = alloc::boxed::Box::into_raw(tasks) as *mut [Task; MAX_TASKS];
    *tasks_lock = Some(unsafe { alloc::boxed::Box::from_raw(ptr) });
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
    
    // Safety: tick is called from IRQ context, but we use without_interrupts for consistency
    // and to handle potential nested interrupt scenarios.
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            let current_pid = crate::cpu::smp::get_current_pid();
            let task = &mut tasks[current_pid];
            if task.state == TaskState::Running {
                task.cpu_time_ticks += 1;
                
                if task.last_tsc != 0 {
                    let delta = current_tsc.wrapping_sub(task.last_tsc);
                    task.total_cycles += delta;
                    let weight = task.priority as u64;
                    task.vruntime += (delta * weight) / 128;
                }
                task.last_tsc = current_tsc;
            }
        }
    });
    
    // Find next task to run
    crate::net::poll();
    schedule();
}

pub fn post_signal(pid: usize, sig: u32) -> isize {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            let task = &mut tasks[pid];
            task.signals |= 1 << sig;
            return 0;
        }
        -1
    })
}

pub fn check_current_signal(sig: u32) -> bool {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            let current_pid = crate::cpu::smp::get_current_pid();
            let task = &mut tasks[current_pid];
            if (task.signals >> sig) & 1 == 1 {
                if sig != crate::process::task::SIGKILL {
                    task.signals &= !(1 << sig);
                }
                return true;
            }
        }
        false
    })
}

pub fn set_priority(pid: usize, priority: u8) -> isize {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            tasks[pid].priority = priority;
            return 0;
        }
        -1
    })
}
pub fn get_current_pid() -> usize {
    crate::cpu::smp::get_current_pid()
}

pub fn spawn(func: extern "C" fn()) {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            for i in 0..MAX_TASKS {
                if tasks[i].state == TaskState::Free {
                    // 1. Reset the existing task slot in-place (ZERO stack copy)
                    let current_pid = crate::cpu::smp::get_current_pid();
                    let current_vruntime = tasks[current_pid].vruntime;
                    
                    let t = &mut tasks[i];
                    t.reset(i);
                    t.state = TaskState::Ready;
                    t.vruntime = current_vruntime;
                    
                    // 2. Setup Context
                    let stack = t.stack.as_ref().unwrap();
                    let stack_size = 32768;
                    let stack_top = stack.as_ptr() as u64 + stack_size as u64;
                    let mut sp = stack_top & !0xF;
                    
                    unsafe {
                        sp -= 8;
                        *(sp as *mut u64) = kernel_thread_entry as u64;
                    }
                    
                    t.context.rsp = sp;
                    t.context.r12 = func as u64;
                    return;
                }
            }
        }
    });
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
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            let current_pid = crate::cpu::smp::get_current_pid();
            let (cr3, caps, fds) = (tasks[current_pid].cr3, tasks[current_pid].caps.clone(), tasks[current_pid].fds.clone());

            for i in 0..MAX_TASKS {
                if tasks[i].state == TaskState::Free {
                    let task = &mut tasks[i];
                    task.reset(i);
                    task.state = TaskState::Ready;
                    task.cr3 = cr3;
                    task.caps = caps;
                    task.fds = fds;
                    task.userspace_stack_top = stack_ptr;
                    
                    let stack = task.stack.as_ref().unwrap();
                    let kstack_top = stack.as_ptr() as u64 + 32768;
                    let mut sp = kstack_top & !0xF;
                    unsafe {
                        sp -= 8;
                        *(sp as *mut u64) = kernel_shim_entry as u64;
                    }
                    task.context.rsp = sp;
                    task.context.r12 = entry;
                    task.context.r13 = stack_ptr;
                    return i as isize;
                }
            }
        }
        -1
    })
}

pub fn spawn_user(rip: u64, rsp: u64, cr3: u64) -> usize {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            for i in 0..MAX_TASKS {
                if tasks[i].state == TaskState::Free {
                    let task = &mut tasks[i];
                    task.reset(i);
                    task.state = TaskState::Ready;
                    task.cr3 = cr3;
                    task.userspace_stack_top = rsp;
                    
                    let stack = task.stack.as_ref().unwrap();
                    let kstack_top = stack.as_ptr() as u64 + 32768;
                    let mut sp = kstack_top & !0xF;
                    unsafe {
                        sp -= 8;
                        *(sp as *mut u64) = kernel_shim_entry as u64;
                    }
                    task.context.rsp = sp;
                    task.context.r12 = rip;
                    task.context.r13 = rsp;
                    return i;
                }
            }
        }
        0
    })
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
    let flags = crate::cpu::control::save_cpu_flags();
    crate::cpu::control::cli();

    let mut lock = TASKS.lock();
    if let Some(tasks) = lock.as_mut() {
        let current_pid = crate::cpu::smp::get_current_pid();
        
        let mut best_pid = None;
        let mut min_vruntime = u64::MAX;

        for i in 0..MAX_TASKS {
            let task = &mut tasks[i];
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

            if task.state == TaskState::Ready {
               let current_ticks = unsafe { TICKS };
               if task.sleep_ticks > current_ticks {
                   continue;
               }

               if task.vruntime < min_vruntime {
                   min_vruntime = task.vruntime;
                   best_pid = Some(i);
               }
            }
        }

        let next_pid = if let Some(pid) = best_pid {
            pid
        } else {
            drop(lock);
            unsafe { crate::cpu::control::restore_cpu_flags(flags); }
            return; 
        };
        
        let old_pid = current_pid;
        crate::cpu::smp::set_current_pid(next_pid);

        let current_tsc = crate::cpu::cpuid::rdtsc();
        
        // Safety: We use indexing into the heap-allocated tasks array
        let next_task = &mut tasks[next_pid];
        let next_cr3 = next_task.cr3;
        let stack = next_task.stack.as_ref().unwrap();
        let kstack_top = stack.as_ptr() as u64 + 32768;
        let next_task_ptr = &next_task.context as *const Context;
        next_task.state = TaskState::Running;
        next_task.last_tsc = current_tsc;

        let old_task = &mut tasks[old_pid];
        if old_task.last_tsc != 0 {
            old_task.total_cycles += current_tsc.wrapping_sub(old_task.last_tsc);
        }
        old_task.last_tsc = 0; 
        if old_task.state == TaskState::Running {
            old_task.state = TaskState::Ready;
        }
        let old_task_ptr = &mut old_task.context as *mut Context;
        
        // Switch CR3/TSS
        if next_cr3 != 0 { unsafe { core::arch::asm!("mov cr3, {}", in(reg) next_cr3); } }
        unsafe { crate::cpu::gdt::set_kernel_stack(kstack_top); }
        
        drop(lock);
        
        unsafe {
            __switch(old_task_ptr, next_task_ptr);
            crate::cpu::control::restore_cpu_flags(flags);
        }
    } else {
        drop(lock);
        unsafe { crate::cpu::control::restore_cpu_flags(flags); }
    }
}

pub fn yield_now() {
    schedule();
}

pub unsafe fn set_current_sleep(target_ticks: u64) {
    let mut lock = TASKS.lock();
    if let Some(tasks) = lock.as_mut() {
        let current_pid = crate::cpu::smp::get_current_pid();
        tasks[current_pid].sleep_ticks = target_ticks;
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
    let mut lock = TASKS.lock();
    if let Some(tasks) = lock.as_mut() {
        let current_pid = crate::cpu::smp::get_current_pid();
        let task = &mut tasks[current_pid];
        
        let (handle, offset) = match task.fds.get_entry(fd) {
            Some(entry) => (entry.handle.clone(), entry.offset),
            None => return -1,
        };
        
        drop(lock);
        
        match handle.read(buf, offset) {
            Ok(n) => {
                 let mut lock = TASKS.lock();
                 if let Some(tasks) = lock.as_mut() {
                     tasks[current_pid].fds.update_offset(fd, offset + n as u64);
                 }
                 n as isize
            },
            Err(_) => -1
        }
    } else { -1 }
}

pub fn process_write(fd: usize, buf: &[u8]) -> isize {
    let current_pid = crate::cpu::smp::get_current_pid();
    let mut lock = TASKS.lock();
    
    let (handle, offset) = if let Some(tasks) = lock.as_mut() {
        let task = &mut tasks[current_pid];
        match task.fds.get_entry(fd) {
            Some(entry) => (entry.handle.clone(), entry.offset),
            None => return -1,
        }
    } else { return -1 };
    
    drop(lock);
    
    match handle.write(buf, offset) {
        Ok(n) => {
             let mut lock = TASKS.lock();
             if let Some(tasks) = lock.as_mut() {
                 tasks[current_pid].fds.update_offset(fd, offset + n as u64);
             }
             n as isize
        },
        Err(_) => -1
    }
}

pub fn process_close(fd: usize) -> isize {
    let current_pid = crate::cpu::smp::get_current_pid();
    let mut lock = TASKS.lock();
    if let Some(tasks) = lock.as_mut() {
        tasks[current_pid].fds.free_fd(fd);
        0
    } else { -1 }
}

pub fn exit_current_task(exit_code: isize) {
    let mut lock = TASKS.lock();
    if let Some(tasks) = lock.as_mut() {
        let current_pid = crate::cpu::smp::get_current_pid();
        let task = &mut tasks[current_pid];
        task.state = TaskState::Zombie;
        task.exit_code = exit_code;
    }
    drop(lock);
    schedule();
}

pub fn wait_pid(pid: usize) -> isize {
    loop {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            let child = &mut tasks[pid];
            if child.state == TaskState::Zombie {
                let code = child.exit_code;
                child.reset(pid); // Reap and reset in-place
                return code;
            }
        } else {
            return -1;
        }
        drop(lock);
        yield_now();
    }
}

pub fn kill_task(pid: usize) -> isize {
    let mut lock = TASKS.lock();
    if pid == 0 { return -1; }
    if let Some(tasks) = lock.as_mut() {
        let task = &mut tasks[pid];
        if task.state != TaskState::Free && task.state != TaskState::Zombie {
             task.state = TaskState::Zombie;
             task.exit_code = -9;
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
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            crate::drivers::video::put_str("PID  State    CPU Ticks\n");
            for i in 0..MAX_TASKS {
                let task = &tasks[i];
                if task.state != TaskState::Free {
                    let state_str = match task.state {
                        TaskState::Running => "Running",
                        TaskState::Ready => "Ready  ",
                        TaskState::Waiting => "Waiting",
                        TaskState::Free => "Free   ",
                        TaskState::Zombie => "Zombie ",
                    };
                    
                    crate::drivers::video::put_char((b'0' + task.id as u8) as char); 
                    crate::drivers::video::put_str("    ");
                    crate::drivers::video::put_str(state_str);
                    crate::drivers::video::put_str("  ");
                    
                    let ticks = task.cpu_time_ticks;
                    if ticks > 0 {
                        crate::drivers::video::put_char(if ticks > 100 { '+' } else { '.' });
                    }
                    crate::drivers::video::put_char('\n');
                }
            }
        }
    }

    unsafe { crate::cpu::control::restore_cpu_flags(flags); }
}

pub fn block_current_task() {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            let current_pid = crate::cpu::smp::get_current_pid();
            tasks[current_pid].state = TaskState::Waiting;
        }
    });
    schedule();
}

pub fn wake_task(pid: usize) {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            if tasks[pid].state == TaskState::Waiting {
                tasks[pid].state = TaskState::Ready;
            }
        }
    });
}

pub fn increment_current_page_count() {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            let current_pid = crate::cpu::smp::get_current_pid();
            if current_pid < MAX_TASKS {
                tasks[current_pid].page_count += 1;
            }
        }
    });
}

// --- Phase 30: Thread Management Extensions ---

pub fn suspend_thread(pid: usize) {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            let state = tasks[pid].state;
            if state == TaskState::Ready || state == TaskState::Running {
                tasks[pid].state = TaskState::Waiting;
            }
        }
    });
}

pub fn resume_thread(pid: usize) {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            if tasks[pid].state == TaskState::Waiting {
                 tasks[pid].state = TaskState::Ready;
            }
        }
    });
}
pub fn bind_thread_cpu(pid: usize, _cpu: usize) {
    let lock = TASKS.lock();
    if let Some(tasks) = lock.as_ref() {
        // Validate PID existence
        if tasks[pid].state != TaskState::Free {
            // Valid
        }
    }
}

pub fn reap_any_zombie() {
    crate::cpu::control::without_interrupts(|| {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            for i in 1..MAX_TASKS {
                if tasks[i].state == TaskState::Zombie {
                    tasks[i].reset(i);
                }
            }
        }
    });
}

pub fn detect_starvation() {
    // Scan for Ready tasks that are at risk
    // Simple stub for now
    let lock = TASKS.lock();
    if let Some(tasks) = lock.as_ref() {
        for i in 0..MAX_TASKS {
            let task = &tasks[i];
            if task.state == TaskState::Ready {
               // ...
            }
        }
    }
}

pub fn detect_priority_inversion() {
     // Stub
}
