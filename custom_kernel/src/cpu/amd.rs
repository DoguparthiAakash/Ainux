use core::arch::asm;
use core::fmt::Write;

// AMD MSRs
pub const MSR_EFER: u32 = 0xC000_0080;
pub const MSR_HWCR: u32 = 0xC001_0015;
pub const MSR_VM_CR: u32 = 0xC001_0114;

// EFER Bits
pub const EFER_SCE: u64 = 1 << 0; // System Call Extensions
pub const EFER_SVME: u64 = 1 << 12; // Secure Virtual Machine Enable

fn rdmsr(msr: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        asm!("rdmsr", in("ecx") msr, out("eax") lo, out("edx") hi, options(nomem, nostack));
    }
    ((hi as u64) << 32) | (lo as u64)
}

fn wrmsr(msr: u32, value: u64) {
    let lo = value as u32;
    let hi = (value >> 32) as u32;
    unsafe {
        asm!("wrmsr", in("ecx") msr, in("eax") lo, in("edx") hi, options(nomem, nostack));
    }
}

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "AMD: Initializing AMD-specific features...\n");

    // Check for Secure Virtual Machine (SVM) support
    // CPUID Fn8000_0001_ECX[SVM] (bit 2)
    let (_, _, ecx, _) = crate::cpu::cpuid::cpuid(0x8000_0001);
    if (ecx & (1 << 2)) != 0 {
        let _ = write!(serial, "AMD: SVM (Secure Virtual Machine) virtualization is supported.\n");
        // We could enable SVME in EFER here if we were writing a hypervisor
        // let efer = rdmsr(MSR_EFER);
        // wrmsr(MSR_EFER, efer | EFER_SVME);
    } else {
        let _ = write!(serial, "AMD: SVM not supported or disabled in BIOS.\n");
    }

    // Enable SYSCALL/SYSRET (SCE) in EFER
    let mut efer = rdmsr(MSR_EFER);
    if (efer & EFER_SCE) == 0 {
        efer |= EFER_SCE;
        wrmsr(MSR_EFER, efer);
        let _ = write!(serial, "AMD: Enabled SYSCALL/SYSRET (SCE) in EFER.\n");
    }

    // Check for Invariant TSC (CPUID 0x8000_0007 EDX bit 8)
    let (_, _, _, edx_7) = crate::cpu::cpuid::cpuid(0x8000_0007);
    if (edx_7 & (1 << 8)) != 0 {
        let _ = write!(serial, "AMD: Invariant TSC is supported. Timer will not drift with P-states.\n");
    } else {
        let _ = write!(serial, "AMD: Warning: Invariant TSC NOT supported. Potential timer drift.\n");
    }

    // Optionally check HWCR (Hardware Configuration Register) for TLB/Cache optimizations
    // (We only read/log it here to avoid crashing older AMDs if not supported)
    let hwcr = rdmsr(MSR_HWCR);
    let _ = write!(serial, "AMD: MSR_HWCR = {:#x}\n", hwcr);

    let _ = write!(serial, "AMD: Initialization complete.\n");
}
