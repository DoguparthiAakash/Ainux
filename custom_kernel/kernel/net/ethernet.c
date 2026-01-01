#include "net.h"
#include "ethernet.h"
#include "arp.h"
#include "ip.h"
#include "../drivers/net/rtl8139.h"
#include "../libc/stdio.h"
#include "../libc/string.h"

/* Headers */
typedef struct {
    uint8_t dest[6];
    uint8_t src[6];
    uint16_t type;
} __attribute__((packed)) eth_header_t;

void ethernet_send(uint8_t *dest, uint16_t type, uint8_t *data, uint32_t len) {
    uint8_t buffer[1518];
    eth_header_t *eth = (eth_header_t*)buffer;
    
    /* Fill Header */
    memcpy(eth->dest, dest, 6);
    
    uint8_t *my_mac = rtl8139_get_mac_addr();
    memcpy(eth->src, my_mac, 6);
    
    eth->type = htons(type);
    
    /* Copy Data */
    if (len > 1500) len = 1500; /* Cap MTU */
    memcpy(buffer + sizeof(eth_header_t), data, len);
    
    /* Send via Driver */
    /* printf("[ETH] Sending %d bytes to %02x:%02x...\n", len, dest[0], dest[1]); */
    rtl8139_send_packet(buffer, sizeof(eth_header_t) + len);
}

void ethernet_handle_packet(uint8_t *packet, uint32_t len) {
    if (len < sizeof(eth_header_t)) return;
    
    eth_header_t *eth = (eth_header_t*)packet;
    
    /* Check Destination (Promiscuous mode handling if needed) */
    /* For now assume driver filtered or we accept all broadcast */
    
    uint16_t type = ntohs(eth->type);
    uint8_t *payload = packet + sizeof(eth_header_t);
    uint32_t payload_len = len - sizeof(eth_header_t);
    
    /* printf("[ETH] Recv Proto 0x%04x Len %d\n", type, payload_len); */
    
    /* Dispatch */
    /* Dispatch */
    if (type == ETHERTYPE_ARP) {
        arp_handle_packet(payload, payload_len);
        /* printf("[ETH] ARP Packet Received!\n"); */
    } else if (type == ETHERTYPE_IP) {
        ip_handle_packet(payload, payload_len);
        /* printf("[ETH] IP Packet Received!\n"); */
    } else {
        /* printf("[ETH] Unknown Protocol 0x%04x\n", type); */
    }
}
