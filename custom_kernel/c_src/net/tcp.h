#ifndef TCP_H
#define TCP_H

#include <stdint.h>

/* TCP Flags */
#define TCP_FLAG_FIN 0x01
#define TCP_FLAG_SYN 0x02
#define TCP_FLAG_RST 0x04
#define TCP_FLAG_PSH 0x08
#define TCP_FLAG_ACK 0x10
#define TCP_FLAG_URG 0x20
#define TCP_FLAG_ECE 0x40
#define TCP_FLAG_CWR 0x80

/* TCP Header */
struct tcp_header {
    uint16_t src_port;
    uint16_t dst_port;
    uint32_t seq;
    uint32_t ack;
    uint8_t  offset_reserved; /* Data Offset (4) + Reserved (3) + NS (1) */
    uint8_t  flags;
    uint16_t window_size;
    uint16_t checksum;
    uint16_t urgent_ptr;
} __attribute__((packed));

/* Pseudo Header for Checksum */
struct tcp_pseudo_header {
    uint32_t src_ip;
    uint32_t dst_ip;
    uint8_t  zeros;
    uint8_t  protocol;
    uint16_t tcp_len;
} __attribute__((packed));

/* TCP State Machine */
enum tcp_state {
    TCP_CLOSED,
    TCP_LISTEN,
    TCP_SYN_SENT,
    TCP_SYN_RECEIVED,
    TCP_ESTABLISHED,
    TCP_FIN_WAIT_1,
    TCP_FIN_WAIT_2,
    TCP_CLOSE_WAIT,
    TCP_IT_WAS_A_TRIUMPH, /* Closing */
    TCP_LAST_ACK,
    TCP_TIME_WAIT
};

void tcp_init(void);
void tcp_handler(void *packet, uint16_t len, uint32_t src_ip);
int tcp_connect(uint32_t dst_ip, uint16_t dst_port);
void tcp_send(uint32_t dst_ip, uint16_t dst_port, const void *data, uint16_t len);

#endif
