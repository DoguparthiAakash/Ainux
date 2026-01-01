#ifndef ARP_H
#define ARP_H

#include <stdint.h>

void arp_send_request(uint32_t dest_ip);
void arp_handle_packet(uint8_t *data, uint32_t len);

#endif
