#include "cpu.h"
#include <libc/stdio.h>
#include "log.h"

/* MSR Constants */
#define MSR_EFER     0xC0000080
#define MSR_STAR     0xC0000081
#define MSR_LSTAR    0xC0000082
#define MSR_SFMASK   0xC0000084

#define EFER_SCE     0x01

/* GDT Selectors (from gdt.h) */
#define KERNEL_CODE  0x08
#define USER_CODE    0x18
/* STAR layout: 
   Bits 63-48: User Code Selector (sysret adds +16 for SS, +0 for CS? No, sysret loads CS=Selector+16, SS=Selector+8).
   Actually, STAR expects:
   - Syscall CS/SS (Bits 47-32): Kernel Code (0x08). Target CS=0x08, SS=0x10.
   - Sysret CS/SS (Bits 63-48): User Code Base. Target CS=0x10+16 (0x23??), SS=0x10+8?
   Let's check docs carefully.
   
   STAR details:
   31:0  - Target IP for legacy mode (unused in long mode)
   47:32 - Syscall CS/SS Selectors (Ring 0). 
           CS = Field, SS = Field + 8.
           If we put 0x08 (Kernel Code), CS=0x08, SS=0x10 (Kernel Data). Correct.
   63:48 - Sysret CS/SS Selectors (Ring 3).
           CS = Field + 16, SS = Field + 8.
           If we use 0x08 (Kernel Code)? No.
           We want User CS (0x1B) and User SS (0x23).
           Field + 16 = 0x1B  => Field = 0x0B? Alignment issues.
           Usually OSes arrange GDT such that:
           User Code 32 (unused), User Data (0x20), User Code 64 (0x18)?
           Or User Code (0x18), User Data (0x20).
           
           If we put 0x08 (Kernel Code) in 63:48?
           CS = 0x08 + 16 = 0x18. (User Code). Correct.
           SS = 0x08 + 8  = 0x10. (Kernel Data? No, we want User Data 0x20).
           This implies User Data MUST be at UserCode + 8.
           Our GDT:
           1: Kernel Code (0x08)
           2: Kernel Data (0x10)
           3: User Code (0x18)
           4: User Data (0x20)
           
           So if Field=0x13 ?? No segment selectors are 0x10 aligned usually.
           Wait. 
           Selector 0x10 (Kernel Data).
           Field = 0x10 (Kernel Data).
           CS = 0x10 + 16 = 0x20 (User Data?? No we want Code).
           SS = 0x10 + 8 = 0x18 (User Code?? No we want Data).
           
           Actually, the instruction works this way:
           SYSRET:
           CS = STAR[63:48] + 16
           SS = STAR[63:48] + 8
           
           To get CS=0x1B (User Code | 3) and SS=0x23 (User Data | 3).
           We need base selectors 0xXX.
           Base + 16 = 0x18 (User Code). -> Base = 0x08.
           Base + 8 = 0x10 (Kernel Data/User Data?). -> We want 0x20 (User Data).
           Difference is 8.
           But input UserCode is 0x18, UserData is 0x20. Distance is 8.
           Equation:
           Base + 16 = 0x18  => Base = 0x08.
           Base + 8 = 0x10   => But we want 0x20!
           
           So if Base=0x08:
           CS becomes 0x18.
           SS becomes 0x10 (Kernel Data). This is WRONG for Userspace SS.
           
           This suggests GDT layout must be:
           ...
           ? : User Data (SS)
           ? : User Code (CS)
           
           Or:
           Linux uses:
           ...
           
           Let's look at syscall instruction behaviour:
           "SYSRET loads CS from STAR[63:48] + 16. SS from STAR[63:48] + 8".
           Wait, Intel might be different from AMD? Or I have logic reversed.
           AMD Manual:
           "CS selector = STAR[63:48] + 16"
           "SS selector = STAR[63:48] + 8"
           
           So GDT must be:
           Start + 8  = SS (User Data)
           Start + 16 = CS (User Code)
           
           Wait, User Data comes BEFORE User Code?
           Let's re-read GDT layout in gdt.c:
           3: User Code (0x18)
           4: User Data (0x20)
           
           0x20 is > 0x18.
           Code is BEFORE Data.
           Equation:
           Base+16 = Code (0x18)
           Base+8 = Data (0x20) => Impossible if Base constant. 16 > 8. Code must be > Data?
           No.
           If Base=0x10:
           CS = 0x10+16 = 0x20 (User Data used as Code? No).
           SS = 0x10+8 = 0x18 (User Code used as Data? No).
           
           If I swap GDT entries?
           3: User Data (0x18)
           4: User Code (0x20)
           
           Base = 0x10.
           CS = 0x10+16 = 0x20 (User Code). Correct.
           SS = 0x10+8 = 0x18 (User Data). Correct.
           
           SO: I MUST SWAP GDT ENTRIES 3 AND 4 in gdt.c!
           And update gdt.h defines.
*/

static void wrmsr(uint32_t msr, uint64_t val) {
    uint32_t lo = val & 0xFFFFFFFF;
    uint32_t hi = val >> 32;
    __asm__ volatile ("wrmsr" : : "c"(msr), "a"(lo), "d"(hi));
}

extern void syscall_entry(void);

void cpu_init_syscalls(void) {
    kprint("Initializing Syscalls (MSRs)...\n");
    
    /* 1. Enable SCE (System Call Extensions) in EFER */
    /* Read EFER, set bit 0, write back */
    /* Actually needed for 'syscall' instruction */
    /* Assuming standard EFER location 0xC0000080 */
    uint32_t lo, hi;
    __asm__ volatile ("rdmsr" : "=a"(lo), "=d"(hi) : "c"(MSR_EFER));
    lo |= EFER_SCE;
    wrmsr(MSR_EFER, ((uint64_t)hi << 32) | lo);
    
    /* 2. Setup STAR */
    /* Bits 63-48: User Base (0x10 -> CS=0x20, SS=0x18) IF I swap GDT. */
    /* Bits 47-32: Kernel Base (0x08 -> CS=0x08, SS=0x10). */
    uint64_t star = 0;
    star |= (uint64_t)0x08 << 32; /* Kernel Base */
    star |= (uint64_t)0x10 << 48; /* User Base (Assuming GDT Swap) */
    wrmsr(MSR_STAR, star);
    
    /* 3. Setup LSTAR (Entry Point) */
    wrmsr(MSR_LSTAR, (uint64_t)syscall_entry);
    
    /* 4. Setup SFMASK (Flags to clear on syscall) */
    /* Clear IF (Interrupts) to prevent nested interrupts on kernel stack swap? */
    /* Usually 0x200 (IF) */
    wrmsr(MSR_SFMASK, 0x200); 
    
    kprint("Syscalls Enabled.\n");
}
       /* For kprint */

static struct cpu_info_t g_cpu_info;

void cpuid(uint32_t leaf, uint32_t *eax, uint32_t *ebx, uint32_t *ecx, uint32_t *edx) {
    __asm__ volatile ("cpuid"
        : "=a"(*eax), "=b"(*ebx), "=c"(*ecx), "=d"(*edx)
        : "a"(leaf));
}

static void cpu_detect(void) {
    uint32_t eax, ebx, ecx, edx;
    
    /* Get Vendor */
    cpuid(0, &eax, &ebx, &ecx, &edx);
    
    /* Reconstruct vendor string from EBX, EDX, ECX (order is important) */
    /* Vendor string is 12 chars + null terminator */
    uint32_t *vendor_ptr = (uint32_t *)&g_cpu_info.vendor_string[0];
    vendor_ptr[0] = ebx;
    vendor_ptr[1] = edx;
    vendor_ptr[2] = ecx;
    g_cpu_info.vendor_string[12] = '\0';
    
    /* Get Family/Model/Stepping from Leaf 1 */
    cpuid(1, &eax, &ebx, &ecx, &edx);
    
    g_cpu_info.stepping = eax & 0xF;
    g_cpu_info.model = (eax >> 4) & 0xF;
    g_cpu_info.family = (eax >> 8) & 0xF;
    
    /* Extended Model/Family handling */
    if (g_cpu_info.family == 6 || g_cpu_info.family == 15) {
        g_cpu_info.model += ((eax >> 16) & 0xF) << 4;
    }
    if (g_cpu_info.family == 15) {
        g_cpu_info.family += (eax >> 20) & 0xFF;
    }
}

void cpu_init(void) {
    kprint("Initializing CPU Support...\n");
    
    cpu_detect();
    cpu_init_syscalls();
    
    kprint("CPU Vendor: ");
    kprint(g_cpu_info.vendor_string);
    kprint("\n");
    
    /* Vendor Specific Initialization */
    // int is_amd = 0;
    // int is_intel = 0;
    
    /* Simple string compare (we don't have strcmp from libc yet in this context, or maybe we do, but let's be safe) */
    /* Check for "AuthenticAMD" */
    const char *amd_str = CPU_VENDOR_AMD;
    const char *intel_str = CPU_VENDOR_INTEL;
    
    int match_amd = 1;
    for(int i=0; i<12; i++) { if(g_cpu_info.vendor_string[i] != amd_str[i]) match_amd = 0; }
    
    int match_intel = 1;
    for(int i=0; i<12; i++) { if(g_cpu_info.vendor_string[i] != intel_str[i]) match_intel = 0; }
    
    if (match_amd) {
        // is_amd = 1;
        kprint_color(KLOG_COLOR_CYAN, "Detected AMD CPU.\n");
        /* AMD Specific Init here usually implies checking for SVM (virtualization) or MSRs */
        /* For now, we just acknowledge it */
    } else if (match_intel) {
        // is_intel = 1;
        kprint_color(KLOG_COLOR_CYAN, "Detected Intel CPU.\n");
    } else {
        kprint_color(KLOG_COLOR_YELLOW, "Unknown CPU Vendor.\n");
    }
    
    /* Report Family/Model */
    kprint("CPU Family: ");
    char num_buf[16];
    int idx;
    
    /* Print Family */
    {
        uint32_t n = g_cpu_info.family;
        idx = 0;
        if (n == 0) num_buf[idx++] = '0';
        else { while(n) { num_buf[idx++] = '0' + (n % 10); n /= 10; } }
        for(int i=idx-1; i>=0; i--) { char str[2] = {num_buf[i], 0}; kprint(str); }
    }
    
    kprint(" Model: ");
    {
        uint32_t n = g_cpu_info.model;
        idx = 0;
        if (n == 0) num_buf[idx++] = '0';
        else { while(n) { num_buf[idx++] = '0' + (n % 10); n /= 10; } }
        for(int i=idx-1; i>=0; i--) { char str[2] = {num_buf[i], 0}; kprint(str); }
    }
    kprint("\n");
}

const struct cpu_info_t* cpu_get_info(void) {
    return &g_cpu_info;
}
