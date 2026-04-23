/* PS/2 Controller Driver */
/* PS/2 Controller Driver */
#include <stdint.h>

#ifdef _MSC_VER
#include <intrin.h>
static inline void outb(uint16_t port, uint8_t val) {
    __outbyte(port, val);
}
static inline uint8_t inb(uint16_t port) {
    return __inbyte(port);
}
#else
static inline void outb(uint16_t port, uint8_t val) {
    __asm__ volatile ( "outb %0, %1" : : "a"(val), "Nd"(port) );
}

static inline uint8_t inb(uint16_t port) {
    uint8_t ret;
    __asm__ volatile ( "inb %1, %0" : "=a"(ret) : "Nd"(port) );
    return ret;
}
#endif

// Serial kprint (using serial port 0x3F8)
static void kprint(const char *msg) {
    while (*msg) {
        // Wait for Transmit Holding Register Empty (THRE) bit 5 of LSR (port + 5)
        while (!(inb(0x3F8 + 5) & 0x20));
        outb(0x3F8, *msg++);
    }
}

static void kprint_hex(uint8_t n) {
    const char *hex = "0123456789ABCDEF";
    char buf[5] = {'0', 'x', hex[(n >> 4) & 0xF], hex[n & 0xF], 0};
    kprint(buf);
}

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

static void ps2_wait_write(void) {
    while (inb(PS2_STATUS_PORT) & PS2_STATUS_INPUT_BUFFER);
}

static void ps2_wait_read(void) {
    while (!(inb(PS2_STATUS_PORT) & PS2_STATUS_OUTPUT_BUFFER));
}

void ps2_init(void) {
    kprint("[PS2] Initializing Controller...\n");

    /* 1. Disable devices to prevent interrupts during config */
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_DISABLE_PORT1);
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_DISABLE_PORT2);

    /* 2. Flush Output Buffer (Discard any pending data) */
    while (inb(PS2_STATUS_PORT) & PS2_STATUS_OUTPUT_BUFFER) {
        inb(PS2_DATA_PORT);
    }

    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_READ_CONFIG);
    ps2_wait_read();
    uint8_t config = inb(PS2_DATA_PORT);
    
    kprint("[PS2] Config Read: ");
    kprint_hex(config);
    kprint("\n");

    /* 
       Enable Keyboard IRQ (bit 0)
       Enable Mouse IRQ (bit 1)
       Enable System Flag (bit 2) - indicates POST passed
       Enable Port 1 Clock (clear bit 4)
       Enable Port 2 Clock (clear bit 5)
       Enable Translation (bit 6) - Set 2 to Set 1 for compatibility
    */
    config |= (1 << 0) | (1 << 1) | (1 << 6);
    config &= ~((1 << 4) | (1 << 5)); 
    
    kprint("[PS2] Writing Config: ");
    kprint_hex(config);
    kprint("\n");

    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_WRITE_CONFIG);
    ps2_wait_write();
    outb(PS2_DATA_PORT, config);

    /* 4. Perform Controller Self-Test (Optional but good) */
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_CONTROLLER_TEST);
    ps2_wait_read();
    if (inb(PS2_DATA_PORT) != 0x55) {
        kprint("[PS2] Controller Self-Test FAILED.\n");
    }

    /* 5. Enable Devices */
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_ENABLE_PORT1);
    ps2_wait_write();
    outb(PS2_CMD_PORT, PS2_CMD_ENABLE_PORT2);
    
    /* 6. Enable Keyboard Scanning specifically */
    ps2_wait_write();
    outb(PS2_DATA_PORT, 0xF4); // Enable Scanning command to keyboard
    
    // Wait for ACK
    // ps2_wait_read();
    // inb(PS2_DATA_PORT);

    kprint("[PS2] Controller Configured.\n");
}
