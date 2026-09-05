/* PS/2 Controller Driver */
#include "ps2.h"
#include <stdint.h>
#include "../io.h"

#define PS2_DATA_PORT 0x60
#define PS2_STATUS_PORT 0x64
#define PS2_CMD_PORT 0x64

/* Status Register Bits */
#define PS2_STATUS_OUTPUT_BUFFER    1
#define PS2_STATUS_INPUT_BUFFER     2

/* Commands */
#define PS2_CMD_DISABLE_PORT1       0xAD
#define PS2_CMD_ENABLE_PORT1        0xAE
#define PS2_CMD_DISABLE_PORT2       0xA7
#define PS2_CMD_ENABLE_PORT2        0xA8
#define PS2_CMD_READ_CONFIG         0x20
#define PS2_CMD_WRITE_CONFIG        0x60
#define PS2_CMD_CONTROLLER_TEST     0xAA
#define PS2_CMD_PORT1_TEST          0xAB
#define PS2_CMD_PORT2_TEST          0xA9

extern void kprint(const char *msg);

static void ps2_wait_write(void) {
    while (inb(PS2_STATUS_PORT) & PS2_STATUS_INPUT_BUFFER);
}

static void ps2_wait_read(void) {
    while (!(inb(PS2_STATUS_PORT) & PS2_STATUS_OUTPUT_BUFFER));
}

void ps2_init(void) {
    kprint("[PS2] Initializing Controller...\n");

    /* 1. Disable devices */
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_DISABLE_PORT1);
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_DISABLE_PORT2);

    /* 2. Flush Output Buffer */
    while (inb(PS2_STATUS_PORT) & PS2_STATUS_OUTPUT_BUFFER) {
        inb(PS2_DATA_PORT);
    }

    /* 3. Set Config Byte */
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_READ_CONFIG);
    ps2_wait_read();
    uint8_t config = inb(PS2_DATA_PORT);
    
    /* Enable IRQs (bit 0=port1, bit 1=port2) and translation (bit 6) */
    config |= (1 << 0) | (1 << 1) | (1 << 6);
    
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_WRITE_CONFIG);
    ps2_wait_write();
    outb(PS2_DATA_PORT, config);

    /* 4. Enable Devices */
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_ENABLE_PORT1);
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_ENABLE_PORT2);
    
    /* 5. Reset Mouse (0xFF) to ensure it streams */
    /* This might be done in mouse.c but good to do here if needed */
    /* But we let mouse.c handle specific device logic */

    kprint("[PS2] Controller Configured.\n");
}
