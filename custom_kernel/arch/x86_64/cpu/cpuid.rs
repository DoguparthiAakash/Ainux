// =============================================================================
// Ainux CPUID — CPU Detection & Feature Enumeration
// Ported from legacy_archive/kernel/hal/x86_64/cpu.c to Rust
// =============================================================================

use core::arch::asm;
use core::fmt::Write;
use spin::Mutex;

#[derive(Debug, Clone)]
pub struct CpuFeatures {
    pub vendor: [u8; 13],
    pub brand: [u8; 49],
    pub family: u32,
    pub model: u32,
    pub stepping: u32,
    // Feature flags (EDX from leaf 1)
    pub has_sse: bool,
    pub has_sse2: bool,
    pub has_fpu: bool,
    pub has_apic: bool,
    pub has_msr: bool,
    pub has_pae: bool,
    pub has_pse: bool,
    // Feature flags (ECX from leaf 1)
    pub has_sse3: bool,
    pub has_sse41: bool,
    pub has_sse42: bool,
    pub has_avx: bool,
    pub has_aes: bool,
    pub has_x2apic: bool,
    // Extended features (leaf 0x80000001)
    pub has_nx: bool,
    pub has_long_mode: bool,
    pub has_1gb_pages: bool,
    pub has_rdtscp: bool,
    // Topology
    pub max_cpuid_leaf: u32,
    pub max_extended_leaf: u32,
    pub logical_cores: u32,
}

impl CpuFeatures {
    pub const fn new() -> Self {
        CpuFeatures {
            vendor: [0; 13],
            brand: [0; 49],
            family: 0,
            model: 0,
            stepping: 0,
            has_sse: false,
            has_sse2: false,
            has_fpu: false,
            has_apic: false,
            has_msr: false,
            has_pae: false,
            has_pse: false,
            has_sse3: false,
            has_sse41: false,
            has_sse42: false,
            has_avx: false,
            has_aes: false,
            has_x2apic: false,
            has_nx: false,
            has_long_mode: false,
            has_1gb_pages: false,
            has_rdtscp: false,
            max_cpuid_leaf: 0,
            max_extended_leaf: 0,
            logical_cores: 1,
        }
    }

    pub fn vendor_str(&self) -> &str {
        let len = self.vendor.iter().position(|&b| b == 0).unwrap_or(12);
        core::str::from_utf8(&self.vendor[..len]).unwrap_or("Unknown")
    }

    pub fn brand_str(&self) -> &str {
        let len = self.brand.iter().position(|&b| b == 0).unwrap_or(48);
        core::str::from_utf8(&self.brand[..len]).unwrap_or("")
    }

    pub fn is_amd(&self) -> bool {
        self.vendor_str() == "AuthenticAMD"
    }

    pub fn is_intel(&self) -> bool {
        self.vendor_str() == "GenuineIntel"
    }
}

pub static CPU_FEATURES: Mutex<CpuFeatures> = Mutex::new(CpuFeatures::new());

/// Execute CPUID instruction (saves/restores rbx to avoid LLVM conflict)
#[inline(always)]
pub fn cpuid(leaf: u32) -> (u32, u32, u32, u32) {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    unsafe {
        asm!(
            "push rbx",
            "cpuid",
            "mov {ebx_out:e}, ebx",
            "pop rbx",
            inout("eax") leaf => eax,
            ebx_out = out(reg) ebx,
            out("ecx") ecx,
            out("edx") edx,
            options(nostack, preserves_flags)
        );
    }
    (eax, ebx, ecx, edx)
}

/// Execute CPUID with subleaf (saves/restores rbx)
#[inline(always)]
pub fn cpuid_sub(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    unsafe {
        asm!(
            "push rbx",
            "cpuid",
            "mov {ebx_out:e}, ebx",
            "pop rbx",
            inout("eax") leaf => eax,
            ebx_out = out(reg) ebx,
            inout("ecx") subleaf => ecx,
            out("edx") edx,
            options(nostack, preserves_flags)
        );
    }
    (eax, ebx, ecx, edx)
}

/// Detect CPU and populate global features
pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "CPUID: Detecting CPU...\n");

    let mut features = CpuFeatures::new();

    // Leaf 0: Vendor string + max leaf
    let (max_leaf, ebx, ecx, edx) = cpuid(0);
    features.max_cpuid_leaf = max_leaf;

    // Reconstruct vendor: EBX, EDX, ECX order
    let vendor_bytes: [u8; 12] = unsafe {
        let mut buf = [0u8; 12];
        buf[0..4].copy_from_slice(&ebx.to_le_bytes());
        buf[4..8].copy_from_slice(&edx.to_le_bytes());
        buf[8..12].copy_from_slice(&ecx.to_le_bytes());
        buf
    };
    features.vendor[..12].copy_from_slice(&vendor_bytes);

    let _ = write!(serial, "CPUID: Vendor: {}\n", features.vendor_str());

    // Leaf 1: Feature flags + Family/Model/Stepping
    if max_leaf >= 1 {
        let (eax, ebx, ecx, edx) = cpuid(1);

        features.stepping = eax & 0xF;
        features.model = (eax >> 4) & 0xF;
        features.family = (eax >> 8) & 0xF;

        // Extended model/family
        if features.family == 6 || features.family == 15 {
            features.model += ((eax >> 16) & 0xF) << 4;
        }
        if features.family == 15 {
            features.family += (eax >> 20) & 0xFF;
        }

        // Logical processor count
        features.logical_cores = (ebx >> 16) & 0xFF;
        if features.logical_cores == 0 { features.logical_cores = 1; }

        // EDX features
        features.has_fpu = (edx & (1 << 0)) != 0;
        features.has_pse = (edx & (1 << 3)) != 0;
        features.has_msr = (edx & (1 << 5)) != 0;
        features.has_pae = (edx & (1 << 6)) != 0;
        features.has_apic = (edx & (1 << 9)) != 0;
        features.has_sse = (edx & (1 << 25)) != 0;
        features.has_sse2 = (edx & (1 << 26)) != 0;

        // ECX features
        features.has_sse3 = (ecx & (1 << 0)) != 0;
        features.has_sse41 = (ecx & (1 << 19)) != 0;
        features.has_sse42 = (ecx & (1 << 20)) != 0;
        features.has_aes = (ecx & (1 << 25)) != 0;
        features.has_avx = (ecx & (1 << 28)) != 0;
        features.has_x2apic = (ecx & (1 << 21)) != 0;

        let _ = write!(serial, "CPUID: Family {} Model {} Stepping {}\n",
            features.family, features.model, features.stepping);
        let _ = write!(serial, "CPUID: {} logical cores\n", features.logical_cores);
    }

    // Extended leaves
    let (max_ext, _, _, _) = cpuid(0x80000000);
    features.max_extended_leaf = max_ext;

    if max_ext >= 0x80000001 {
        let (_, _, ecx, edx) = cpuid(0x80000001);
        features.has_long_mode = (edx & (1 << 29)) != 0;
        features.has_nx = (edx & (1 << 20)) != 0;
        features.has_rdtscp = (edx & (1 << 27)) != 0;
        features.has_1gb_pages = (edx & (1 << 26)) != 0;
    }

    // Brand string (leaves 0x80000002 - 0x80000004)
    if max_ext >= 0x80000004 {
        let mut brand_buf = [0u8; 48];
        for i in 0..3u32 {
            let (a, b, c, d) = cpuid(0x80000002 + i);
            let off = (i as usize) * 16;
            brand_buf[off..off+4].copy_from_slice(&a.to_le_bytes());
            brand_buf[off+4..off+8].copy_from_slice(&b.to_le_bytes());
            brand_buf[off+8..off+12].copy_from_slice(&c.to_le_bytes());
            brand_buf[off+12..off+16].copy_from_slice(&d.to_le_bytes());
        }
        features.brand[..48].copy_from_slice(&brand_buf);
        let _ = write!(serial, "CPUID: Brand: {}\n", features.brand_str());
    }

    // Print feature summary
    let _ = write!(serial, "CPUID: Features: ");
    if features.has_sse { let _ = write!(serial, "SSE "); }
    if features.has_sse2 { let _ = write!(serial, "SSE2 "); }
    if features.has_sse3 { let _ = write!(serial, "SSE3 "); }
    if features.has_sse41 { let _ = write!(serial, "SSE4.1 "); }
    if features.has_sse42 { let _ = write!(serial, "SSE4.2 "); }
    if features.has_avx { let _ = write!(serial, "AVX "); }
    if features.has_aes { let _ = write!(serial, "AES-NI "); }
    if features.has_nx { let _ = write!(serial, "NX "); }
    if features.has_1gb_pages { let _ = write!(serial, "1GB-Pages "); }
    if features.has_x2apic { let _ = write!(serial, "x2APIC "); }
    if features.has_rdtscp { let _ = write!(serial, "RDTSCP "); }
    let _ = write!(serial, "\n");

    // Enable FPU + SSE (from legacy cpu.c)
    enable_sse();

    // Enable Write-Combining in PAT
    unsafe { init_pat(); }

    *CPU_FEATURES.lock() = features.clone();
    let _ = write!(serial, "CPUID: Detection complete\n");

    if features.is_amd() {
        crate::cpu::amd::init();
    }
}

/// Enable FPU and SSE via CR0/CR4 (ported from legacy cpu.c)
fn enable_sse() {
    unsafe {
        // CR0: Clear EM (bit 2), set MP (bit 1)
        let cr0: u64;
        asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack));
        let cr0 = (cr0 & !(1 << 2)) | (1 << 1);
        asm!("mov cr0, {}", in(reg) cr0, options(nomem, nostack));

        // CR4: Set OSFXSR (bit 9) + OSXMMEXCPT (bit 10)
        let cr4: u64;
        asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack));
        let cr4 = cr4 | (1 << 9) | (1 << 10);
        asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack));
    }
}

/// Configures the Page Attribute Table (PAT) to enable Write-Combining (WC).
/// The PAT MSR (0x277) holds 8 entries (8 bits each).
/// Default PAT1 is WT (Write-Through, 0x04).
/// We change PAT1 to WC (Write-Combining, 0x01).
unsafe fn init_pat() {
    let mut pat = crate::cpu::control::rdmsr(0x277);
    
    // Clear PAT1 (bits 8..15)
    pat &= !(0xFF << 8);
    // Set PAT1 to 0x01 (WC)
    pat |= 0x01 << 8;
    
    crate::cpu::control::wrmsr(0x277, pat);
}

/// Read the TSC (Time Stamp Counter) — useful for high-precision timing
#[inline(always)]
pub fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
    }
    ((hi as u64) << 32) | (lo as u64)
}
