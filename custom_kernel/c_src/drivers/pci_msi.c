#include "pci.h"
#include "../io.h"
#include "../log.h"

/* 
 * PCI Capability List Traversal 
 * Returns the offset of the requested capability, or 0 if not found.
 */
uint8_t pci_find_capability(uint8_t bus, uint8_t slot, uint8_t func, uint8_t cap_id) {
    uint8_t status = (pci_read_config(bus, slot, func, PCI_STATUS) >> 16) & 0xFFFF;
    
    /* Check for Capabilities List bit (Bit 4) */
    if (!(status & (1 << 4))) {
        return 0; 
    }
    
    uint8_t cap_ptr = pci_read_config(bus, slot, func, PCI_CAPABILITIES_PTR) & 0xFF;
    
    while (cap_ptr != 0) {
        uint32_t cap_header = pci_read_config(bus, slot, func, cap_ptr);
        uint8_t id = cap_header & 0xFF;
        uint8_t next = (cap_header >> 8) & 0xFF;
        
        if (id == cap_id) {
            return cap_ptr;
        }
        
        cap_ptr = next;
    }
    
    return 0;
}

/*
 * Enable MSI for a device
 * Helper to set a simple default vector.
 */
int pci_enable_msi(uint8_t bus, uint8_t slot, uint8_t func, uint8_t vector) {
    uint8_t cap_offset = pci_find_capability(bus, slot, func, PCI_CAP_ID_MSI);
    if (!cap_offset) {
        return 0; /* No MSI support */
    }
    
    /* Read Control */
    uint16_t msi_ctrl = (pci_read_config(bus, slot, func, cap_offset + PCI_MSI_CTRL) >> 16) & 0xFFFF;
    int is_64bit = (msi_ctrl & 0x0080);
    
    /* Address: Same as local APIC logic usually. 0xFEE00000 | (ProcessorID << 12) */
    /* Dest ID 0 (BSP), No Redirection. */
    uint32_t address = 0xFEE00000; 
    uint32_t data = vector & 0xFF; /* Delivery: Fixed, Edge, Active High */
    
    /* Write Address */
    pci_write_config(bus, slot, func, cap_offset + PCI_MSI_ADDR_LO, address);
    if (is_64bit) {
        pci_write_config(bus, slot, func, cap_offset + PCI_MSI_ADDR_HI, 0);
        pci_write_config(bus, slot, func, cap_offset + PCI_MSI_DATA + 4, data);
    } else {
        pci_write_config(bus, slot, func, cap_offset + PCI_MSI_DATA, data);
    }
    
    /* Enable MSI in Control Register (Bit 0) */
    /* Also clear Multiple Message Enable (Bits 4-6) to 0 (1 vector) */
    msi_ctrl &= ~0x0070; 
    msi_ctrl |= 0x0001;
    
    /* Write back control (Word access via 32-bit wrapper potentially tricky, masking needed) */
    /* pci_write_config writes 32 bits. We need to preserve adjacent bytes? 
       Yes. pci_write_config takes aligned offset usually? 
       Actually our implementation does 32-bit write. 
       Let's assume offsets are 32-bit aligned for simplicity or do RMW.
    */
    uint32_t ctrl_dw = pci_read_config(bus, slot, func, cap_offset);
    /* Control is in the upper word of the first DWord (ID + Next + Ctrl) */
    /* Wait, Offset 0: ID (8), Next (8), Ctrl (16) */
    /* So cap_offset points to ID. */
    
    ctrl_dw &= 0x0000FFFF; /* Clear old control */
    ctrl_dw |= (msi_ctrl << 16);
    
    pci_write_config(bus, slot, func, cap_offset, ctrl_dw);
    
    kprint("[PCI] MSI Enabled for device.\n");
    return 1;
}
