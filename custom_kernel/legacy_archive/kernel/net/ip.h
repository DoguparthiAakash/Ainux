#ifndef IP_H
#define IP_H

#include <stdint.h>

#define IP_PROTO_ICMP 1
#define IP_PROTO_TCP  6
#define IP_PROTO_UDP  17

void ip_send(uint32_t dst_ip, uint8_t protocol, uint8_t *data, uint32_t len);
void ip_handle_packet(uint8_t *packet, uint32_t len);

#endif
