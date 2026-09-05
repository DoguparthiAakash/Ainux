#include "netdev.h"
#include "../libc/string.h"
#include "../libc/stdio.h"

static net_device_t *g_netdev_list = 0;

void netdev_register(net_device_t *dev) {
    if (!dev) return;
    
    dev->next = g_netdev_list;
    g_netdev_list = dev;
    
    printf("[NetDev] Registered %s (Type %d)\n", dev->name, dev->type);
}

net_device_t* netdev_get_by_name(char *name) {
    net_device_t *cur = g_netdev_list;
    while (cur) {
        if (strcmp(cur->name, name) == 0) return cur;
        cur = cur->next;
    }
    return 0;
}

net_device_t* netdev_get_default(void) {
    /* Return first available */
    return g_netdev_list;
}

void netdev_iter(void (*cb)(net_device_t *dev)) {
    net_device_t *cur = g_netdev_list;
    while (cur) {
        cb(cur);
        cur = cur->next;
    }
}
