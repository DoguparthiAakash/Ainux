#include "pci.h"
#include "../libc/stdio.h"

/* Wrapper for IO ports (from simple assembly or existing helpers) */
static inline void outl(uint16_t port, uint32_t val) {
    __asm__ volatile ( "outl %0, %1" : : "a"(val), "Nd"(port) );
}

static inline uint32_t inl(uint16_t port) {
    uint32_t ret;
    __asm__ volatile ( "inl %1, %0" : "=a"(ret) : "Nd"(port) );
    return ret;
}

uint32_t pci_read_config(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset) {
    /* Address Format:
       Bit 31: Enable bit
       Bits 30-24: Reserved
       Bits 23-16: Bus Number
       Bits 15-11: Device Number (Slot)
       Bits 10-8: Function Number
       Bits 7-2: Register Number (Offset & 0xFC)
       Bits 1-0: 00
    */
    uint32_t address = (uint32_t)((bus << 16) | (slot << 11) |
              (func << 8) | (offset & 0xFC) | ((uint32_t)0x80000000));
 
    outl(PCI_CONFIG_ADDRESS, address);
    
    /* (offset & 2) * 8) = 0 for 32-bit read usually, but standard says aligned read */
    return inl(PCI_CONFIG_DATA);
}

const char* pci_get_vendor_name(uint16_t vendor_id) {
    switch (vendor_id) {
        case 0x8086: return "Intel";
        case 0x1234: return "QEMU";
        case 0x10EC: return "Realtek";
        default: return "Unknown";
    }
}

const char* pci_get_device_name(uint16_t vendor_id, uint16_t device_id) {
    if (vendor_id == 0x8086) {
        switch (device_id) {
            case 0x1237: return "440FX Host Bridge";
            case 0x7000: return "PIIX3 ISA Bridge";
            case 0x7010: return "PIIX3 IDE Interface"; // If visible
            case 0x100E: return "E1000 Ethernet";
            case 0x7113: return "PIIX4 Power Management";
            case 0x29C0: return "Q35 Host Bridge";
            default: return "Intel Device";
        }
    } else if (vendor_id == 0x1234) {
        switch (device_id) {
            case 0x1111: return "Standard VGA";
            default: return "QEMU Device";
        }
    }
    return "Unknown Device";
}

void pci_check_device(uint8_t bus, uint8_t device, uint8_t function) {
    uint32_t vendor_reg = pci_read_config(bus, device, function, 0x00);
    uint16_t vendor_id = (uint16_t)(vendor_reg & 0xFFFF);
    
    if (vendor_id == 0xFFFF) return; /* Device doesn't exist */
    
    uint16_t device_id = (uint16_t)(vendor_reg >> 16);
    
    uint32_t class_reg = pci_read_config(bus, device, function, 0x08);
    uint8_t class_code = (uint8_t)(class_reg >> 24);
    uint8_t subclass = (uint8_t)(class_reg >> 16);
    
    const char *v_name = pci_get_vendor_name(vendor_id);
    const char *d_name = pci_get_device_name(vendor_id, device_id);
    
    printf("PCI %d:%d.%d [%s] %s (%x:%x)\n",
           bus, device, function, v_name, d_name, vendor_id, device_id);
}

void pci_scan_bus(void) {
    printf("Scanning PCI Bus...\n");
    for (uint16_t bus = 0; bus < 256; bus++) {
        for (uint8_t slot = 0; slot < 32; slot++) {
             pci_check_device((uint8_t)bus, slot, 0);
             /* TODO: Check multi-function bit to scan funcs 1-7 */
        }
    }
}
