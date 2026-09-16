use alloc::vec::Vec;
use core::arch::global_asm;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelType {
    /// The primary monolithic OS core (Mithl OS). Runs with highest priority.
    MonolithicCore,
    /// A sibling kernel for BSD subsystems (Networking, Security).
    BSDHybrid,
    /// A lightweight hypervisor-level task.
    MicroTask,
}

/// Architecture-specific CPU context for a suspended kernel.
#[derive(Debug, Default)]
#[repr(C)]
pub struct KernelContext {
    pub rsp: u64, // Stack pointer where registers were saved
    pub cr3: u64, // Page table base
}

/// Represents an entire Kernel Environment running concurrently.
pub struct KernelEnvironment {
    pub id: u64,
    pub ktype: KernelType,
    pub context: KernelContext,
    pub is_running: bool,
}

// Global queue of kernel environments.
lazy_static::lazy_static! {
    static ref KERNEL_ENVIRONMENTS: Mutex<Vec<KernelEnvironment>> = Mutex::new(Vec::new());
}

static mut CURRENT_KERNEL_ID: u64 = 0;

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "Mithl OS: Microkernel Hypervisor & Scheduler Initialized.\n");
    
    // Register the currently executing monolithic kernel (Mithl OS) as ID 0
    let main_kernel = KernelEnvironment {
        id: 0,
        ktype: KernelType::MonolithicCore,
        context: KernelContext::default(),
        is_running: true,
    };
    
    KERNEL_ENVIRONMENTS.lock().push(main_kernel);
}

pub fn register_kernel(ktype: KernelType, stack_ptr: u64, page_table: u64) -> u64 {
    let mut envs = KERNEL_ENVIRONMENTS.lock();
    let id = envs.len() as u64;
    let new_env = KernelEnvironment {
        id,
        ktype,
        context: KernelContext {
            rsp: stack_ptr,
            cr3: page_table,
        },
        is_running: false,
    };
    envs.push(new_env);
    id
}

/// The main hypervisor scheduling loop. Called by a timer interrupt (e.g. APIC).
/// Uses priority for the Monolithic OS and round-robin for others.
#[no_mangle]
pub extern "C" fn microkernel_schedule(old_rsp: u64) -> u64 {
    unsafe {
        // Acknowledge the hardware timer interrupt
        crate::cpu::pic::notify_eoi(0);
    }

    let is_monolithic = {
        let mut envs = KERNEL_ENVIRONMENTS.lock();
        if envs.is_empty() {
            return old_rsp; // No environments, do nothing.
        }

        let current_id = unsafe { CURRENT_KERNEL_ID } as usize;
        envs[current_id].context.rsp = old_rsp;
        envs[current_id].is_running = false;
        envs[current_id].ktype == KernelType::MonolithicCore
    };

    // Tick the monolithic kernel's internal process scheduler if it was running.
    // MUST be called without KERNEL_ENVIRONMENTS lock, because tick() can perform
    // a context switch and suspend this execution path.
    if is_monolithic {
        crate::process::scheduler::tick();
    }

    let mut envs = KERNEL_ENVIRONMENTS.lock();
    let current_id = unsafe { CURRENT_KERNEL_ID } as usize;

    // Priority + Round Robin Scheduling Algorithm:
    // 1. Try to schedule the Monolithic Core (highest priority) 75% of the time.
    // 2. Schedule BSD / MicroTasks in a round-robin fashion the other 25% of the time.
    
    // For this stub, we implement a simple round-robin favoring ID 0 (Monolithic).
    let mut next_id = (current_id + 1) % envs.len();
    
    // Simplistic priority logic: If next_id is not the monolithic core, 
    // we can skip it randomly or via a counter to prioritize the OS.
    if envs[next_id].ktype != KernelType::MonolithicCore {
        // Fallback to round-robin among siblings
    }

    envs[next_id].is_running = true;
    unsafe { CURRENT_KERNEL_ID = next_id as u64 };

    // Return the new stack pointer to the assembly stub
    if next_id == current_id {
        old_rsp
    } else {
        envs[next_id].context.rsp
    }
}

// ----------------------------------------------------------------------------
// Low-Level Hypervisor Context Switch Assembly
// ----------------------------------------------------------------------------

global_asm!(r#"
.global switch_kernel_context
switch_kernel_context:
    // This function is invoked during a hardware timer interrupt in Ring 0.
    // At this point, the CPU has already pushed SS, RSP, RFLAGS, CS, RIP.

    // 1. Save general-purpose registers of the current kernel environment.
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push rbp
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx
    push rax

    // 2. Pass the current stack pointer (RSP) to the Rust scheduler logic.
    mov rdi, rsp
    
    // 3. Call the hypervisor scheduler (microkernel_schedule)
    // It returns the new kernel's stack pointer in rax.
    call microkernel_schedule
    
    // 4. Swap to the new kernel's stack.
    mov rsp, rax

    // 5. Restore general-purpose registers for the new kernel environment.
    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop rbp
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15

    // 6. Return from interrupt (pops RIP, CS, RFLAGS, RSP, SS)
    iretq
"#);

extern "C" {
    pub fn switch_kernel_context();
}
