#include "mouse.h"
#include "gfx/gfx.h"

extern void kprint(const char *s);

/* Helper for I/O ports (replicated here or use common header later) */
static inline void outb(uint16_t port, uint8_t val) {
    __asm__ volatile ( "outb %0, %1" : : "a"(val), "Nd"(port) );
}
static inline uint8_t inb(uint16_t port) {
    uint8_t ret;
    __asm__ volatile ( "inb %1, %0" : "=a"(ret) : "Nd"(port) );
    return ret;
}

#define MOUSE_PORT_DATA    0x60
#define MOUSE_PORT_STATUS  0x64
#define MOUSE_PORT_CMD     0x64

/* Mouse State */
static MouseState mouse_state = { 400, 300, 0, 0, 0, 0 }; /* Start center-ish */
static uint8_t mouse_cycle = 0;
static int8_t mouse_byte[3];

static void mouse_wait(uint8_t type) {
    uint32_t timeout = 100000;
    if (type == 0) { /* Data */
        while (timeout--) {
            if ((inb(MOUSE_PORT_STATUS) & 1) == 1) return;
        }
    } else { /* Signal */
        while (timeout--) {
            if ((inb(MOUSE_PORT_STATUS) & 2) == 0) return;
        }
    }
}

static void mouse_write(uint8_t write) {
    mouse_wait(1);
    outb(MOUSE_PORT_CMD, 0xD4);
    mouse_wait(1);
    outb(MOUSE_PORT_DATA, write);
}

static uint8_t mouse_read(void) {
    mouse_wait(0);
    return inb(MOUSE_PORT_DATA);
}

void mouse_init(void) {
    uint8_t status;
    
    /* Enable Auxiliary Device */
    mouse_wait(1);
    outb(MOUSE_PORT_CMD, 0xA8);
    
    /* Enable Interrupts - NO, DISABLE for Polling */
    /* If we rely on 'keyboard_poll', having interrupts enabled causes a race 
       where ISR steals the byte and poll sees nothing, or vice versa (ISR blocked).
       
       Standard PS/2 Init asks to enable usage of IRQ12 (Bit 1 in Cmd), 
       but we should MASK it in the PIC if we want pure polling?
       Or easier: Don't set Bit 1 in the controller config byte.
    */
    
    mouse_wait(1);
    outb(MOUSE_PORT_CMD, 0x20); /* Read Config */
    mouse_wait(0);
    /* status = (inb(MOUSE_PORT_DATA) | 2); // Enable IRQ12 */
    status = (inb(MOUSE_PORT_DATA) & ~2); /* DISABLE IRQ12 */
    
    mouse_wait(1);
    outb(MOUSE_PORT_CMD, 0x60); /* Write Config */
    mouse_wait(1);
    outb(MOUSE_PORT_DATA, status);
    
    /* Magic Sequence to enable Scroll Wheel (IntelliMouse) - DISABLED for Stability */
    /*
    mouse_write(0xF3); mouse_write(200); mouse_read();
    mouse_write(0xF3); mouse_write(100); mouse_read();
    mouse_write(0xF3); mouse_write(80);  mouse_read();
    
    mouse_write(0xF2); 
    mouse_read(); 
    uint8_t id = mouse_read();
    if (id == 3) {
        kprint("[Mouse] Scroll Wheel Enabled (IntelliMouse mode)\n");
    }
    */

    /* Use Default Settings */
    mouse_write(0xF6);
    mouse_read(); /* Ack */
    
    /* Enable Streaming */
    mouse_write(0xF4);
    mouse_read(); /* Ack */
    
    mouse_state.x = 400;
    mouse_state.y = 300;
}

void mouse_handler(void) {
    uint8_t status = inb(MOUSE_PORT_STATUS);
    if (!(status & 0x01)) return;
    if (!(status & 0x20)) return; /* Ensure it is mouse data */
    
    uint8_t data = inb(MOUSE_PORT_DATA);
    
    /* Process Packet */
    /* IntelliMouse sends 4 bytes: Status, X, Y, Z */
    
    switch(mouse_cycle) {
        case 0:
            if ((data & 0x08) == 0) return; /* Sync bit absent? */
            mouse_byte[0] = data;
            mouse_cycle++;
            break;
        case 1:
            mouse_byte[1] = data;
            mouse_cycle++;
            break;
        case 2:
            mouse_byte[2] = data;
            
            /* Update State (Standard PS/2) */
            /* Byte 1: Y ovfl, X ovfl, Y sign, X sign, 1, Mid, Right, Left */
            /* Byte 2: X Movement */
            /* Byte 3: Y Movement */
            
            int8_t x_rel = mouse_byte[1];
            int8_t y_rel = mouse_byte[2];
            
            /* Handle sign extension manually if needed, or rely on int8_t cast */
            /* Actually, PS/2 data packet:
               Byte 0: Yovfl Xovfl Ysign Xsign 1 M R L
            */
            
            if (mouse_byte[0] & 0x40) x_rel = 0; // X Overflow?
            if (mouse_byte[0] & 0x80) y_rel = 0; // Y Overflow?
            
            /* Logic for signs is implicitly handled by int8_t cast if bits set? 
               Wait, standard packet:
               Byte 1 is just movement. If sign bit in Byte 0 is set, we must extend sign.
               But int8_t cast of Byte 1 doesn't know about Byte 0 sign bit.
               
               Correct conversion:
            */
            int x = (int)mouse_byte[1];
            int y = (int)mouse_byte[2];
            
            if (mouse_byte[0] & 0x10) x |= 0xFFFFFF00; /* X Sign */
            if (mouse_byte[0] & 0x20) y |= 0xFFFFFF00; /* Y Sign */
            
            mouse_state.left_btn = (mouse_byte[0] & 0x01);
            mouse_state.right_btn = (mouse_byte[0] & 0x02);
            mouse_state.middle_btn = (mouse_byte[0] & 0x04);
            
            mouse_state.x += x;
            mouse_state.y -= y; /* PS/2 Y is bottom-to-top? No, usually top-down but Y moves up? 
                                   Typically Y increases UP in PS/2. Screen Y increases DOWN.
                                   So we subtract Y. */
            
            /* Clamping to actual screen size */
            uint64_t w, h, p;
            void *addr;
            gfx_get_info(&w, &h, &p, &addr);
            
            int screen_w = (int)w;
            int screen_h = (int)h;
            
            if (mouse_state.x < 0) mouse_state.x = 0;
            if (mouse_state.y < 0) mouse_state.y = 0;
            if (mouse_state.x >= screen_w) mouse_state.x = screen_w - 1;
            if (mouse_state.y >= screen_h) mouse_state.y = screen_h - 1;
            
            mouse_cycle = 0;
            
            // Debug: Print dot on movement to verify driver
            // kprint(".");
            // Or fuller debug:
            /*
            char s[32];
            klog_render_hex64(mouse_state.x, s);
            kprint("M: "); kprint(s); kprint("\n");
            */
            break;
    }
}

MouseState mouse_get_state(void) {
    return mouse_state;
}
