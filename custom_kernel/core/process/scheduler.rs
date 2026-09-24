use core::arch::{asm, naked_asm};
use alloc::sync::Arc;
use spin::Mutex;
use crate::process::task::{Task, TaskState, Context};
use crate::process::switch::__switch;

pub const MAX_TASKS: usize = 64;

pub static TASKS: Mutex<[Option<Task>; MAX_TASKS]> = Mutex::new([const { None }; MAX_TASKS]);

pub static FOREGROUND_PID: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

static mut TICKS: u64 = 0;
pub static READY_BITMAP: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
pub static SCHEDULER_INITIALIZED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

pub fn set_ready(pid: usize) {
    if pid < MAX_TASKS {
        READY_BITMAP.fetch_or(1u64 << pid, core::sync::atomic::Ordering::Relaxed);
    }
}

fn clear_ready(pid: usize) {
    if pid < MAX_TASKS {
        READY_BITMAP.fetch_and(!(1u64 << pid), core::sync::atomic::Ordering::Relaxed);
    }
}

pub fn get_ticks() -> u64 {
    unsafe { TICKS }
}

pub fn set_foreground_pid(pid: usize) {
    FOREGROUND_PID.store(pid, core::sync::atomic::Ordering::SeqCst);
}

pub fn get_foreground_pid() -> usize {
    FOREGROUND_PID.load(core::sync::atomic::Ordering::SeqCst)
}

pub fn init() {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        // Initialize Task 0 as the current running kernel task
        let root_cell = crate::process::cell::CELL_MANAGER.lock().cells.first().cloned().expect("No root cell");
        let mut task0 = Task::new_free(root_cell.clone());
        task0.id = 0;
        task0.state = TaskState::Running;
        task0.name = alloc::string::String::from("kernel");
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
    let current_ticks = get_ticks();
    let mut tasks = TASKS.lock();
    
    // Wake up sleeping tasks
    for i in 1..MAX_TASKS {
        if let Some(task) = &mut tasks[i] {
            if task.state == TaskState::Waiting && task.sleep_ticks > 0 && current_ticks >= task.sleep_ticks {
                task.sleep_ticks = 0;
                task.state = TaskState::Ready;
                set_ready(i);
            }
        }
    }

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

#[no_mangle]
pub extern "C" fn handle_ctrl_c() {
    let mut pid = get_foreground_pid();
    if pid == 0 {
        pid = crate::cpu::smp::get_current_pid();
    }
    
    if pid != 0 {
        crate::cpu::without_interrupts(|| {
            let mut tasks = TASKS.lock();
            if let Some(task) = &mut tasks[pid] {
                if task.name != "kernel" {
                    task.signals |= 1 << crate::process::task::SIGINT;
                    task.state = crate::process::task::TaskState::Zombie;
                    task.exit_code = 130; // Standard SIGINT exit code
                }
            }
        });
        
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        use core::fmt::Write;
        let _ = write!(serial, "\n^C\n");
    }
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

pub fn get_current_cell_context() -> Arc<crate::process::cell::ExecutionCell> {
    get_current_cell()
}

pub fn get_current_cell() -> Arc<crate::process::cell::ExecutionCell> {
    let tasks = TASKS.lock();
    let current_pid = crate::cpu::smp::get_current_pid();
    tasks[current_pid].as_ref().expect("No current task").cell.clone()
}

pub fn spawn_kernel_task(func: u64, name: &str) -> usize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        for i in 0..MAX_TASKS {
            if tasks[i].is_none() || tasks[i].as_ref().unwrap().state == TaskState::Free {
                // Root tasks go to Root Cell
                let cell = crate::process::cell::CELL_MANAGER.lock().cells.first().cloned().expect("No root cell");

                let mut task = Task::new_free(cell);
                task.id = i;
                task.state = TaskState::Ready;
                task.name = alloc::string::String::from(name);

                tasks[i] = Some(task);
                let task_ref = tasks[i].as_mut().unwrap();

                // Setup Stack
                let stack_top = task_ref.stack.as_ptr() as u64 + task_ref.stack.len() as u64;
                let mut sp = stack_top & !0xF;
                unsafe {
                    sp -= 8; *(sp as *mut u64) = kernel_thread_entry as u64; // ret
                }
                task_ref.context.rsp = sp;
                task_ref.context.r12 = func;
                
                set_ready(i);
                return i;
            }
        }
        0
    })
}

pub fn spawn(func: extern "C" fn(), name: &str) -> usize {
    spawn_kernel_task(func as u64, name)
}

#[unsafe(naked)]
pub extern "C" fn kernel_thread_entry() {
    unsafe {
        core::arch::naked_asm!(
            "call {}",
            "sti",
            "mov rax, r12",
            "call rax",
            "mov rdi, 0",
            "call {}",
            "ud2",
            sym unlock_tasks_from_asm,
            sym exit_current_task
        );
    }
}

#[no_mangle]
pub extern "C" fn unlock_tasks_from_asm() {
    unsafe { TASKS.force_unlock(); }
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
                // Inherit Cell
                let cell = if let Some(current) = &tasks[current_pid] {
                    current.cell.clone()
                } else {
                    crate::process::cell::CELL_MANAGER.lock().cells.first().cloned().expect("No root cell")
                };

                let mut task = Task::new_free(cell);
                task.id = i;
                task.state = TaskState::Ready;
                task.name = alloc::format!("{}-child", tasks[current_pid].as_ref().unwrap().name);
                
                // Implement COW Fork for the address space (stub for Phase 2)
                let new_address_space = alloc::sync::Arc::new(spin::Mutex::new(crate::mm::address_space::AddressSpace::new_user()));
                task.cr3 = new_address_space.lock().pml4_phys;
                if task.cr3 == 0 {
                    return -1; // OOM allocating page tables
                }
                task.address_space = Some(new_address_space);
                task.caps = caps; // Inherit Caps
                task.fds = fds; // Clone FDs 
                task.namespace = tasks[current_pid].as_ref().unwrap().namespace.clone();
                
                task.userspace_stack_top = stack_ptr;
                
                // Setup Kernel Stack for Return to User
                let kstack_top = task.stack.as_ptr() as u64 + 16384;
                let mut sp = kstack_top & !0xF;
                unsafe {
                    sp -= 8;
                    *(sp as *mut u64) = kernel_shim_entry as u64; // ret
                }
                task.context.rsp = sp;
                task.context.r12 = entry; // Set r12 so kernel_shim_entry can retrieve it
                task.context.r13 = stack_ptr; // Set r13 so kernel_shim_entry can retrieve it
                
                tasks[i] = Some(task);
                set_ready(i);
                return i as isize;
            }
        }
        -1
    })
}

pub fn spawn_user(rip: u64, rsp: u64, address_space: alloc::sync::Arc<spin::Mutex<crate::mm::address_space::AddressSpace>>, name: &str) -> usize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        for i in 0..MAX_TASKS {
            if tasks[i].is_none() || tasks[i].as_ref().unwrap().state == TaskState::Free {
                // Assign Root Cell by default for user spawn (can be moved later)
                let root_cell = crate::process::cell::CELL_MANAGER.lock().cells.first().cloned().expect("No root cell");
                let mut task = Task::new_free(root_cell);
                task.id = i;
                task.state = TaskState::Ready;
                task.name = alloc::string::String::from(name);
                task.cr3 = address_space.lock().pml4_phys;
                task.address_space = Some(address_space);
                task.userspace_stack_top = rsp;
                task.parent_id = Some(crate::process::scheduler::get_current_pid());
                
                tasks[i] = Some(task);
                let task_ref = tasks[i].as_mut().unwrap();

                let kstack_top = task_ref.stack.as_ptr() as u64 + 16384;
                let mut sp = kstack_top & !0xF;
                unsafe {
                    sp -= 8;
                    *(sp as *mut u64) = kernel_shim_entry as u64; // ret
                }
                task_ref.context.rsp = sp;
                task_ref.context.r12 = rip;
                task_ref.context.r13 = rsp;
                
                let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
                use core::fmt::Write;
                let _ = write!(serial, "DEBUG spawn_user [PID {}]: task.stack.as_ptr()={:#x}, kstack_top={:#x}, sp={:#x}\n", i, task_ref.stack.as_ptr() as u64, kstack_top, sp);
                
                unsafe {
                    let gdt_ptr = crate::cpu::gdt::GDT.as_ptr() as *const u64;
                    let desc = *gdt_ptr.add(3);
                    let _ = write!(serial, "GDT[3] = {:#018x}\n", desc);
                    let desc4 = *gdt_ptr.add(4);
                    let _ = write!(serial, "GDT[4] = {:#018x}\n", desc4);
                }

                set_ready(i);
                return i;
            }
        }
        0 // Error PID
    })
}

#[unsafe(naked)]
pub extern "C" fn kernel_shim_entry() {
    unsafe {
        core::arch::naked_asm!(
            "push r12",
            "push r13",
            "call {}",
            "pop r13",
            "pop r12",
            "mov rdi, r12",
            "mov rsi, r13",
            "jmp {}",
            sym unlock_tasks_from_asm,
            sym crate::cpu::userspace::enter_userspace
        );
    }
}

pub fn schedule() {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();

        let next_pid = if let Some(next) = pick_next_task_internal(&mut tasks, current_pid) {
            next
        } else {
             if let Some(task) = &tasks[current_pid] {
                 if task.state == TaskState::Running {
                     return; // The current task is the only one runnable. Keep running it.
                 }
             }
             // We MUST switch to the idle task (Task 0) since the current task is blocked/dead.
             0
        };
        
        crate::cpu::smp::set_current_pid(next_pid);
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
    } else {
        unsafe {
             let kernel_cr3 = crate::mm::vmm::KERNEL_PML4.load(core::sync::atomic::Ordering::Relaxed) as u64;
             core::arch::asm!("mov cr3, {}", in(reg) kernel_cr3);
        }
    }
    
    // Update TSS RSP0
    let kstack_top = next_task.stack.as_ptr() as u64 + next_task.stack.len() as u64;
    unsafe {
        crate::cpu::gdt::set_kernel_stack(kstack_top);
        crate::cpu::syscall::set_syscall_kernel_stack(kstack_top);
    }
    
    // Maturation: Final pointers for switch
    let old_task_ptr = &mut old_task.context as *mut Context;
    let next_task_ptr = &next_task.context as *const Context;
    
    // Leak the MutexGuard so the lock stays held during the context switch.
    // The lock will be released by the incoming task once it resumes.
    core::mem::forget(tasks); 
    
    unsafe {
        __switch(old_task_ptr, next_task_ptr);
        TASKS.force_unlock();
    }
    });
}

fn pick_next_task_internal(tasks: &mut [Option<Task>; MAX_TASKS], current: usize) -> Option<usize> {
    // 1. Get current Cell Strategy
    if let Some(task) = &tasks[current] {
        let cell = &task.cell;
        let strategy = cell.strategy.lock().clone();
        if let Some(next) = strategy.pick_next(tasks, current) {
            if let Some(task) = &tasks[next] {
                if task.state == TaskState::Ready {
                    return Some(next);
                }
            }
        }
    }

    // 2. Fallback to global bitmask (Very fast O(1))
    let mut mask = READY_BITMAP.load(core::sync::atomic::Ordering::Relaxed);
    while mask != 0 {
        let next_pid = mask.trailing_zeros() as usize;
        if next_pid < MAX_TASKS {
            if let Some(task) = &tasks[next_pid] {
                if task.state == TaskState::Ready {
                    return Some(next_pid);
                } else {
                    clear_ready(next_pid); // Stale bit
                }
            } else {
                clear_ready(next_pid); // Stale bit
            }
        }
        mask &= !(1u64 << next_pid); // clear the bit we just checked
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
        let current_pid = crate::cpu::smp::get_current_pid();
        if let Some(task) = &mut tasks[current_pid] {
            task.sleep_ticks = target_ticks;
            if target_ticks > 0 {
                task.state = TaskState::Waiting;
            }
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
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "process_open: path='{}', flags={:#x}\n", path, flags);

    match crate::fs::vfs::resolve_path(path) {
        Ok(inode) => {
            match inode.open(flags) {
                Ok(handle) => {
                    let fd = crate::cpu::without_interrupts(|| {
                        let mut tasks = TASKS.lock();
                        let current_pid = crate::cpu::smp::get_current_pid();
                        if let Some(task) = &mut tasks[current_pid] {
                            task.fds.alloc_fd(handle)
                        } else {
                            None
                        }
                    });
                    if let Some(fd) = fd {
                        let _ = write!(serial, "process_open: '{}' -> fd={}\n", path, fd);
                        fd as isize
                    } else {
                        let _ = write!(serial, "process_open: '{}' -> EMFILE (fd table full)\n", path);
                        -24 // EMFILE
                    }
                }
                Err(e) => {
                    let _ = write!(serial, "process_open: inode.open('{}') failed: {:?}\n", path, e);
                    -13 // EACCES
                }
            }
        }
        Err(e) => {
            let _ = write!(serial, "process_open: resolve_path('{}') failed: {:?}\n", path, e);
            -2 // ENOENT
        }
    }
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

pub fn process_mmap(addr: u64, length: u64, prot: u32, flags: u32, _fd: i32, _offset: u64) -> isize {
    // Only support anonymous mapping for now (MAP_ANONYMOUS = 0x20)
    let map_anonymous = 0x20;
    if (flags & map_anonymous) == 0 {
        return usize::MAX as isize; // -ENODEV or -EINVAL
    }
    
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        if let Some(task) = &mut tasks[current_pid] {
            let num_pages = (length + 0xFFF) / 0x1000;
            let target_addr = if addr == 0 {
                let start = task.mmap_base;
                task.mmap_base += num_pages * 0x1000;
                start
            } else {
                addr
            };
            
            // Map the pages
            for i in 0..num_pages {
                let virt = target_addr + i * 0x1000;
                // vmm_flags: USER = 4, WRITE = 2, PRESENT = 1 -> 0x07
                let vmm_flags = 0x07; 
                let _ = unsafe { crate::mm::vmm::map_page_allocate_in_pml4(task.cr3, virt, vmm_flags) };
            }
            
            // Zero the memory since it's an anonymous mapping
            unsafe {
                core::ptr::write_bytes(target_addr as *mut u8, 0, (num_pages * 0x1000) as usize);
            }
            
            target_addr as isize
        } else {
            usize::MAX as isize
        }
    })
}

pub fn exit_current_task(exit_code: isize) {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        
        let mut parent_to_wake = None;
        if let Some(task) = &mut tasks[current_pid] {
             task.state = TaskState::Zombie;
             task.exit_code = exit_code;
             parent_to_wake = task.parent_id;
             clear_ready(current_pid);
        }
        
        if let Some(pid) = parent_to_wake {
             if let Some(parent) = &mut tasks[pid] {
                 if parent.state == TaskState::Waiting {
                     parent.state = TaskState::Ready;
                     set_ready(pid);
                 }
             }
        }
        
        drop(tasks);
        schedule();
    });
}

pub fn wait_pid(pid: usize) -> isize {
    sys_wait4(pid as isize, 0, 0, 0)
}

pub fn sys_wait4(target_pid: isize, status_ptr: u64, options: i32, _rusage_ptr: u64) -> isize {
    let current_pid = crate::cpu::smp::get_current_pid();
    loop {
        let mut child_found = false;
        let mut reaped_pid = -1;
        let mut exit_code = 0;
        
        let res = crate::cpu::without_interrupts(|| {
            let mut tasks = TASKS.lock();
            for i in 0..MAX_TASKS {
                if i != current_pid {
                    if let Some(task) = &tasks[i] {
                        if task.parent_id == Some(current_pid) {
                            if target_pid <= 0 || target_pid as usize == i {
                                child_found = true;
                                if task.state == TaskState::Zombie {
                                    reaped_pid = i as isize;
                                    exit_code = task.exit_code;
                                    tasks[i] = None;
                                    return Some((reaped_pid, exit_code));
                                }
                            }
                        }
                    }
                }
            }
            
            if child_found && (options & 1) != 0 {
                return Some((0, 0)); // WNOHANG
            }
            
            if child_found {
                if let Some(parent) = &mut tasks[current_pid] {
                    parent.state = TaskState::Waiting;
                }
                clear_ready(current_pid);
                None // Block
            } else {
                Some((-1, 0)) // ECHILD
            }
        });
        
        if let Some((pid, code)) = res {
            if pid > 0 && status_ptr != 0 && crate::mm::user::validate_user_range(status_ptr, 4) {
                let status_val = ((code & 0xFF) << 8) as u32;
                unsafe { *(status_ptr as *mut u32) = status_val; }
            }
            return pid;
        }
        
        // Blocked, yield
        yield_now();
    }
}

pub fn kill_task(pid: usize) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        
        let current_cell_id = if let Some(t) = &tasks[current_pid] {
            t.cell.id
        } else {
            0
        };

        if pid == 0 { return -1; } 
        
        let mut parent_to_wake = None;
        if let Some(target) = &mut tasks[pid] {
            let target_cell_id = target.cell.id;
            
            // SECURITY: Only allow kill if same cell or caller is ROOT (0)
            if current_cell_id != 0 && current_cell_id != target_cell_id {
                return -1; // Permission Denied
            }

            if target.state != TaskState::Free && target.state != TaskState::Zombie {
                 target.state = TaskState::Zombie;
                 target.exit_code = -9; 
                 parent_to_wake = target.parent_id;
                 clear_ready(pid);
            } else {
                 return -1;
            }
        } else {
            return -1;
        }

        if let Some(parent_pid) = parent_to_wake {
             if let Some(parent) = &mut tasks[parent_pid] {
                 if parent.state == TaskState::Waiting {
                     parent.state = TaskState::Ready;
                     set_ready(parent_pid);
                 }
             }
        }
        
        0
    })
}

pub fn print_task_list() {
    // Critical Section: Disable Interrupts to prevent deadlock with Tick
    let flags = crate::cpu::control::save_cpu_flags();
    unsafe { core::arch::asm!("cli", options(nomem, nostack)); }

    {
        let tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        let current_cell_id = if let Some(t) = &tasks[current_pid] {
            t.cell.id
        } else {
            0
        };

        crate::drivers::video::put_str("PID  State    CPU Ticks\n");
        for i in 0..MAX_TASKS {
            if let Some(task) = &tasks[i] {
                let target_cell_id = task.cell.id;

                // SOVEREIGN FILTER: Non-root cells only see their own peers
                if current_cell_id != 0 && target_cell_id != current_cell_id {
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

#[repr(C)]
pub struct SyscallState {
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub r11: u64,
    pub rcx: u64,
    pub user_rsp: u64,
}

pub fn sys_fork(state: *const SyscallState) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let current_pid = crate::cpu::smp::get_current_pid();
        
        let (cr3, caps, fds, cell, pgid, sid, cwd, environ, ruid, euid, rgid, egid, umask) = if let Some(current) = &tasks[current_pid] {
            (
                current.cr3, 
                current.caps.clone(), 
                current.fds.clone(), 
                current.cell.clone(),
                current.pgid,
                current.sid,
                current.cwd.clone(),
                current.environ.clone(),
                current.ruid,
                current.euid,
                current.rgid,
                current.egid,
                current.umask
            )
        } else {
            return -1;
        };

        for i in 0..MAX_TASKS {
            if tasks[i].is_none() || tasks[i].as_ref().unwrap().state == TaskState::Free {
                // Duplicate address space
                let new_address_space = if let Some(parent_as) = tasks[current_pid].as_ref().unwrap().address_space.as_ref() {
                    alloc::sync::Arc::new(spin::Mutex::new(parent_as.lock().clone()))
                } else {
                    return -1; // OOM or Error
                };
                let new_cr3 = new_address_space.lock().pml4_phys;
                if new_cr3 == 0 {
                    return -1; // OOM
                }
                
                let mut task = Task::new_free(cell);
                task.id = i;
                task.state = TaskState::Ready;
                task.name = alloc::format!("{}-child", tasks[current_pid].as_ref().unwrap().name);
                task.cr3 = new_cr3;
                task.address_space = Some(new_address_space);
                task.caps = caps;
                task.fds = fds;
                task.parent_id = Some(current_pid);
                task.pgid = pgid;
                task.sid = sid;
                task.cwd = cwd;
                task.environ = environ;
                task.ruid = ruid;
                task.euid = euid;
                task.rgid = rgid;
                task.egid = egid;
                task.umask = umask;
                
                // Copy SyscallState to child's stack
                let kstack_top = task.stack.as_ptr() as u64 + 16384;
                let mut sp = kstack_top & !0xF;
                unsafe {
                    sp -= core::mem::size_of::<SyscallState>() as u64;
                    core::ptr::copy_nonoverlapping(state, sp as *mut SyscallState, 1);
                    
                    // Push return address for __switch (jump to syscall_sysret_shim)
                    sp -= 8;
                    *(sp as *mut u64) = crate::cpu::syscall::syscall_sysret_shim as u64;
                }
                
                task.context.rsp = sp;
                
                tasks[i] = Some(task);
                set_ready(i);
                
                return i as isize; // Return child PID to parent
            }
        }
        -1
    })
}

pub fn sys_execve(path_ptr: u64, _argv_ptr: u64, _envp_ptr: u64, state: *mut SyscallState) -> isize {
    let mut len = 0;
    while len < 256 {
        let mut b = [0u8; 1];
        if crate::mm::user::copy_from_user((path_ptr + len) as *const u8, &mut b).is_err() {
            return -14; // EFAULT
        }
        if b[0] == 0 { break; }
        len += 1;
    }
    let mut path_buf = alloc::vec![0u8; len as usize];
    crate::mm::user::copy_from_user(path_ptr as *const u8, &mut path_buf).unwrap();
    let path = core::str::from_utf8(&path_buf).unwrap_or("");
    
    // Parse argv array if present
    let mut args = alloc::vec![alloc::string::String::from(path)];
    let argv_ptr = _argv_ptr;
    if argv_ptr != 0 {
        let mut ptr_offset = 0;
        loop {
            let mut ptr_buf = [0u8; 8];
            if crate::mm::user::copy_from_user((argv_ptr + ptr_offset) as *const u8, &mut ptr_buf).is_err() {
                break;
            }
            let str_ptr = u64::from_le_bytes(ptr_buf);
            if str_ptr == 0 { break; }
            
            let mut str_len = 0;
            while str_len < 4096 { 
                let mut b = [0u8; 1];
                if crate::mm::user::copy_from_user((str_ptr + str_len) as *const u8, &mut b).is_err() {
                    break;
                }
                if b[0] == 0 { break; }
                str_len += 1;
            }
            let mut str_buf = alloc::vec![0u8; str_len as usize];
            if crate::mm::user::copy_from_user(str_ptr as *const u8, &mut str_buf).is_ok() {
                if let Ok(s) = core::str::from_utf8(&str_buf) {
                    // Start adding from arg 1 if argv[0] was provided, to prevent duplicating argv[0]
                    // But actually typical C expects argv[0] to be the command itself.
                    // If the user provided argv, we should probably just use their argv directly,
                    // but for safety let's just push it. Wait! If the user provides argv, we clear our default path arg.
                    if ptr_offset == 0 {
                        args.clear();
                    }
                    args.push(alloc::string::String::from(s));
                }
            }
            ptr_offset += 8;
        }
    }
    
    if let Ok(inode) = crate::fs::vfs::resolve_path(path) {
        let address_space = alloc::sync::Arc::new(spin::Mutex::new(crate::mm::address_space::AddressSpace::new_user()));
        let new_cr3 = address_space.lock().pml4_phys;
        if new_cr3 == 0 { return -12; } // ENOMEM
        
        if let Ok(entry) = crate::process::loader::load_elf(inode, new_cr3) {
            let stack_base = 0x00007FFFFFFFE000u64;
            let stack_pages = 256u64;
            for p in 0..stack_pages {
                let frame = crate::mm::pmm::PMM.lock().as_mut().unwrap().alloc_frame().unwrap();
                unsafe { crate::mm::vmm::map_page_in_pml4(new_cr3, stack_base - (p * 4096), frame, 0x07); }
            }
            let stack_top = stack_base + 4096;
            let initial_sp = crate::process::loader::setup_user_stack(new_cr3, stack_top, entry, &args);
            
            crate::cpu::without_interrupts(|| {
                let mut tasks = TASKS.lock();
                let current_pid = crate::cpu::smp::get_current_pid();
                if let Some(task) = &mut tasks[current_pid] {
                    // Only memory leaks the old cr3 for now, no free logic yet
                    task.cr3 = new_cr3;
                    task.address_space = Some(address_space);
                    task.name = alloc::string::String::from(path);
                    unsafe { core::arch::asm!("mov cr3, {}", in(reg) new_cr3); }
                }
            });
            
            unsafe {
                (*state).rcx = entry;
                (*state).user_rsp = initial_sp;
                (*state).a1 = args.len() as u64; // argc (mapped to rdi)
                (*state).a2 = initial_sp + 8;    // argv (mapped to rsi)
            }
            return 0; // returns 0 to the new process through rax
        }
    }
    -2 // ENOENT
}
pub fn sys_exit(code: isize) {
    let pid = get_current_pid();
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[pid] {
            task.state = TaskState::Zombie;
            task.exit_code = code;
            
            // Wake up parent if waiting
            if let Some(parent_id) = task.parent_id {
                if let Some(parent) = &mut tasks[parent_id] {
                    if parent.state == TaskState::Waiting {
                        parent.state = TaskState::Ready;
                        set_ready(parent_id);
                    }
                    parent.pending_signals |= 1 << 17; // SIGCHLD (17 on Linux)
                }
            }
        }
        clear_ready(pid);
    });
    // Yield the CPU
    unsafe { core::arch::asm!("int 0x20"); } // Or schedule()
    loop {}
}

pub fn process_dup(oldfd: usize) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[get_current_pid()] {
            if let Some(newfd) = task.fds.dup(oldfd) {
                return newfd as isize;
            }
        }
        -1
    })
}

pub fn process_dup2(oldfd: usize, newfd: usize) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[get_current_pid()] {
            if let Some(res_fd) = task.fds.dup2(oldfd, newfd) {
                return res_fd as isize;
            }
        }
        -1
    })
}

pub fn process_getcwd(buf: &mut [u8]) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[get_current_pid()] {
            let cwd_bytes = task.cwd.as_bytes();
            if buf.len() < cwd_bytes.len() + 1 {
                return -1; // ERANGE
            }
            buf[..cwd_bytes.len()].copy_from_slice(cwd_bytes);
            buf[cwd_bytes.len()] = 0; // null-terminate
            return cwd_bytes.len() as isize + 1;
        }
        -1
    })
}

pub fn process_chdir(path: &str) -> isize {
    if let Ok(inode) = crate::fs::vfs::resolve_path(path) {
        if let Ok(stat) = inode.stat() {
            if stat.file_type == crate::fs::vfs::FileType::Directory {
                crate::cpu::without_interrupts(|| {
                    let mut tasks = TASKS.lock();
                    if let Some(task) = &mut tasks[get_current_pid()] {
                        task.cwd = alloc::string::String::from(path);
                    }
                });
                return 0;
            }
        }
    }
    -1 // ENOENT or ENOTDIR
}

pub fn get_parent_pid(pid: usize) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[pid] {
            if let Some(ppid) = task.parent_id {
                return ppid as isize;
            }
        }
        -1
    })
}

pub fn process_stat(path: &str, stat_out: &mut crate::fs::vfs::CStat) -> isize {
    if let Ok(inode) = crate::fs::vfs::resolve_path(path) {
        if let Ok(f_stat) = inode.stat() {
            stat_out.st_ino = inode.inode_num() as u64;
            stat_out.st_mode = f_stat.mode as u32;
            if f_stat.file_type == crate::fs::vfs::FileType::Directory {
                stat_out.st_mode |= 0o040000; // S_IFDIR
            } else if f_stat.file_type == crate::fs::vfs::FileType::File {
                stat_out.st_mode |= 0o100000; // S_IFREG
            }
            stat_out.st_uid = f_stat.uid as u32;
            stat_out.st_gid = f_stat.gid as u32;
            stat_out.st_size = f_stat.size as i64;
            stat_out.st_blksize = 4096;
            stat_out.st_blocks = (f_stat.size as i64 + 511) / 512;
            stat_out.st_mtime = f_stat.mtime as i64;
            return 0;
        }
    }
    -1
}

pub fn process_fstat(fd: usize, stat_out: &mut crate::fs::vfs::CStat) -> isize {
    // Currently our FileHandle doesn't directly expose stat, but if it's open, 
    // we would normally query its inode. For simplicity in this basic kernel,
    // we return a dummy successful stat for open FDs. 
    // (Or we can extend FileHandle to return FileStat).
    // Let's implement a dummy stat for now.
    let is_valid = crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = &mut tasks[get_current_pid()] {
            task.fds.get_entry(fd).is_some()
        } else {
            false
        }
    });
    
    if is_valid {
        stat_out.st_mode = 0o100000 | 0o666; // S_IFREG | rw-rw-rw-
        stat_out.st_size = 0;
        stat_out.st_blksize = 4096;
        0
    } else {
        -1 // EBADF
    }
}

pub fn sys_get_tasks(buf_ptr: u64, buf_len: usize) -> isize {
    let mut out_str = alloc::string::String::new();
    out_str.push_str("PID  Name            State    CPU Ticks\n");
    
    let current_pid = get_current_pid();
    let current_cell_id = {
        let tasks = TASKS.lock();
        if let Some(t) = &tasks[current_pid] {
            t.cell.id
        } else {
            0
        }
    };
    
    crate::cpu::without_interrupts(|| {
        let tasks = TASKS.lock();
        for i in 0..MAX_TASKS {
            if let Some(task) = &tasks[i] {
                if current_cell_id != 0 && task.cell.id != current_cell_id {
                    continue;
                }
                
                let state_str = match task.state {
                    TaskState::Running => "Running",
                    TaskState::Ready => "Ready  ",
                    TaskState::Waiting => "Waiting",
                    TaskState::Free => "Free   ",
                    TaskState::Zombie => "Zombie ",
                };
                
                let line = alloc::format!("{:<4} {:<15} {:<8} {}\n", task.id, task.name, state_str, task.cpu_time_ticks);
                out_str.push_str(&line);
            }
        }
    });
    
    let bytes = out_str.as_bytes();
    let copy_len = core::cmp::min(bytes.len(), buf_len);
    
    if crate::mm::user::copy_to_user(buf_ptr as *mut u8, &bytes[..copy_len]).is_err() {
        return -1;
    }
    
    copy_len as isize
}

pub fn sys_getuid() -> isize {
    crate::cpu::without_interrupts(|| {
        let tasks = TASKS.lock();
        let pid = get_current_pid();
        if let Some(task) = &tasks[pid] {
            return task.ruid as isize;
        }
        -1
    })
}

pub fn sys_geteuid() -> isize {
    crate::cpu::without_interrupts(|| {
        let tasks = TASKS.lock();
        let pid = get_current_pid();
        if let Some(task) = &tasks[pid] {
            return task.euid as isize;
        }
        -1
    })
}

pub fn sys_getgid() -> isize {
    crate::cpu::without_interrupts(|| {
        let tasks = TASKS.lock();
        let pid = get_current_pid();
        if let Some(task) = &tasks[pid] {
            return task.rgid as isize;
        }
        -1
    })
}

pub fn sys_getegid() -> isize {
    crate::cpu::without_interrupts(|| {
        let tasks = TASKS.lock();
        let pid = get_current_pid();
        if let Some(task) = &tasks[pid] {
            return task.egid as isize;
        }
        -1
    })
}

pub fn sys_setuid(uid: u32) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let pid = get_current_pid();
        if let Some(task) = &mut tasks[pid] {
            // Unprivileged can only set to ruid or euid
            if task.euid == 0 || task.ruid == uid || task.euid == uid {
                task.ruid = uid;
                task.euid = uid;
                return 0;
            }
            return -1; // EPERM
        }
        -1
    })
}

pub fn sys_setgid(gid: u32) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let pid = get_current_pid();
        if let Some(task) = &mut tasks[pid] {
            if task.euid == 0 || task.rgid == gid || task.egid == gid {
                task.rgid = gid;
                task.egid = gid;
                return 0;
            }
            return -1; // EPERM
        }
        -1
    })
}

pub fn sys_setreuid(ruid: u32, euid: u32) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let pid = get_current_pid();
        if let Some(task) = &mut tasks[pid] {
            // Simplified check: root can do anything. Others can swap or set to their current values.
            let is_root = task.euid == 0;
            let valid_ruid = ruid == 0xFFFFFFFF || is_root || ruid == task.ruid || ruid == task.euid;
            let valid_euid = euid == 0xFFFFFFFF || is_root || euid == task.ruid || euid == task.euid;
            
            if valid_ruid && valid_euid {
                if ruid != 0xFFFFFFFF { task.ruid = ruid; }
                if euid != 0xFFFFFFFF { task.euid = euid; }
                return 0;
            }
            return -1; // EPERM
        }
        -1
    })
}

pub fn sys_setregid(rgid: u32, egid: u32) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        let pid = get_current_pid();
        if let Some(task) = &mut tasks[pid] {
            let is_root = task.euid == 0;
            let valid_rgid = rgid == 0xFFFFFFFF || is_root || rgid == task.rgid || rgid == task.egid;
            let valid_egid = egid == 0xFFFFFFFF || is_root || egid == task.rgid || egid == task.egid;
            
            if valid_rgid && valid_egid {
                if rgid != 0xFFFFFFFF { task.rgid = rgid; }
                if egid != 0xFFFFFFFF { task.egid = egid; }
                return 0;
            }
            return -1; // EPERM
        }
        -1
    })
}
