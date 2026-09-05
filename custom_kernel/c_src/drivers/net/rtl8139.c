#include "rtl8139.h"
#include "drivers/pci.h" 
#include "libc/stdio.h"
#include "libc/string.h"
#include "mm/heap.h"
#include "mm/pmm.h"
#include "hal/x86_64/idt.h" 
#include "../../net/netdev.h"
#include "../../net/ethernet.h"

/* Registers */
#define RTL_REG_MAC0  0x00
#define RTL_REG_MAR0  0x08
#define RTL_REG_TSD0  0x10 /* Transmit Status of Descriptor 0 */
#define RTL_REG_TSAD0 0x20 /* Transmit Start Address of Descriptor 0 */
#define RTL_REG_RBSTART 0x30 /* RX Buffer Start */
#define RTL_REG_CMD   0x37
#define RTL_REG_IMR   0x3C
#define RTL_REG_ISR   0x3E
#define RTL_REG_RCR   0x44 /* RX Config */

#define RTL_CMD_RESET 0x10
#define RTL_CMD_RE    0x08
#define RTL_CMD_TE    0x04

/* IO Wrappers */
static inline void outb(uint16_t port, uint8_t val) {
    __asm__ volatile ( "outb %0, %1" : : "a"(val), "Nd"(port) );
}
static inline void outl(uint16_t port, uint32_t val) {
    __asm__ volatile ( "outl %0, %1" : : "a"(val), "Nd"(port) );
}
static inline void outw(uint16_t port, uint16_t val) {
    __asm__ volatile ( "outw %0, %1" : : "a"(val), "Nd"(port) );
}
static inline uint8_t inb(uint16_t port) {
    uint8_t ret;
    __asm__ volatile ( "inb %1, %0" : "=a"(ret) : "Nd"(port) );
    return ret;
}
static inline uint16_t inw(uint16_t port) {
    uint16_t ret;
    __asm__ volatile ( "inw %1, %0" : "=a"(ret) : "Nd"(port) );
    return ret;
}

/* State */
static uint16_t io_base = 0;
static uint8_t mac_addr[6];
static uint8_t *rx_buffer = NULL;
static uint8_t *tx_buffer = NULL; /* Single TX buffer for simplicity */
static int tx_cur = 0;

/* NetDev Wrapper */
static net_device_t rtl_dev;

int rtl8139_send_wrapper(net_device_t *dev, uint8_t *data, uint32_t len) {
    rtl8139_send_packet(data, len);
    if(dev) dev->tx_bytes += len;
    return len;
}

void rtl8139_init(void) {
    uint8_t bus, slot, func;
    if (!pci_find_device(0x10EC, 0x8139, &bus, &slot, &func)) {
        printf("[RTL8139] Device not found!\n");
        return;
    }
    
    printf("[RTL8139] Found at %d:%d.%d\n", bus, slot, func);
    
    /* Enable Bus Master */
    uint32_t cmd = pci_read_config(bus, slot, func, 0x04);
    if (!(cmd & 0x04)) {
        printf("[RTL8139] Enabling Bus Master...\n");
        cmd |= 0x04; /* Bus Master */
        pci_write_config(bus, slot, func, 0x04, cmd);
    }
       
    /* Read BAR0 (IO Base) */
    uint32_t bar0 = pci_read_config(bus, slot, func, 0x10);
    if (bar0 & 1) {
        io_base = bar0 & 0xFFFFFFFC;
    } else {
        printf("[RTL8139] BAR0 is not IO! Aborting.\n");
        return;
    }
    
    printf("[RTL8139] IO Base: 0x%x\n", io_base);
    
    /* Turn on (Power on) */
    outb(io_base + 0x52, 0x0);
    
    /* Software Reset */
    outb(io_base + RTL_REG_CMD, RTL_CMD_RESET);
    while ( (inb(io_base + RTL_REG_CMD) & RTL_CMD_RESET) != 0 ) {
        /* Wait */
    }
    
    /* Init Buffers */
    rx_buffer = (uint8_t*)kmalloc(8192 + 16 + 1500); /* Recommended size */
    tx_buffer = (uint8_t*)kmalloc(4096);
    
    /* Hack: Assume rx_buffer is reachable. */
    uint64_t rx_phys = (uint64_t)rx_buffer - 0xFFFF800000000000; 
    
    outl(io_base + RTL_REG_RBSTART, (uint32_t)rx_phys);
    
    /* Config Receive: Accept Broadcast (1), Match (2), Multicast (4), Physical (8), Wrap (128) */
    /* 0x0F = AB+AM+APM+AAP */
    outl(io_base + RTL_REG_RCR, 0x0F); 
    
    /* Enable RE/TE */
    outb(io_base + RTL_REG_CMD, RTL_CMD_RE | RTL_CMD_TE);
    
    /* Get MAC */
    for(int i=0; i<6; i++) {
        mac_addr[i] = inb(io_base + RTL_REG_MAC0 + i);
    }
    
    printf("[RTL8139] MAC: %02x:%02x:%02x:%02x:%02x:%02x\n",
        mac_addr[0], mac_addr[1], mac_addr[2], mac_addr[3], mac_addr[4], mac_addr[5]);
    
    /* Register NetDev */
    strcpy(rtl_dev.name, "eth0");
    rtl_dev.type = NETDEV_TYPE_ETHERNET;
    rtl_dev.state = NETDEV_STATE_UP;
    memcpy(rtl_dev.mac_, mac_addr, 6);
    rtl_dev.send = rtl8139_send_wrapper;
    /* Poll called globally, or we hook it here? For now global poll calls rtl specific */
    
    netdev_register(&rtl_dev);
        
    printf("[RTL8139] Initialized.\n");
}

uint8_t* rtl8139_get_mac_addr(void) {
    return mac_addr;
}

void rtl8139_send_packet(void *data, uint32_t len) {
    /* Copy to TX buffer (if needed, or use data if phys contiguous) */
    /* RTL8139 has 4 TX Descriptors. Simple Single-Buffer use: */
    
    uint64_t tx_phys = (uint64_t)tx_buffer - 0xFFFF800000000000;
    
    memcpy(tx_buffer, data, len);
    
    /* Write Addr */
    outl(io_base + RTL_REG_TSAD0 + (tx_cur * 4), (uint32_t)tx_phys);
    /* Write Len & Status (Trigger Send) */
    outl(io_base + RTL_REG_TSD0 + (tx_cur * 4), len);
    
    tx_cur = (tx_cur + 1) % 4;
}

/* RX State */
static int rx_curr_offset = 0;

void rtl8139_poll(void) {
    if (!rx_buffer) return;
    
    /* Check Cmd Register for Buffer Empty */
    if (inb(io_base + RTL_REG_CMD) & 1) return; /* Buffer Empty */
    
    /* We have data! */
    /* Loop until empty */
    while ((inb(io_base + RTL_REG_CMD) & 1) == 0) {
        
        uint16_t *t = (uint16_t*)(rx_buffer + rx_curr_offset);
        uint16_t status = t[0];
        uint16_t len = t[1];
        
        /* Validate Status */
        if (status & 1) {
            /* Packet OK */
            /* printf("[RTL8139] RX Packet! Len=%d Status=0x%x\n", len, status); */
            uint8_t *packet = rx_buffer + rx_curr_offset + 4;
            /* Len includes CRC (4 bytes). Real data is len - 4 */
            /* But header tells full len. */
            
            ethernet_handle_packet(packet, len - 4);
        } else {
             printf("[RTL8139] RX Error! Status=0x%x\n", status);
        }
        
        /* Update Offset */
        /* Align to 4 bytes: (len + 4 + 3) & ~3 */
        /* Logic from RTL8139 spec */
        rx_curr_offset = (rx_curr_offset + len + 4 + 3) & ~3;
        
        /* Wrap */
        if (rx_curr_offset >= 8192) rx_curr_offset -= 8192; // Simple Wrap or specialized? 
        /* RTL8139 uses strict wrapping */
        
        /* Tell card we processed it */
        outw(io_base + 0x38, rx_curr_offset - 16); // CAPR
    }
}
