#ifndef ICMP_H
#define ICMP_H

#include <stdint.h>

void icmp_send_echo(uint32_t dst_ip, uint16_t id, uint16_t seq);
void icmp_handle_packet(uint8_t *packet, uint32_t len, uint32_t src_ip);

#endif
