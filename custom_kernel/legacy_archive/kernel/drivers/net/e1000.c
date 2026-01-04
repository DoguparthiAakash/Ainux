#include "../pci.h"
#include "../../libc/stdio.h"
#include "../../libc/string.h"
#include "../../net/netdev.h"

/* Intel E1000 Registers */
#define E1000_CTRL     0x0000
#define E1000_STATUS   0x0008
#define E1000_EEPROM   0x0014
#define E1000_ICR      0x00C0
#define E1000_RCTL     0x0100
#define E1000_TCTL     0x0400
#define E1000_RDBAL    0x2800
#define E1000_TDBAL    0x3800

/* Global Device */
static net_device_t e1000_dev;
static uint32_t mmio_base = 0;

int e1000_send(net_device_t *dev, uint8_t *data, uint32_t len) {
    /* TODO: Implement TX Ring Buffer */
    /* printf("[E1000] Send %d bytes (Stub)\n", len); */
    if (dev) dev->tx_bytes += len;
    return len;
}

void e1000_init(void) {
    uint8_t bus, slot, func;
    /* Vendor: 8086 (Intel), Device: 100E (82540EM) */
    if (!pci_find_device(0x8086, 0x100E, &bus, &slot, &func)) {
        /* Try common QEMU variant 1000 */
        if (!pci_find_device(0x8086, 0x1000, &bus, &slot, &func)) {
             return; /* Not found */
        }
    }
    
    printf("[E1000] Found Intel Pro/1000 at %d:%d.%d\n", bus, slot, func);
    
    /* Bus Master */
    uint32_t cmd = pci_read_config(bus, slot, func, 0x04);
    if (!(cmd & 0x04)) {
        pci_write_config(bus, slot, func, 0x04, cmd | 0x04);
    }
    
    /* Get MMIO Base (BAR0) */
    uint32_t bar0 = pci_read_config(bus, slot, func, 0x10);
    mmio_base = bar0 & 0xFFFFFFF0;
    
    printf("[E1000] MMIO Base: 0x%x\n", mmio_base);
    
    /* Identify MAC from EEPROM or RAL? */
    /* For now, fake MAC or read from device if simplified */
    uint8_t mac_addr[6] = {0x52, 0x54, 0x00, 0xEE, 0x10, 0x00};
    
    /* Register NetDev */
    strcpy(e1000_dev.name, "eth1");
    e1000_dev.type = NETDEV_TYPE_ETHERNET;
    e1000_dev.state = NETDEV_STATE_UP;
    memcpy(e1000_dev.mac_, mac_addr, 6);
    e1000_dev.send = e1000_send;
    
    netdev_register(&e1000_dev);
    
    printf("[E1000] Initialized.\n");
}
