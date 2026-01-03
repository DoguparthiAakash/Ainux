#ifndef _PCI_H
#define _PCI_H

#include <stdint.h>

/* PCI Config Space Ports */
#define PCI_CONFIG_ADDRESS 0xCF8
#define PCI_CONFIG_DATA    0xCFC

/* Config Offsets */
#define PCI_VENDOR_ID      0x00
#define PCI_DEVICE_ID      0x02
#define PCI_COMMAND        0x04
#define PCI_STATUS         0x06
#define PCI_REVISION_ID    0x08
#define PCI_PROG_IF        0x09
#define PCI_SUBCLASS       0x0A
#define PCI_CLASS_CODE     0x0B
#define PCI_HEADER_TYPE    0x0E
#define PCI_CAPABILITIES_PTR 0x34

/* Capability IDs */
#define PCI_CAP_ID_MSI     0x05
#define PCI_CAP_ID_MSIX    0x11
/* MSI Offsets */
#define PCI_MSI_CTRL       0x02
#define PCI_MSI_ADDR_LO    0x04
#define PCI_MSI_ADDR_HI    0x08
#define PCI_MSI_DATA       0x0C

void pci_init(void);
void pci_scan_bus(void);
uint32_t pci_read_config(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset);
void pci_write_config(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset, uint32_t value);
int pci_find_device(uint16_t vendor_id, uint16_t device_id, uint8_t *bus, uint8_t *slot, uint8_t *func);
uint8_t pci_find_capability(uint8_t bus, uint8_t slot, uint8_t func, uint8_t cap_id);
int pci_enable_msi(uint8_t bus, uint8_t slot, uint8_t func, uint8_t vector);

#endif
