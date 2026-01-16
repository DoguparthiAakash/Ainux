#include "net.h"
#include "ethernet.h"
#include "arp.h"
#include "../libc/string.h"
#include "../libc/stdio.h"
#include "../drivers/net/rtl8139.h"

typedef struct {
    uint16_t hw_type;   /* 1 = Ethernet */
    uint16_t pro_type;  /* 0x0800 = IP */
    uint8_t  hw_len;    /* 6 */
    uint8_t  pro_len;   /* 4 */
    uint16_t opcode;    /* 1 = Request, 2 = Reply */
    uint8_t  src_mac[6];
    uint32_t src_ip;
    uint8_t  dst_mac[6];
    uint32_t dst_ip;
} __attribute__((packed)) arp_packet_t;

void arp_send_request(uint32_t dest_ip) {
    arp_packet_t arp;
    
    arp.hw_type = htons(0x0001);
    arp.pro_type = htons(0x0800);
    arp.hw_len = 6;
    arp.pro_len = 4;
    arp.opcode = htons(0x0001); /* Request */
    
    uint8_t *my_mac = rtl8139_get_mac_addr();
    memcpy(arp.src_mac, my_mac, 6);
    
    /* Fake Source IP: 10.0.2.15 (standard QEMU func) */
    /* 10.0.2.15 = 15 | 2<<8 | 0<<16 | 10<<24 (Little Endian machine?) */
    /* Network byte order required? */
    /* IP is u32. Let's use standard dot conversion helpers later. */
    /* For now hardcode: 10.0.2.15 */
    /* 15.2.0.10 in memory */
    arp.src_ip = (10) | (0 << 8) | (2 << 16) | (15 << 24); /* Standard manual LE fill? No, struct is just bytes. */
    /* Valid IP struct usually helps. */
    
    /* actually arp.src_ip is just a u32. */
    /* htonl? */
    /* 10.0.2.15 -> 0x0A00020F */
    /* htonl(0x0A00020F) */
    arp.src_ip = htonl(0x0A00020F); // 10.0.2.15
    
    /* Target */
    memset(arp.dst_mac, 0, 6); /* Unknown */
    arp.dst_ip = htonl(dest_ip);
    
    /* Broadcast MAC for Ethernet Frame */
    uint8_t broadcast[6] = {0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF};
    
    ethernet_send(broadcast, ETHERTYPE_ARP, (uint8_t*)&arp, sizeof(arp_packet_t));
    printf("[ARP] Request sent for %x\n", dest_ip);
}

void arp_handle_packet(uint8_t *data, uint32_t len) {
    if (len < sizeof(arp_packet_t)) return;
    arp_packet_t *arp = (arp_packet_t*)data;
    
    if (ntohs(arp->opcode) == 2) {
        printf("[ARP] Reply! IP %x is at MAC %02x:%02x:%02x:%02x:%02x:%02x\n",
            ntohl(arp->src_ip),
            arp->src_mac[0], arp->src_mac[1], arp->src_mac[2],
            arp->src_mac[3], arp->src_mac[4], arp->src_mac[5]);
    } else if (ntohs(arp->opcode) == 1) {
         /* ARP Request - Reply if it calls us */
         /* Implement later */
    }
}
