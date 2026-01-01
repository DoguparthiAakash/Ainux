#ifndef ETHERNET_H
#define ETHERNET_H

#include <stdint.h>

void ethernet_send(uint8_t *dest, uint16_t type, uint8_t *data, uint32_t len);
void ethernet_handle_packet(uint8_t *packet, uint32_t len);

#endif
