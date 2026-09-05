#ifndef NET_H
#define NET_H

#include <stdint.h>

/* Constants */
#define ETHERTYPE_ARP  0x0806
#define ETHERTYPE_IP   0x0800

typedef struct {
    uint8_t addr[6];
} mac_addr_t;

/* Endian Swap Helpers */
static inline uint16_t htons(uint16_t v) {
    return (v << 8) | (v >> 8);
}
static inline uint16_t ntohs(uint16_t v) {
    return (v << 8) | (v >> 8);
}
static inline uint32_t htonl(uint32_t v) {
    return ((v & 0xFF) << 24) | ((v & 0xFF00) << 8) | 
           ((v & 0xFF0000) >> 8) | ((v >> 24) & 0xFF);
}
static inline uint32_t ntohl(uint32_t v) {
    return htonl(v);
}

/* Prototypes */
void net_init(void);
void net_rx_loop(void); /* Polling loop */

#endif
