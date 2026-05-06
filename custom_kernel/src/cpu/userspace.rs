use core::arch::asm;
use crate::cpu::gdt::{USER_CODE, USER_DATA};

pub unsafe fn enter_userspace(entry_point: u64, stack_ptr: u64) {
    // DIAGNOSTICS: Check mappings before transition
    let cr3 = crate::mm::vmm::read_cr3();
    let cpu_id = crate::cpu::percpu::get_current_cpu_id();
    
    unsafe {
        crate::cpu::idt::print_serial("\n[Userspace] CPU ");
        crate::cpu::idt::print_hex(cpu_id as u64);
        crate::cpu::idt::print_serial(" entering user mode...\n");
        crate::cpu::idt::print_serial("  RIP: "); crate::cpu::idt::print_hex(entry_point);
        crate::cpu::idt::print_serial("  RSP: "); crate::cpu::idt::print_hex(stack_ptr);
        crate::cpu::idt::print_serial("  CR3: "); crate::cpu::idt::print_hex(cr3);
        crate::cpu::idt::print_serial("\n");

        // Validate Entry Point Mapping
        if let Some(entry_pte) = crate::mm::vmm::get_mapping_info(cr3, entry_point) {
            crate::cpu::idt::print_serial("  Entry PTE: "); crate::cpu::idt::print_hex(entry_pte);
            if entry_pte & 0x4 == 0 { crate::cpu::idt::print_serial(" [ERR: NOT USER]"); }
            if entry_pte & 0x1 == 0 { crate::cpu::idt::print_serial(" [ERR: NOT PRESENT]"); }
        } else {
            crate::cpu::idt::print_serial("  Entry PTE: [ERR: NOT MAPPED]");
        }
        crate::cpu::idt::print_serial("\n");

        // Validate Stack Mapping (check top of stack)
        if let Some(stack_pte) = crate::mm::vmm::get_mapping_info(cr3, stack_ptr - 8) {
            crate::cpu::idt::print_serial("  Stack PTE: "); crate::cpu::idt::print_hex(stack_pte);
            if stack_pte & 0x4 == 0 { crate::cpu::idt::print_serial(" [ERR: NOT USER]"); }
        } else {
            crate::cpu::idt::print_serial("  Stack PTE: [ERR: NOT MAPPED]");
        }
        crate::cpu::idt::print_serial("\n");
        
        // Final IRETQ Frame Check
        crate::cpu::idt::print_serial("  CS: "); crate::cpu::idt::print_hex(USER_CODE as u64);
        crate::cpu::idt::print_serial(" SS: "); crate::cpu::idt::print_hex(USER_DATA as u64);
        crate::cpu::idt::print_serial("\n");
    }

    // We must fake an interrupt stack frame to return "back" to Ring 3.
    // Frame: [SS, RSP, RFLAGS, CS, RIP]
    asm!(
        "cli",
        "mov ds, {ss:e}",
        "mov es, {ss:e}",
        "swapgs",
        "mov fs, {zero:e}",
        "mov gs, {zero:e}",
        "push {ss}",
        "push {rsp}",
        "push {rflags}",
        "push {cs}",
        "push {rip}",
        "iretq",
        ss = in(reg) USER_DATA as u64,
        rsp = in(reg) stack_ptr,
        rflags = const 0x3202,
        cs = in(reg) USER_CODE as u64,
        rip = in(reg) entry_point,
        zero = in(reg) 0u64,
        options(noreturn)
    );
}
