use core::sync::atomic::{AtomicU64, Ordering};
use core::arch::asm;

static PRNG_STATE: AtomicU64 = AtomicU64::new(0);

fn rdtsc() -> u64 {
    let mut low: u32;
    let mut high: u32;
    unsafe {
        asm!("rdtsc", out("eax") low, out("edx") high, options(nomem, nostack));
    }
    ((high as u64) << 32) | (low as u64)
}

/// Initialize PRNG with a seed. If 0 is provided, uses rdtsc.
pub fn init(seed: u64) {
    let mut actual_seed = seed;
    if actual_seed == 0 {
        actual_seed = rdtsc();
    }
    // Make sure seed is never 0
    if actual_seed == 0 {
        actual_seed = 0x1234567890ABCDEF;
    }
    PRNG_STATE.store(actual_seed, Ordering::Relaxed);
}

/// XorShift64* PRNG
pub fn get_random_u64() -> u64 {
    let mut x = PRNG_STATE.load(Ordering::Relaxed);
    if x == 0 {
        x = rdtsc();
        if x == 0 {
            x = 0x1234567890ABCDEF;
        }
    }
    
    // XorShift
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    
    // Store back state
    PRNG_STATE.store(x, Ordering::Relaxed);
    
    // Multiply by a constant (from XorShift*)
    x.wrapping_mul(0x2545F4914F6CDD1D)
}

pub fn get_random_bytes(buf: &mut [u8]) {
    let mut i = 0;
    while i < buf.len() {
        let rand = get_random_u64();
        let bytes = rand.to_le_bytes();
        let copy_len = core::cmp::min(8, buf.len() - i);
        buf[i..i+copy_len].copy_from_slice(&bytes[..copy_len]);
        i += copy_len;
    }
}
