#include "netdev.h"
#include "../../libc/stdio.h"
#include "../../libc/string.h"

/* Simulated Wi-Fi Driver for Demonstration */
/* Since QEMU has no Wi-Fi, we simulate a card `wlan0` to demonstrate the `internet` command */

static net_device_t sim_wifi;

int sim_wifi_scan(net_device_t *dev) {
    if(!dev) return 0;
    
    printf("\nScanning for networks...\n");
    for(volatile int i=0; i<5000000; i++); /* Simulate Delay */
    
    /* Mock Results */
    printf("1. [Home_WiFi]    Signal: 90%%  Sec: WPA2\n");
    printf("2. [Guest_Net]    Signal: 60%%  Sec: Open\n");
    printf("3. [Starbucks]    Signal: 40%%  Sec: WPA2\n");
    printf("4. [iPhone_Hot]   Signal: 85%%  Sec: WPA2\n");
    
    return 4;
}

int sim_wifi_connect(net_device_t *dev, char *ssid, char *pass) {
    printf("[Wi-Fi] Connecting to '%s'...\n", ssid);
    for(volatile int i=0; i<10000000; i++); /* Sim Delay */
    
    if (strcmp(pass, "password") == 0 || strlen(pass) > 0) {
        printf("[Wi-Fi] Authentication Successful.\n");
        printf("[Wi-Fi] Obtaining IP Address...\n");
        // dev->ip_addr = 0xC0A80105; // 192.168.1.5
        dev->state = NETDEV_STATE_CONNECTED;
        printf("[Wi-Fi] Connected.\n");
        return 1;
    } else {
        printf("[Wi-Fi] Auth Failed. Wrong Password.\n");
        return 0;
    }
}

int sim_wifi_disconnect(net_device_t *dev) {
    printf("[Wi-Fi] Disconnecting...\n");
    dev->state = NETDEV_STATE_UP;
    printf("[Wi-Fi] Disconnected.\n");
    return 1;
}

void sim_wifi_init(void) {
    strcpy(sim_wifi.name, "wlan0");
    sim_wifi.type = NETDEV_TYPE_WIFI;
    sim_wifi.state = NETDEV_STATE_UP;
    /* Synthetic MAC */
    sim_wifi.mac_[0]=0xDE; sim_wifi.mac_[1]=0xAD; sim_wifi.mac_[2]=0xBE; 
    sim_wifi.mac_[3]=0xEF; sim_wifi.mac_[4]=0x00; sim_wifi.mac_[5]=0x01;
    
    sim_wifi.scan = sim_wifi_scan;
    sim_wifi.connect = sim_wifi_connect;
    sim_wifi.disconnect = sim_wifi_disconnect;
    
    netdev_register(&sim_wifi);
}
