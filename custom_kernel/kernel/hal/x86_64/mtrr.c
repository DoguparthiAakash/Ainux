#include "cpu.h"
#include <stdint.h>
#include "../../log.h"

/* MSR Constants */
#define MSR_MTRR_CAP        0xFE
#define MSR_MTRR_DEF_TYPE   0x2FF
#define MSR_MTRR_PHYSBASE0  0x200
#define MSR_MTRR_PHYSMASK0  0x201

#define MTRR_TYPE_UC        0x00
#define MTRR_TYPE_WC        0x01
#define MTRR_TYPE_WT        0x04
#define MTRR_TYPE_WP        0x05
#define MTRR_TYPE_WB        0x06

#define MTRR_ENABLE         (1 << 11)
#define MTRR_FIXED_ENABLE   (1 << 10)

static void native_cpuid(uint32_t leaf, uint32_t *eax, uint32_t *ebx, uint32_t *ecx, uint32_t *edx) {
    __asm__ volatile ("cpuid"
        : "=a"(*eax), "=b"(*ebx), "=c"(*ecx), "=d"(*edx)
        : "a"(leaf));
}

static void native_wrmsr(uint32_t msr, uint64_t val) {
    uint32_t lo = val & 0xFFFFFFFF;
    uint32_t hi = val >> 32;
    __asm__ volatile ("wrmsr" : : "c"(msr), "a"(lo), "d"(hi));
}

static uint64_t native_rdmsr(uint32_t msr) {
    uint32_t lo, hi;
    __asm__ volatile ("rdmsr" : "=a"(lo), "=d"(hi) : "c"(msr));
    return ((uint64_t)hi << 32) | lo;
}

static uint8_t get_physical_address_bits(void) {
    uint32_t eax, ebx, ecx, edx;
    native_cpuid(0x80000008, &eax, &ebx, &ecx, &edx);
    return eax & 0xFF;
}

/* Set Memory Type Range Register */
/* debug helper */
static void print_hex(uint64_t val) {
    kprint("0x");
    for (int i = 60; i >= 0; i -= 4) {
        int nibble = (val >> i) & 0xF;
        char c = (nibble < 10) ? ('0' + nibble) : ('A' + nibble - 10);
        char str[2] = {c, 0};
        kprint(str);
    }
}

void mtrr_set_wc(uint64_t base, uint64_t size) {
    /* 1. Check Support */
    uint32_t eax = 0, ebx = 0, ecx = 0, edx = 0;
    native_cpuid(1, &eax, &ebx, &ecx, &edx);
    if (!(edx & (1 << 12))) {
        kprint("[MTRR] Not supported.\n");
        return;
    }
    
    /* 2. Read Cap */
    uint64_t cap = native_rdmsr(MSR_MTRR_CAP);
    int vcnt = cap & 0xFF;
    if (vcnt == 0) return;
    
    kprint("[MTRR] Detected variable MTRRs.\n");
    
    /* 3. Find Free Pair */
    int mtrr_idx = -1;
    for (int i = 0; i < vcnt; i++) {
        uint64_t mask = native_rdmsr(MSR_MTRR_PHYSMASK0 + i * 2);
        if (!(mask & (1 << 11))) { /* Valid Bit */
            mtrr_idx = i;
            break;
        }
    }
    
    if (mtrr_idx == -1) {
        kprint("[MTRR] No free MTRR registers.\n");
        return;
    }
    
    /* 4. Calculate Mask */
    /* MTRR requires P2 size and Base alignment */
    
    /* Round UP to next power of 2 */
    uint64_t pot_size = 1;
    while (pot_size < size) pot_size <<= 1;
    
    /* Check alignment of Base */
    if (base & (pot_size - 1)) {
        kprint("[MTRR] Base not aligned to PoT size. Scaling down...\n");
        kprint("[MTRR] Aborting WC to avoid GPF.\n");
        return; 
    }
    
    uint64_t phys_addr_bits = get_physical_address_bits();
    
    /* Sanity Check / Clamp PhysBits */
    /* VBox often GPFs if we assume 36 but it implements 32 or requires specific handling. */
    /* Defaulting to 32 is safer: it avoids setting reserved bits. */
    if (phys_addr_bits < 32 || phys_addr_bits > 52) {
        kprint("[MTRR] Abnormal PhysBits: "); print_hex(phys_addr_bits); kprint("\n");
        phys_addr_bits = 32; /* Safe Default (Avoids GPF) */
    }
    
    /* For VirtualBox specifically, sometimes it reports 40 but strict MTRR mask expects 36? */
    /* Or the other way around. Let's trust CPUID but cap at 36 if it fails? */
    /* Actually, generic VBox usually is 36 or 40. */
    
    uint64_t mask = (~(pot_size - 1)) & ((1ULL << phys_addr_bits) - 1);
    
    /* 5. Write Pair */
    /* Base: Addr | Type (WC=1) */
    uint64_t base_val = (base & 0xFFFFFFFFFF000ULL) | MTRR_TYPE_WC;
    
    /* Mask: Mask | Valid (11) */
    uint64_t mask_val = (mask & 0xFFFFFFFFFF000ULL) | (1 << 11);
    
    kprint("[MTRR] Setting WC. Base: "); 
    print_hex(base_val);
    kprint(" Mask: ");
    print_hex(mask_val);
    kprint("\n");
    
    native_wrmsr(MSR_MTRR_PHYSBASE0 + mtrr_idx * 2, base_val);
    native_wrmsr(MSR_MTRR_PHYSMASK0 + mtrr_idx * 2, mask_val);
    
    kprint("[MTRR] WC enabled for Framebuffer.\n");
}
