#include "keyboard.h"

extern void kprint(const char *msg);

#define KEYBOARD_DATA_PORT 0x60
#define KEYBOARD_STATUS_PORT 0x64

/* Scancodes for modifier keys */
#define KEY_LSHIFT_PRESS   0x2A
#define KEY_LSHIFT_RELEASE 0xAA
#define KEY_RSHIFT_PRESS   0x36
#define KEY_RSHIFT_RELEASE 0xB6
#define KEY_LCTRL_PRESS    0x1D
#define KEY_LCTRL_RELEASE  0x9D
#define KEY_CAPSLOCK       0x3A

/* Global state for modifiers */
static int shift_pressed = 0;
static int ctrl_pressed = 0;
static int capslock_active = 0;

/* US QWERTY Lowercase Map */
static const char scancode_map_lower[] = {
    0,  27, '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', '\b',
    '\t', 'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[', ']', '\n',
    0, /* Ctrl */
    'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '`',
    0, /* Left shift */
    '\\', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/', 
    0, /* Right shift */
    '*',
    0, /* Alt */
    ' ', /* Space */
};

/* US QWERTY Uppercase/Symbol Map */
static const char scancode_map_upper[] = {
    0,  27, '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+', '\b',
    '\t', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', '{', '}', '\n',
    0, /* Ctrl */
    'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', ':', '"', '~',
    0, /* Left shift */
    '|', 'Z', 'X', 'C', 'V', 'B', 'N', 'M', '<', '>', '?', 
    0, /* Right shift */
    '*',
    0, /* Alt */
    ' ', /* Space */
};

/* Circular buffer for keyboard input */
#define KB_BUFFER_SIZE 256
static char kb_buffer[KB_BUFFER_SIZE];
static int kb_read_pos = 0;
static int kb_write_pos = 0;

static inline uint8_t inb(uint16_t port) {
    uint8_t ret;
    __asm__ volatile ("inb %1, %0" : "=a"(ret) : "Nd"(port));
    return ret;
}

void keyboard_handler(void) {
    /* 
     * Disable ISR logic to prevent race condition with Polling.
     * We suspect interrupts are firing but maybe not clearing correctly,
     * or conflicting with the aggressive polling loop.
     * Since we are relying on polling, we ignore interrupts here.
     * The End of Interrupt (EOI) should be handled by the IDT stub.
     */
    return;
}

void keyboard_poll(void) {
    uint8_t status = inb(KEYBOARD_STATUS_PORT);
    if (status & 1) { /* Output buffer full */
        if (status & 0x20) return; /* Mouse data */
        
        static int e0_prefix = 0;
        static uint8_t last_scancode = 0; /* Dedup */

        uint8_t scancode = inb(KEYBOARD_DATA_PORT);
        
        /* Ignore if identical to last scancode (Fixes double-typing on fast poll) */
        /* Note: This kills auto-repeat for now, but stability is priority */
        if (scancode == last_scancode) {
             /* Read it to clear buffer, but don't process */
             return; 
        }
        last_scancode = scancode;
        
        /* Copy-paste of handler logic to update buffer */
        if (scancode == 0xE0) { e0_prefix = 1; return; }
        
        if (e0_prefix) {
            e0_prefix = 0;
            if (scancode & 0x80) return;
            char c = 0;
            switch (scancode) {
                case 0x48: c = KEY_UP; break;
                case 0x50: c = KEY_DOWN; break;
                case 0x4B: c = KEY_LEFT; break;
                case 0x4D: c = KEY_RIGHT; break;
            }
            if (c != 0) {
                kb_buffer[kb_write_pos] = c;
                kb_write_pos = (kb_write_pos + 1) % KB_BUFFER_SIZE;
            }
            return;
        }

        if (scancode == KEY_LSHIFT_PRESS || scancode == KEY_RSHIFT_PRESS) { shift_pressed = 1; return; }
        if (scancode == KEY_LSHIFT_RELEASE || scancode == KEY_RSHIFT_RELEASE) { shift_pressed = 0; return; }
        if (scancode == KEY_LCTRL_PRESS) { ctrl_pressed = 1; return; }
        if (scancode == KEY_LCTRL_RELEASE) { ctrl_pressed = 0; return; }
        if (scancode == KEY_CAPSLOCK) { capslock_active = !capslock_active; return; }

        if (scancode & 0x80) return;

        char c = 0;
        if (scancode < sizeof(scancode_map_lower)) {
            if (shift_pressed) {
                c = scancode_map_upper[scancode];
                if (capslock_active && c >= 'A' && c <= 'Z') c = scancode_map_lower[scancode]; 
            } else {
                c = scancode_map_lower[scancode];
                if (capslock_active && c >= 'a' && c <= 'z') c = scancode_map_upper[scancode];
            }

            if (c != 0) {
                kb_buffer[kb_write_pos] = c;
                kb_write_pos = (kb_write_pos + 1) % KB_BUFFER_SIZE;
            }
        }
    }
}

/* New function to expose Ctrl state */
int keyboard_is_ctrl_active(void) {
    return ctrl_pressed;
}

void keyboard_init(void) {
    kprint("[KB] Initialized (US Layout + Ctrl)\n");
}

char keyboard_getchar(void) {
    while (kb_read_pos == kb_write_pos) {
        // __asm__ volatile ("hlt"); /* Wait for interrupt */
        /* Since interrupts might be broken, we POLL */
        keyboard_poll();
    }
    char c = kb_buffer[kb_read_pos];
    kb_read_pos = (kb_read_pos + 1) % KB_BUFFER_SIZE;
    return c;
}

int keyboard_available(void) {
    /* Poll immediately to check for new inputs */
    keyboard_poll();
    return kb_read_pos != kb_write_pos;
}
