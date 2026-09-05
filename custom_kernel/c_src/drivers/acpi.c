#include "acpi.h"
#include "../io.h"
#include "../libc/stdio.h"
#include "../libc/string.h"
#include "../libc/stdlib.h"
#include "boot_info.h"

extern void kprint(const char *msg);

/* Globals for Shutdown */
static uint32_t pm1a_cnt_blk = 0;
static uint32_t pm1b_cnt_blk = 0;
static uint16_t slp_typa = 0;
static uint16_t slp_typb = 0;
static int acpi_enabled = 0;

static uint32_t smi_cmd = 0;
static uint8_t acpi_enable = 0;
static uint8_t acpi_disable = 0;

/* Debug helper */
static void sys_halt_loop(void);

static void print_ptr(uint64_t v) {
    /* Manual hex since we don't include custom headers here */
    kprint("0x");
    const char *digits = "0123456789ABCDEF";
    for(int i=60; i>=0; i-=4) {
        kprint((char[]){digits[(v>>i)&0xF], 0});
    }
}

/* Helper: Convert Physical to Virtual using HHDM */
static void *phys_to_virt(uint64_t phys) {
    if (phys == 0) return NULL;
    return (void *)(phys + g_hhdm_offset);
}

static void *acpi_find_table(struct acpi_header *rsdt, const char *sig) {
    if (!rsdt) return NULL;
    uint32_t entries = (rsdt->length - sizeof(struct acpi_header)) / 4;
    uint32_t *pointers = (uint32_t *)(rsdt + 1);
    
    for (uint32_t i = 0; i < entries; i++) {
        /* Pointers in RSDT are Physical 32-bit addresses */
        uint64_t phys = (uint64_t)pointers[i];
        struct acpi_header *h = (struct acpi_header *)phys_to_virt(phys);
        if (memcmp(h->signature, sig, 4) == 0) return h;
    }
    return NULL;
}

/* Parse DSDT to find _S5 package */
/* This is a hacky parser looking for byte patterns */
static int acpi_parse_dsdt(uint8_t *dsdt, uint32_t len) {
    /* Look for "_S5_" which is 5F 53 35 5F */
    for (uint32_t i = 0; i < len - 12; i++) {
        if (memcmp(&dsdt[i], "_S5_", 4) == 0) {
            /* Found _S5_ */
            uint8_t *p = &dsdt[i+4];
            
            /* Start crude parsing */
            int found_pkg = 0;
            for(int k=0; k<16; k++) {
                if (p[k] == 0x12) { /* Package Op */
                    p = &p[k];
                    found_pkg = 1;
                    break;
                }
            }
            if (!found_pkg) continue;
            
            p++; /* Skip 0x12 */
            
            /* Parse PkgLength */
            uint8_t b0 = *p;
            int bytes_read = 0;
            
            if ((b0 & 0xC0) == 0) {
                 bytes_read = 1;
            } else if ((b0 & 0xC0) == 0x40) {
                 bytes_read = 2;
            } else {
                 bytes_read = 1;
            }
            p += bytes_read;
            
            p++; /* NumElements */
            
            /* Now finding SLP_TYPa */
             if (*p == 0x0A) p++; /* BytePrefix */
             slp_typa = *p << 10;
             p++;
             
             /* Element 1: PM1b_CNT val */
             if (*p == 0x0A) p++;
             slp_typb = *p << 10;
             
             /* kprint("ACPI: Found _S5_ object. Ready for shutdown.\n"); */
             return 1;
        }
    }
    return 0;
}

void acpi_init(uint64_t rsdp_addr) {
    kprint("ACPI: Initializing...\n");
    
    if (rsdp_addr == 0) {
        kprint("ACPI: RSDP is NULL.\n");
        return;
    }
    
    /* RSDP pass from Limine is ALREADY VIRTUAL (HHDM mapped) */
    struct rsdp_descriptor *rsdp = (struct rsdp_descriptor *)rsdp_addr;
    
    /* Verify Signature */
    if (memcmp(rsdp->signature, "RSD PTR ", 8) != 0) {
        kprint("ACPI: Invalid RSDP signature.\n");
        return;
    }
    
    /* Get RSDT */
    struct acpi_header *rsdt = (struct acpi_header *)phys_to_virt((uint64_t)rsdp->rsdt_address);
    if (!rsdt) return;
    
    /* Find FADT (Signature "FACP") */
    struct fadt_header *fadt = (struct fadt_header *)acpi_find_table(rsdt, "FACP");
    if (!fadt) {
        kprint("ACPI: FADT not found.\n");
        return;
    }
    
    /* Store Control Blocks */
    pm1a_cnt_blk = fadt->pm1a_control_block;
    pm1b_cnt_blk = fadt->pm1b_control_block;
    
    smi_cmd = fadt->smi_command_port;
    acpi_enable = fadt->acpi_enable;
    acpi_disable = fadt->acpi_disable;
    
    /* Parse DSDT */
    struct acpi_header *dsdt = (struct acpi_header *)phys_to_virt((uint64_t)fadt->dsdt);
    if (dsdt) {
        if (acpi_parse_dsdt((uint8_t*)dsdt + sizeof(struct acpi_header), dsdt->length - sizeof(struct acpi_header))) {
            acpi_enabled = 1;
            kprint("ACPI: Enabled (Shutdown Ready).\n");
        } else {
            kprint("ACPI: _S5_ not found in DSDT.\n");
        }
    }
}

void acpi_power_off(void) {
    /* Just-In-Time ACPI Enable */
    if (!acpi_enabled) {
        if (smi_cmd != 0 && acpi_enable != 0) {
            kprint("ACPI: Enabling Controller...\n");
            outb(smi_cmd, acpi_enable);
             
            /* Wait for SCI_EN (Bit 0 in PM1a_CNT) */
            /* Timeout loop */
            int i;
            for(i=0; i<500000; i++) {
                if (inw(pm1a_cnt_blk) & 1) break; 
                /* Busy wait */
                for(volatile int k=0; k<100; k++);
            }
            if (i >= 500000) kprint("ACPI: Timeout waiting for SCI_EN (proceeding anyway).\n");
            else {
                acpi_enabled = 1;
                kprint("ACPI: Controller Enabled.\n");
            }
        }
    }
    
    kprint("ACPI: Powering Off...\n");
    kprint("Debug: PM1a="); print_ptr(pm1a_cnt_blk); 
    kprint(" SLPa="); print_ptr(slp_typa); 
    kprint("\n");

    /* Validate we have a block to write to */
    if (pm1a_cnt_blk == 0) {
        kprint("ACPI: PM1a Block 0. Halt.\n");
        sys_halt_loop();
    }

    /* Write SLP_TYPa | SLP_EN (bit 13) */
    /* 0x2000 = Bit 13 (SLP_EN) */
    outw(pm1a_cnt_blk, slp_typa | 0x2000);
    
    if (pm1b_cnt_blk) {
        outw(pm1b_cnt_blk, slp_typb | 0x2000);
    }
    
    /* Fallback Loop */
    for(volatile int j=0; j<100000; j++);
    
    kprint("ACPI: Hardware didn't shutdown. Retrying...\n");
    outw(pm1a_cnt_blk, slp_typa | 0x2000);
    
    kprint("ACPI: Halted.\n");
    sys_halt_loop();
}

void sys_halt_loop(void) {
    for(;;) {
        __asm__ volatile ("cli; hlt");
    }
}
