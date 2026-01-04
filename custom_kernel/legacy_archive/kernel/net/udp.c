#include "udp.h"
#include "ip.h"
#include "net.h"
#include "../libc/string.h"
#include "../libc/stdio.h"
#include "../ex/mm/heap.h"

typedef struct {
    uint16_t src_port;
    uint16_t dst_port;
    uint16_t length;
    uint16_t checksum;
} __attribute__((packed)) udp_header_t;

/* Definitions for Port Callbacks */
#define MAX_UDP_SOCKETS 16

typedef struct {
    uint16_t port;
    udp_callback_t callback;
    int used;
} udp_socket_t;

static udp_socket_t sockets[MAX_UDP_SOCKETS];

void udp_init(void) {
    memset(sockets, 0, sizeof(sockets));
}

void udp_bind(uint16_t port, udp_callback_t callback) {
    for (int i = 0; i < MAX_UDP_SOCKETS; i++) {
        if (!sockets[i].used) {
            sockets[i].port = port;
            sockets[i].callback = callback;
            sockets[i].used = 1;
            return;
        }
    }
    printf("[UDP] Error: No free sockets to bind port %d\n", port);
}

void udp_send(uint32_t dst_ip, uint16_t src_port, uint16_t dst_port, uint8_t *data, uint32_t len) {
    uint32_t packet_len = sizeof(udp_header_t) + len;
    uint8_t *packet = (uint8_t*)kmalloc(packet_len);
    if (!packet) return;
    
    udp_header_t *udp = (udp_header_t*)packet;
    udp->src_port = htons(src_port);
    udp->dst_port = htons(dst_port);
    udp->length = htons(packet_len);
    udp->checksum = 0; /* Optional in IPv4 */
    
    memcpy(packet + sizeof(udp_header_t), data, len);
    
    /* Send via IP (Proto 17 = UDP) */
    ip_send(dst_ip, 17, packet, packet_len);
    
    kfree(packet);
}

void udp_handle_packet(uint8_t *packet, uint32_t len, uint32_t src_ip) {
    if (len < sizeof(udp_header_t)) return;
    
    udp_header_t *udp = (udp_header_t*)packet;
    uint16_t dst_port = ntohs(udp->dst_port);
    uint16_t src_port = ntohs(udp->src_port);
    uint16_t data_len = ntohs(udp->length) - sizeof(udp_header_t);
    uint8_t *data = packet + sizeof(udp_header_t);
    
    /* Find bound socket */
    for (int i = 0; i < MAX_UDP_SOCKETS; i++) {
        /* printf("[UDP] Packet for port %d\n", dst_port); */
        if (sockets[i].used && sockets[i].port == dst_port) {
            sockets[i].callback(data, data_len, src_ip, src_port);
            return;
        }
    }
    
    /* printf("[UDP] Dropped packet for port %d (No listener)\n", dst_port); */
}
