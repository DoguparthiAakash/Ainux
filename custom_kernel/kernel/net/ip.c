#include "net.h"
#include "ip.h"
#include "ethernet.h"
#include "arp.h"
#include "icmp.h"
#include "../libc/string.h"
#include "../libc/stdio.h"

typedef struct {
    uint8_t  ihl : 4;
    uint8_t  version : 4;
    uint8_t  tos;            /* Type of Service */
    uint16_t len;            /* Total Length */
    uint16_t id;             /* Identification */
    uint16_t frag_offset;    /* Flags & Fragment Offset */
    uint8_t  ttl;            /* Time To Live */
    uint8_t  proto;          /* Protocol */
    uint16_t checksum;       /* Header Checksum */
    uint32_t src_ip;
    uint32_t dst_ip;
} __attribute__((packed)) ip_header_t;

/* Checksum Helper */
static uint16_t checksum(void *data, uint32_t len) {
    uint32_t sum = 0;
    uint16_t *p = (uint16_t*)data;
    
    while (len > 1) {
        sum += *p++;
        len -= 2;
    }
    if (len) sum += *(uint8_t*)p;
    
    while (sum >> 16) sum = (sum & 0xFFFF) + (sum >> 16);
    
    return ~sum;
}

/* Hardcoded IP Config for now */
static uint32_t my_ip = 0x0A00020F; /* 10.0.2.15 */

void ip_send(uint32_t dst_ip, uint8_t protocol, uint8_t *data, uint32_t len) {
    uint8_t buffer[1500];
    ip_header_t *ip = (ip_header_t*)buffer;
    
    memset(ip, 0, sizeof(ip_header_t));
    ip->version = 4;
    ip->ihl = 5;
    ip->tos = 0;
    ip->len = htons(sizeof(ip_header_t) + len);
    ip->id = htons(0);
    ip->frag_offset = htons(0x4000); /* Don't Fragment */
    ip->ttl = 64;
    ip->proto = protocol;
    ip->src_ip = htonl(my_ip);
    ip->dst_ip = htonl(dst_ip);
    
    ip->checksum = checksum(ip, sizeof(ip_header_t));
    
    memcpy(buffer + sizeof(ip_header_t), data, len);
    
    /* Need MAC to send via Ethernet. */
    /* ARP Interaction! */
    /* For now, let's assume we know the Gateway MAC or just Broadcast? */
    /* Ideally we check ARP Cache. */
    /* If we just use Broadcast for everything, it's inefficient but works for simple demo. */
    
    /* BUT `ethernet_send` takes a MAC. */
    /* We need ARP resolution. */
    /* Let's fake it: If dst_ip == Gateway, use Cached Gateway MAC? */
    /* Or just Broadcast IP packets? Most switches flood them, but routers might drop. */
    
    /* Lets use a global "last known mac" for now from ARP? */
    /* Or just broadcast ff:ff:ff:ff:ff:ff and hope for the best? */
    /* QEMU User Mode Net might reject Broadcast IP unicasts? */
    
    uint8_t mac[6] = {0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF};
    
    ethernet_send(mac, ETHERTYPE_IP, buffer, sizeof(ip_header_t) + len);
}

void ip_handle_packet(uint8_t *packet, uint32_t len) {
    if (len < sizeof(ip_header_t)) return;
    ip_header_t *ip = (ip_header_t*)packet;
    
    if (ip->version != 4) return;
    
    /* Check Dest IP (skip for promiscuous) */
    /* if (ntohl(ip->dst_ip) != my_ip) return; */
    
    uint32_t header_len = ip->ihl * 4;
    uint8_t *payload = packet + header_len;
    uint32_t payload_len = ntohs(ip->len) - header_len;
    
    if (ip->proto == IP_PROTO_ICMP) {
         icmp_handle_packet(payload, payload_len, ntohl(ip->src_ip));
         /* printf("[IP] ICMP Packet!\n"); */
    } else {
         /* printf("[IP] Proto %d\n", ip->proto); */
    }
}
