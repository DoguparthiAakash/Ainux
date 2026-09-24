// Ainux KPI — IRQ handler table for Linux driver compatibility.
//
// Linux drivers call request_irq() to register handlers. This module maintains
// a table mapping IRQ vectors (0-255) to C function pointers, and routes
// incoming interrupts to the registered C handler.

use spin::Mutex;

type KpiHandler = unsafe extern "C" fn(i32, *mut u8) -> i32;

struct KpiIrqEntry {
    handler: Option<KpiHandler>,
    dev_id:  *mut u8,
}

// SAFETY: We are single-threaded bare metal; the pointer is never accessed
// concurrently. Mutex protects against nested IRQ re-entry.
unsafe impl Send for KpiIrqEntry {}
unsafe impl Sync for KpiIrqEntry {}

impl KpiIrqEntry {
    const fn empty() -> Self {
        KpiIrqEntry { handler: None, dev_id: core::ptr::null_mut() }
    }
}

const MAX_IRQ: usize = 256;
static IRQ_TABLE: Mutex<[KpiIrqEntry; MAX_IRQ]> = Mutex::new({
    // const-init array of empty entries
    // Rust doesn't support [expr; N] for non-Copy types in const context,
    // so we initialise manually with a macro-expanded block.
    // Using a zeroed approach:
    let mut arr: [KpiIrqEntry; MAX_IRQ] = unsafe {
        core::mem::MaybeUninit::zeroed().assume_init()
    };
    // MaybeUninit::zeroed is safe here — null pointers and None (0) are valid.
    arr
});

/// Register a Linux C IRQ handler for the given IRQ vector.
pub fn register_kpi_handler(irq: u8, handler: KpiHandler, dev_id: *mut u8) {
    let mut table = IRQ_TABLE.lock();
    table[irq as usize] = KpiIrqEntry { handler: Some(handler), dev_id };
}

/// Remove a registered KPI IRQ handler.
pub fn unregister_kpi_handler(irq: u8) {
    let mut table = IRQ_TABLE.lock();
    table[irq as usize] = KpiIrqEntry::empty();
}

/// Called from the Ainux IDT interrupt dispatcher.
/// Returns true if a KPI handler was found and called.
pub fn dispatch_kpi_irq(irq: u8) -> bool {
    // Lock briefly to copy the handler pointer — don't hold it during the call
    let (handler, dev_id) = {
        let table = IRQ_TABLE.lock();
        let entry = &table[irq as usize];
        (entry.handler, entry.dev_id)
    };
    if let Some(h) = handler {
        unsafe { h(irq as i32, dev_id); }
        true
    } else {
        false
    }
}

/// Enable an IRQ on the 8259 PIC (master/slave).
pub fn enable_irq(irq: u8) {
    unsafe {
        if irq < 8 {
            let mask: u8 = core::arch::x86_64::__cpuid(0).eax as u8; // placeholder read
            // Read current mask and clear the bit
            let mut mask: u8;
            core::arch::asm!("in al, 0x21", out("al") mask, options(nostack));
            mask &= !(1 << irq);
            core::arch::asm!("out 0x21, al", in("al") mask, options(nostack));
        } else {
            let mut mask: u8;
            core::arch::asm!("in al, 0xA1", out("al") mask, options(nostack));
            mask &= !(1 << (irq - 8));
            core::arch::asm!("out 0xA1, al", in("al") mask, options(nostack));
        }
    }
}

/// Disable an IRQ on the 8259 PIC.
pub fn disable_irq(irq: u8) {
    unsafe {
        if irq < 8 {
            let mut mask: u8;
            core::arch::asm!("in al, 0x21", out("al") mask, options(nostack));
            mask |= 1 << irq;
            core::arch::asm!("out 0x21, al", in("al") mask, options(nostack));
        } else {
            let mut mask: u8;
            core::arch::asm!("in al, 0xA1", out("al") mask, options(nostack));
            mask |= 1 << (irq - 8);
            core::arch::asm!("out 0xA1, al", in("al") mask, options(nostack));
        }
    }
}
