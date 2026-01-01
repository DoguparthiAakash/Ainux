#ifndef DNS_H
#define DNS_H

#include <stdint.h>

/* Perform DNS Lookup */
/* Returns 1 if request sent, 0 on failure */
int dns_resolve(char *hostname);

void dns_init(void);

#endif
