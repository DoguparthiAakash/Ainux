#include "../pci.h"
#include "../../libc/stdio.h"
#include "../../libc/string.h"
#include "../../net/netdev.h"

static net_device_t ath_dev;

int ath_scan(net_device_t *dev) {
    printf("[ATH] Hardware Scan initiated... (No Radio Found)\n");
    return 0;
}

int ath_connect(net_device_t *dev, char *ssid, char *pass) {
    (void)pass;
    printf("[ATH] Connecting to %s... Failed (Radio Off)\n", ssid);
    return 0;
}

void wifi_ath_init(void) {
    uint8_t bus, slot, func;
    
    /* Find Atheros (0x168C) or Generic Class 0x0280 */
    /* Let's scan for Class 0x0280 manually since pci_find_device uses ID */
    /* We'll iterate manually or add pci_find_class later. */
    /* For now, check specific ID or just generic log. */
    
    /* If user wants generic detection, we iterate PCI. */
    printf("[WiFi] Scanning for Atheros adapters...\n");
    /* Stub unless we add pci_find_class. */
    /* Assuming if 0x168C exists... */
    if (pci_find_device(0x168C, 0x0013, &bus, &slot, &func)) { /* Example AR5001 */
         printf("[ATH] Found Atheros AR5xxx at %d:%d.%d\n", bus, slot, func);
         
         strcpy(ath_dev.name, "wlan1");
         ath_dev.type = NETDEV_TYPE_WIFI;
         ath_dev.state = NETDEV_STATE_DOWN;
         ath_dev.scan = ath_scan;
         ath_dev.connect = ath_connect;
         
         netdev_register(&ath_dev);
    }
}
