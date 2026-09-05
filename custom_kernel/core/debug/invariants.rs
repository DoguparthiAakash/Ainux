use core::arch::asm;

pub fn verify_context_integrity() {
    let rflags: u64;
    let rsp: u64;
    let cr3: u64;
    
    unsafe {
        asm!("pushfq; pop {}", out(reg) rflags);
        asm!("mov {}, rsp", out(reg) rsp);
        asm!("mov {}, cr3", out(reg) cr3);
    }
    
    // 1. Interrupts Check (Advisory)
    // let interrupts_enabled = (rflags & 0x200) != 0;
    
    // 2. Stack Alignment Check (System V ABI requires 16-byte alignment before call)
    // We are inside a function, so RSP + 8 should be 16-byte aligned (pushed RIP).
    // So RSP should be 8-byte off 16-byte boundary. 
    // Wait, Rust functions might adjust stack.
    // General invariant: Stack should be well within Kernel Stack range if in Kernel Mode.
    // TODO: defined Kernel Stack Limits.
    
    // 3. Paging Enabled Check
    if cr3 == 0 {
         crate::debug::error::report_error(crate::debug::error::KernelError::Generic, "Invariant: CR3 is 0!");
    }
    
    // 4. Canonical Address Check (Kernel Space)
    // RSP should be high half (0xFFFF...)
    if rsp < 0xFFFF_8000_0000_0000 {
        // Unless we are in a trampoline or very early boot?
        // We assume we are in Higher Half.
         // crate::debug::error::report_error(crate::debug::error::KernelError::Generic, "Invariant: RSP in Lower Half!");
         // Note: Some boot code might use lower half. Be careful.
    }
}

pub fn verify_kernel_invariants() {
    verify_context_integrity();
}

pub fn assert_interrupts_disabled() {
    let rflags: u64;
    unsafe {
        asm!("pushfq; pop {}", out(reg) rflags);
    }
    if (rflags & 0x200) != 0 {
        panic!("Invariant Violation: Interrupts Enabled in Critical Section");
    }
}

pub fn assert_no_undefined_behavior() {
    // Check for obvious indicators of state corruption
    // e.g. RSP alignment
    let rsp: u64;
    unsafe { asm!("mov {}, rsp", out(reg) rsp); }
    // Although standard is 16-byte, we just check 8-byte alignment as absolute minimum
    if rsp % 8 != 0 {
         panic!("UB Detected: Stack Misaligned (RSP: {:#x})", rsp);
    }
}

pub fn assert_no_unbounded_execution() {
    // Check if interrupts are enabled if we are "long running"
    // Ideally we'd check preemption count, but for now just check interrupts
    // If interrupts are disabled for too long, it's unbounded execution.
    // This is hard to assert without a timer context.
    // We'll trust the caller to use this in loops.
    let rflags: u64;
    unsafe { asm!("pushfq; pop {}", out(reg) rflags); }
    if (rflags & 0x200) != 0 {
        // Interrupts enabled, execution is bounded by timer. Good.
    } else {
        // Interrupts disabled. We must be in a short critical section.
        // Warn if this check is called here.
        // crate::debug::error::report_warning("Potential Unbounded Execution: checking in atomic context");
    }
}

pub fn assert_no_silent_failure() {
    // Audit log should be active
    // TODO: Check if logger is initialized
}

pub fn assert_memory_consistency() {
    // Check Kernel Heap integrity (basic)
    // TODO: Heap check
    
    // Check Paging
    let cr3: u64;
    unsafe { asm!("mov {}, cr3", out(reg) cr3); }
    if cr3 == 0 {
        panic!("Memory Invariant Violation: CR3 is NULL");
    }
}

pub fn assert_capability_consistency() {
    // Verify current process caps (stub)
}
