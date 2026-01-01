#include "net.h"
#include "ethernet.h"
#include "arp.h"
#include "ip.h"
#include "netdev.h"
#include "../libc/stdio.h"
#include "../libc/string.h"

/* Headers */
typedef struct {
    uint8_t dest[6];
    uint8_t src[6];
    uint16_t type;
} __attribute__((packed)) eth_header_t;

#include "../ex/mm/heap.h"

void ethernet_send(uint8_t *dest, uint16_t type, uint8_t *data, uint32_t len) {
    net_device_t *dev = netdev_get_default();
    if (!dev) {
        printf("[ETH] Error: No Default Network Device!\n");
        return;
    }

    uint8_t *buffer = (uint8_t*)kmalloc(1518);
    if (!buffer) return;

    eth_header_t *eth = (eth_header_t*)buffer;
    
    /* Fill Header */
    memcpy(eth->dest, dest, 6);
    
    memcpy(eth->src, dev->mac_, 6);
    
    eth->type = htons(type);
    
    /* Copy Data */
    if (len > 1500) len = 1500; /* Cap MTU */
    memcpy(buffer + sizeof(eth_header_t), data, len);
    
    /* Send via Driver */
    dev->send(dev, buffer, sizeof(eth_header_t) + len);
    
    kfree(buffer);
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
