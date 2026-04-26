use core::arch::{asm, naked_asm};
use spin::Mutex;
use crate::process::task::{Task, TaskState, Context};
use crate::process::switch::__switch;

pub const MAX_TASKS: usize = 32;

pub static TASKS: Mutex<[Option<Task>; MAX_TASKS]> = Mutex::new([const { None }; MAX_TASKS]);

static mut TICKS: u64 = 0;
pub static READY_BITMAP: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
pub static SCHEDULER_INITIALIZED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

fn set_ready(pid: usize) {
    if pid < MAX_TASKS {
        READY_BITMAP.fetch_or(1 << pid, core::sync::atomic::Ordering::Relaxed);
    }
}

fn clear_ready(pid: usize) {
    if pid < MAX_TASKS {
        READY_BITMAP.fetch_and(!(1 << pid), core::sync::atomic::Ordering::Relaxed);
    }
}

pub fn get_ticks() -> u64 {
    unsafe { TICKS }
}

pub fn init() {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        // Initialize Task 0 as the current running kernel task
        let root_room = crate::process::room::ROOM_MANAGER.lock().rooms.first().cloned().expect("No root room");
        let mut task0 = Task::new_free(root_room.clone());
        task0.id = 0;
        task0.state = TaskState::Running;
        tasks[0] = Some(task0);
        // Task 0 is running, not ready
        clear_ready(0);
        SCHEDULER_INITIALIZED.store(true, core::sync::atomic::Ordering::SeqCst);
    });
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
    // 1. Automatic Reap (Every 20 ticks)
    if unsafe { TICKS % 20 == 0 } {
        reap_any_zombie();
    }

    // 2. Poll Network
    crate::net::poll();
    schedule();
}

pub fn post_signal(pid: usize, sig: u32) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[pid] {
            task.signals |= 1 << sig;
            return 0;
        }
        -1
    })
}

pub fn check_current_signal(sig: u32) -> bool {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        if let Some(task) = &mut tasks[current_pid] {
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
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[pid] {
            task.priority = priority;
            return 0;
        }
        -1
    })
}
pub fn get_current_pid() -> usize {
    crate::cpu::smp::get_current_pid()
}

pub fn get_current_room() -> Option<alloc::sync::Arc<crate::process::room::RoomContext>> {
    let pid = get_current_pid();
    let tasks = TASKS.lock();
    tasks[pid].as_ref().map(|t| t.room.clone())
}

pub fn spawn_kernel_task(func: u64) -> usize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        for i in 0..MAX_TASKS {
            if tasks[i].is_none() || tasks[i].as_ref().unwrap().state == TaskState::Free {
                // Root tasks go to Root Room
                let room = crate::process::room::ROOM_MANAGER.lock().rooms.first().cloned().expect("No root room");

                let mut task = Task::new_free(room);
                task.id = i;
                task.state = TaskState::Ready;

                // Setup Stack
                let stack_top = task.stack.as_ptr() as u64 + 16384;
                let mut sp = stack_top & !0xF;
                unsafe {
                    sp -= 8;
                    *(sp as *mut u64) = kernel_thread_entry as u64; 
                }
                task.context.rsp = sp;
                task.context.r12 = func; 
                
                tasks[i] = Some(task);
                set_ready(i);
                return i;
            }
        }
        0
    })
}

pub fn spawn(func: extern "C" fn()) {
    spawn_kernel_task(func as u64);
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
    crate::cpu::without_interrupts(|| {
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
                // Inherit Room
                let room = if let Some(current) = &tasks[current_pid] {
                    current.room.clone()
                } else {
                    crate::process::room::ROOM_MANAGER.lock().rooms.first().cloned().expect("No root room")
                };

                let mut task = Task::new_free(room);
                task.id = i;
                task.state = TaskState::Ready;
                task.cr3 = cr3; // Shared Memory
                task.caps = caps; // Inherit Caps
                task.fds = fds; // Clone FDs 
                
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
                set_ready(i);
                return i as isize;
            }
        }
        -1
    })
}

pub fn spawn_user(rip: u64, rsp: u64, cr3: u64) -> usize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        for i in 0..MAX_TASKS {
            if tasks[i].is_none() || tasks[i].as_ref().unwrap().state == TaskState::Free {
                // Assign Root Room by default for user spawn (can be moved later)
                let root_room = crate::process::room::ROOM_MANAGER.lock().rooms.first().cloned().expect("No root room");
                let mut task = Task::new_free(root_room);
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
                set_ready(i);
                return i;
            }
        }
        0 // Error PID
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
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();

        let next_pid = if let Some(next) = pick_next_task_internal(&mut tasks, current_pid) {
            next
        } else {
             // IDLE STRATEGY: Reduce host CPU load
             unsafe { core::arch::asm!("hlt"); }
             return;
        };
        
        if next_pid == current_pid {
            return;
        }
    
    // Switch
    let old_pid = current_pid;
    crate::cpu::smp::set_current_pid(next_pid);

    let current_tsc = crate::cpu::cpuid::rdtsc();
    
    // Safety: we know old_pid != next_pid.
    let tasks_ptr = tasks.as_mut_ptr();
    let old_task = unsafe { (*tasks_ptr.add(old_pid)).as_mut().unwrap() };
    
    // Security: Check Stack Canary
    if old_task.canary != crate::process::task::STACK_CANARY_MAGIC {
        panic!("FATAL: Kernel Stack Corruption detected in Task {} (Canary Mismatch)", old_pid);
    }
    
    // Account final cycles for old task
    if old_task.last_tsc != 0 {
        old_task.total_cycles += current_tsc.wrapping_sub(old_task.last_tsc);
    }
    old_task.last_tsc = 0; // Clear on deschedule

    let next_task = unsafe { (*tasks_ptr.add(next_pid)).as_mut().unwrap() };
    next_task.state = TaskState::Running;
    next_task.last_tsc = current_tsc; // Mark start on reschedule
    clear_ready(next_pid);

    if old_task.state == TaskState::Running {
        old_task.state = TaskState::Ready;
        set_ready(old_pid);
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
    });
}

fn pick_next_task_internal(tasks: &mut [Option<Task>; MAX_TASKS], current: usize) -> Option<usize> {
    // 1. Get current Room Strategy
    if let Some(task) = &tasks[current] {
        let room = &task.room;
        let strategy = room.strategy.lock().clone();
        if let Some(next) = strategy.pick_next(tasks, current) {
            if let Some(task) = &tasks[next] {
                if task.state == TaskState::Ready {
                    return Some(next);
                }
            }
        }
    }

    // 2. Fallback to global bitmask (Very fast O(1))
    let mask = READY_BITMAP.load(core::sync::atomic::Ordering::Relaxed);
    if mask != 0 {
        let next_pid = mask.trailing_zeros() as usize;
        if next_pid < MAX_TASKS {
            return Some(next_pid);
        }
    }
    
    None
}

pub fn pick_next_task() -> Option<usize> {
    let mut tasks = TASKS.lock();
    pick_next_task_internal(&mut tasks, get_current_pid())
}

pub fn yield_now() {
    schedule();
}

pub unsafe fn set_current_sleep(target_ticks: u64) {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[crate::cpu::smp::get_current_pid()] {
            task.sleep_ticks = target_ticks;
        }
    });
}

pub fn process_pipe() -> (isize, isize) {
    let (reader, writer) = crate::fs::pipe::create_pipe();
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        if let Some(task) = &mut tasks[current_pid] {
            let r_fd = task.fds.alloc_fd(reader);
            let w_fd = task.fds.alloc_fd(writer);
            if let (Some(r), Some(w)) = (r_fd, w_fd) {
                return (r as isize, w as isize);
            }
        }
        (-1, -1)
    })
}

pub fn process_open(path: &str, flags: u32) -> isize {
    if let Ok(inode) = crate::fs::vfs::resolve_path(path) {
        if let Ok(handle) = inode.open(flags) {
            return crate::cpu::without_interrupts(|| {
                let mut tasks = TASKS.lock();
                let current_pid = crate::cpu::smp::get_current_pid();
                if let Some(task) = &mut tasks[current_pid] {
                    if let Some(fd) = task.fds.alloc_fd(handle) {
                        return fd as isize;
                    }
                }
                -1
            });
        }
    }
    -1
}

pub fn process_mkdir(path: &str) -> isize {
    // Basic implementation: split path into parent and name
    let last_slash = path.rfind('/');
    let (parent_path, name) = if let Some(idx) = last_slash {
        let p = if idx == 0 { "/" } else { &path[..idx] };
        (p, &path[idx + 1..])
    } else {
        (".", path)
    };

    if let Ok(parent_inode) = crate::fs::vfs::resolve_path(parent_path) {
        if parent_inode.mkdir(name).is_ok() {
            return 0;
        }
    }
    -1
}

pub fn process_unlink(path: &str) -> isize {
    let last_slash = path.rfind('/');
    let (parent_path, name) = if let Some(idx) = last_slash {
        let p = if idx == 0 { "/" } else { &path[..idx] };
        (p, &path[idx + 1..])
    } else {
        (".", path)
    };

    if let Ok(parent_inode) = crate::fs::vfs::resolve_path(parent_path) {
        if parent_inode.unlink(name).is_ok() {
            return 0;
        }
    }
    -1
}

pub fn process_read(fd: usize, buf: &mut [u8]) -> isize {
    let current_pid = crate::cpu::smp::get_current_pid();
    let result = crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &tasks[current_pid] {
             match task.fds.get_entry(fd) {
                 Some(entry) => Some((entry.handle.clone(), entry.offset)),
                 None => None,
             }
        } else {
            None
        }
    });
    
    if let Some((handle, offset)) = result {
        match handle.read(buf, offset) {
            Ok(n) => {
                crate::cpu::without_interrupts(|| {
                    let mut tasks = TASKS.lock();
                    if let Some(task) = &mut tasks[current_pid] {
                        task.fds.update_offset(fd, offset + n as u64);
                    }
                });
                n as isize
            },
            Err(_) => -1
        }
    } else {
        -1
    }
}

pub fn process_write(fd: usize, buf: &[u8]) -> isize {
    let current_pid = crate::cpu::smp::get_current_pid();
    let result = crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &tasks[current_pid] {
             match task.fds.get_entry(fd) {
                 Some(entry) => Some((entry.handle.clone(), entry.offset)),
                 None => None,
             }
        } else {
            None
        }
    });
    
    if let Some((handle, offset)) = result {
        match handle.write(buf, offset) {
            Ok(n) => {
                crate::cpu::without_interrupts(|| {
                    let mut tasks = TASKS.lock();
                    if let Some(task) = &mut tasks[current_pid] {
                        task.fds.update_offset(fd, offset + n as u64);
                    }
                });
                n as isize
            },
            Err(_) => -1
        }
    } else {
        -1
    }
}

pub fn process_close(fd: usize) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        if let Some(task) = &mut tasks[current_pid] {
            task.fds.free_fd(fd);
            0
        } else {
            -1
        }
    })
}

pub fn exit_current_task(exit_code: isize) {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        
        if let Some(task) = &mut tasks[current_pid] {
             task.state = TaskState::Zombie;
             task.exit_code = exit_code;
             clear_ready(current_pid);
        }
        
        drop(tasks);
        schedule();
    });
}

pub fn wait_pid(pid: usize) -> isize {
    loop {
        let res = crate::cpu::without_interrupts(|| {
            let mut tasks = TASKS.lock();
            if let Some(child) = &tasks[pid] {
                if child.state == TaskState::Zombie {
                    let code = child.exit_code;
                    tasks[pid] = None; // Reap
                    return Some(code);
                }
            } else {
                return Some(-1);
            }
            None
        });
        
        if let Some(code) = res {
            return code;
        }
        yield_now();
    }
}

pub fn kill_task(pid: usize) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        
        let current_room_id = if let Some(t) = &tasks[current_pid] {
            t.room.id
        } else {
            0
        };

        if pid == 0 { return -1; } 
        
        if let Some(target) = &mut tasks[pid] {
            let target_room_id = target.room.id;
            
            // SECURITY: Only allow kill if same room or caller is ROOT (0)
            if current_room_id != 0 && current_room_id != target_room_id {
                return -1; // Permission Denied
            }

            if target.state != TaskState::Free && target.state != TaskState::Zombie {
                 target.state = TaskState::Zombie;
                 target.exit_code = -9; 
                 return 0;
            }
        }
        -1
    })
}

pub fn print_task_list() {
    // Critical Section: Disable Interrupts to prevent deadlock with Tick
    let flags = crate::cpu::control::save_cpu_flags();
    unsafe { core::arch::asm!("cli", options(nomem, nostack)); }

    {
        let tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        let current_room_id = if let Some(t) = &tasks[current_pid] {
            t.room.id
        } else {
            0
        };

        crate::drivers::video::put_str("PID  State    CPU Ticks\n");
        for i in 0..MAX_TASKS {
            if let Some(task) = &tasks[i] {
                let target_room_id = task.room.id;

                // SOVEREIGN FILTER: Non-root rooms only see their own peers
                if current_room_id != 0 && target_room_id != current_room_id {
                    continue;
                }

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
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        
        if let Some(task) = &mut tasks[current_pid] {
            task.state = TaskState::Waiting;
            clear_ready(current_pid);
        }
        drop(tasks);
        schedule();
    });
}

pub fn wake_task(pid: usize) {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[pid] {
             if task.state == TaskState::Waiting {
                 task.state = TaskState::Ready;
                 set_ready(pid);
             }
        }
    });
}

pub fn increment_current_page_count() {
    if !SCHEDULER_INITIALIZED.load(core::sync::atomic::Ordering::SeqCst) {
        return;
    }
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        if current_pid < MAX_TASKS {
            if let Some(task) = &mut tasks[current_pid] {
                task.page_count += 1;
            }
        }
    });
}

pub fn suspend_thread(pid: usize) {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[pid] {
            if task.state == TaskState::Ready || task.state == TaskState::Running {
                task.state = TaskState::Waiting;
                clear_ready(pid);
            }
        }
    });
}

pub fn resume_thread(pid: usize) {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[pid] {
            if task.state == TaskState::Waiting {
                 task.state = TaskState::Ready;
                 set_ready(pid);
            }
        }
    });
}

pub fn bind_thread_cpu(pid: usize, _cpu: usize) {
    crate::cpu::without_interrupts(|| {
        let tasks = TASKS.lock();
        if tasks[pid].is_some() {
            // affinity logic
        }
    });
}

pub fn reap_any_zombie() {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        for i in 1..MAX_TASKS { 
            if let Some(task) = &tasks[i] {
                if task.state == TaskState::Zombie {
                    tasks[i] = None;
                }
            }
        }
    });
}

pub fn detect_starvation() {
    crate::cpu::without_interrupts(|| {
        let tasks = TASKS.lock();
        for i in 0..MAX_TASKS {
            if let Some(task) = &tasks[i] {
                if task.state == TaskState::Ready {
                    // starvation logic
                }
            }
        }
    });
}

pub fn detect_priority_inversion() {
     // Stub
}
