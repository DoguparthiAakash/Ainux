#include "../pci.h"
#include "../../io.h"
#include "../../log.h"
#include "xhci.h"

static uint8_t xhci_bus = 0;
static uint8_t xhci_slot = 0;
static uint8_t xhci_func = 0;
static int xhci_found = 0;

void xhci_init(void) {
    kprint("[XHCI] Searching for USB 3.0 Controller...\n");
    
    /* Brute force scan for Class 0x0C (Serial Bus), Subclass 0x03 (USB), ProgIF 0x30 (XHCI) */
    /* simplified: rely on pci_find_device if we knew vendor/device, 
       but we need to scan by class. 
       Let's iterate. 
    */
    
    for (uint16_t bus = 0; bus < 256; bus++) {
        for (uint8_t slot = 0; slot < 32; slot++) {
            for (uint8_t func = 0; func < 8; func++) {
                uint16_t vendor = pci_read_config(bus, slot, func, PCI_VENDOR_ID) & 0xFFFF;
                if (vendor == 0xFFFF) continue;
                
                uint8_t class_code = pci_read_config(bus, slot, func, PCI_CLASS_CODE) >> 24;
                uint8_t subclass = (pci_read_config(bus, slot, func, PCI_SUBCLASS) >> 16) & 0xFF;
                uint8_t prog_if = (pci_read_config(bus, slot, func, PCI_PROG_IF) >> 8) & 0xFF;
                
                if (class_code == 0x0C && subclass == 0x03 && prog_if == 0x30) {
                    xhci_bus = bus;
                    xhci_slot = slot;
                    xhci_func = func;
                    xhci_found = 1;
                    kprint("[XHCI] Found Controller at ");
                    /* print B:D:F? need print_hex helpers visible or kprint formating. 
                       We have kprint (string). 
                    */
                    kprint("PCI Bus.\n"); // Placeholder
                    goto found;
                }
            }
        }
    }
    
found:
    if (xhci_found) {
        /* Enable Bus Master */
        uint32_t cmd = pci_read_config(xhci_bus, xhci_slot, xhci_func, PCI_COMMAND);
        cmd |= 0x06; /* Bus Master (2) + Memory Space (4) */
        pci_write_config(xhci_bus, xhci_slot, xhci_func, PCI_COMMAND, cmd);
        
        /* Enable MSI */
        kprint("[XHCI] Attempting to enable MSI...\n");
        if (pci_enable_msi(xhci_bus, xhci_slot, xhci_func, 42)) { /* Vector 42 */
            kprint("[XHCI] MSI Enabled (Vector 42).\n");
        } else {
            kprint("[XHCI] MSI Failed or Not Supported.\n");
        }
        
    } else {
        kprint("[XHCI] No USB 3.0 Controller found.\n");
    }
}
