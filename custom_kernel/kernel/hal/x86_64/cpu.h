#ifndef ARCH_X86_64_CPU_H
#define ARCH_X86_64_CPU_H

#include <stdint.h>

/* CPU Vendor IDs */
#define CPU_VENDOR_INTEL "GenuineIntel"
#define CPU_VENDOR_AMD   "AuthenticAMD"

/* CPU Features (simplified for now) */
struct cpu_info_t {
    char vendor_string[13];
    uint32_t model;
    uint32_t family;
    uint32_t stepping;
    /* Add more as needed: features flags etc */
};

/* Initialize CPU subsystem, detect vendor, enable specific features */
void cpu_init(void);

/* Get detected CPU info */
const struct cpu_info_t* cpu_get_info(void);

/* Helper to read CPUID */
void cpuid(uint32_t leaf, uint32_t *eax, uint32_t *ebx, uint32_t *ecx, uint32_t *edx);

#endif
