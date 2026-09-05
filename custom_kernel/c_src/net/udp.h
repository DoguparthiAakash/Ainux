#ifndef UDP_H
#define UDP_H

#include <stdint.h>

/* UDP Port Constants */
#define UDP_PORT_DNS 53

typedef void (*udp_callback_t)(uint8_t *data, uint32_t len, uint32_t src_ip, uint16_t src_port);

void udp_init(void);
void udp_send(uint32_t dst_ip, uint16_t src_port, uint16_t dst_port, uint8_t *data, uint32_t len);
void udp_handle_packet(uint8_t *packet, uint32_t len, uint32_t src_ip);

/* Register callback for specific port */
void udp_bind(uint16_t port, udp_callback_t callback);

#endif
