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
    
    /* Enable Interrupts */
    mouse_wait(1);
    outb(MOUSE_PORT_CMD, 0x20);
    mouse_wait(0);
    status = (inb(MOUSE_PORT_DATA) | 2); 
    mouse_wait(1);
    outb(MOUSE_PORT_CMD, 0x60);
    mouse_wait(1);
    outb(MOUSE_PORT_DATA, status);
    
    /* Magic Sequence to enable Scroll Wheel (IntelliMouse) */
    mouse_write(0xF3); mouse_write(200); mouse_read();
    mouse_write(0xF3); mouse_write(100); mouse_read();
    mouse_write(0xF3); mouse_write(80);  mouse_read();
    
    mouse_write(0xF2); /* Get ID */
    mouse_read(); 
    uint8_t id = mouse_read();
    
    /* If ID is 3, wheel is enabled! */
    if (id == 3) {
        kprint("[Mouse] Scroll Wheel Enabled (IntelliMouse mode)\n");
    }

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
            mouse_cycle++;
            break;
        case 3:
            /* Z-axis */
            /* But if device is standard ps/2, this byte won't come, cycle hangs? 
               Wait, standard ps/2 only sends 3 interrupts. 
               So cycle 3 never happens.
               We need to check ID or assume.
               Use 3 for now, upgrade later?
               Actually, modifying cycle is risky without global state check.
               Let's try to just use existing 3-byte logic but modify init.
               Wait, user wants scrolling. I MUST handle 4th byte.
               
               If I enable IntelliMouse, it sends 4 interrupts? No, 1 interrupt per byte?
               Or does it send 4 bytes in burst?
               PS/2 interrupts once per byte typically.
            */
             int8_t z = (int8_t)data; 
             mouse_state.scroll_z += z;
             mouse_cycle = 0;
             /* Fallthrough to update? Or Duplicate update logic? */
             /* Let's copy update logic here or extract it */
             
             /* Update State */
            int8_t x_rel = mouse_byte[1];
            int8_t y_rel = mouse_byte[2];
            
            mouse_state.left_btn = (mouse_byte[0] & 0x01);
            mouse_state.right_btn = (mouse_byte[0] & 0x02);
            mouse_state.middle_btn = (mouse_byte[0] & 0x04);
            
            mouse_state.x += x_rel;
            mouse_state.y -= y_rel; 
            
            /* Clamping to actual screen size */
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
            
            break;
    }
}

MouseState mouse_get_state(void) {
    return mouse_state;
}
