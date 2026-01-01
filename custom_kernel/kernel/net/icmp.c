#include "net.h"
#include "icmp.h"
#include "ip.h"
#include "../libc/string.h"
#include "../libc/stdio.h"

typedef struct {
    uint8_t type;
    uint8_t code;
    uint16_t checksum;
    uint16_t id;
    uint16_t seq;
} __attribute__((packed)) icmp_header_t;

/* Checksum - same as IP, reusable? Copying for now to avoid dependency circularity issues */
static uint16_t icmp_checksum(void *data, uint32_t len) {
    uint32_t sum = 0;
    uint16_t *p = (uint16_t*)data;
    while (len > 1) { sum += *p++; len -= 2; }
    if (len) sum += *(uint8_t*)p;
    while (sum >> 16) sum = (sum & 0xFFFF) + (sum >> 16);
    return ~sum;
}

void icmp_send_echo(uint32_t dst_ip, uint16_t id, uint16_t seq) {
    icmp_header_t icmp;
    icmp.type = 8; /* Echo Request */
    icmp.code = 0;
    icmp.id = htons(id);
    icmp.seq = htons(seq);
    icmp.checksum = 0;
    icmp.checksum = icmp_checksum(&icmp, sizeof(icmp_header_t));
    
    ip_send(dst_ip, IP_PROTO_ICMP, (uint8_t*)&icmp, sizeof(icmp_header_t));
    printf("[ICMP] Ping sent to %x\n", dst_ip);
}

void icmp_handle_packet(uint8_t *packet, uint32_t len, uint32_t src_ip) {
    if (len < sizeof(icmp_header_t)) return;
    icmp_header_t *icmp = (icmp_header_t*)packet;
    
    if (icmp->type == 0) {
        /* Echo Reply */
        printf("[ICMP] Reply from %x: seq=%d ttl=? time=?\n", src_ip, ntohs(icmp->seq));
    } else if (icmp->type == 8) {
        /* Echo Request - We should Reply */
        printf("[ICMP] Received Ping from %x\n", src_ip);
        /* TODO: Send Reply */
    }
}
