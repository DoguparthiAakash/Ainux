#ifndef RTL8139_H
#define RTL8139_H

#include <stdint.h>

void rtl8139_init(void);
void rtl8139_send_packet(void *data, uint32_t len);
void rtl8139_poll(void);

/* Helper for Net Stack to poll/hook */
uint8_t* rtl8139_get_mac_addr(void);

#endif
