#include "timer.h"
#include "../io.h"
#include "../log.h"

// PIT I/O Ports
#define PIT_CMD 0x43
#define PIT_CH0 0x40
#define PIT_CH1 0x41
#define PIT_CH2 0x42

static uint64_t ticks = 0;

void timer_init(uint32_t frequency) {
    // The value we send to the PIT is the value to divide it's input clock
    // (1193180 Hz) by, to get our required frequency.
    uint32_t divisor = 1193180 / frequency;

    // Send the command byte.
    outb(PIT_CMD, 0x36);

    // Divisor has to be sent byte-wise, so split here into upper/lower bytes.
    uint8_t l = (uint8_t)(divisor & 0xFF);
    uint8_t h = (uint8_t)( (divisor>>8) & 0xFF );

    // Send the frequency divisor.
    outb(PIT_CH0, l);
    outb(PIT_CH0, h);

    kprint("PIT initialized.\n");
}

uint64_t timer_get_ticks(void) {
    return ticks;
}

// Function to call from the interrupt handler
void timer_handler_callback(void) {
    ticks++;
}
