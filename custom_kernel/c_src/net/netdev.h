#ifndef NETDEV_H
#define NETDEV_H

#include <stdint.h>

#define NETDEV_TYPE_ETHERNET 1
#define NETDEV_TYPE_WIFI     2

#define NETDEV_STATE_DOWN    0
#define NETDEV_STATE_UP      1
#define NETDEV_STATE_CONNECTED 2

typedef struct net_device {
    char name[32];
    uint8_t mac_[6];
    uint32_t ip_addr;
    uint32_t gateway;
    uint32_t mask;
    int type;
    int state;
    
    /* Stats */
    uint32_t rx_bytes;
    uint32_t tx_bytes;
    
    /* Driver Ops */
    int (*send)(struct net_device *dev, uint8_t *data, uint32_t len);
    int (*poll)(struct net_device *dev);
    
    /* Wi-Fi Specific Ops */
    int (*scan)(struct net_device *dev); /* Returns count of networks found */
    int (*connect)(struct net_device *dev, char *ssid, char *pass);
    int (*disconnect)(struct net_device *dev);
    
    struct net_device *next;
} net_device_t;

void netdev_register(net_device_t *dev);
net_device_t* netdev_get_by_name(char *name);
net_device_t* netdev_get_default(void);
void netdev_iter(void (*cb)(net_device_t *dev));

#endif
