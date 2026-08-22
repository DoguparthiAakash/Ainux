#include "timer.h"
#include "../io.h"
#include "../log.h"
#include "mouse.h"
#include "gfx/wm.h"

// PIT I/O Ports
#define PIT_CMD 0x43
#define PIT_CH0 0x40
#define PIT_CH1 0x41
#define PIT_CH2 0x42

static volatile uint64_t ticks = 0;

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

/* Function to call from the interrupt handler */
void timer_handler_callback(void) {
    ticks++;
    /* Composition now handled in thread context (init.c/shell.c) to avoid ISR starvation */
}

void timer_sleep(uint32_t ms) {
    uint64_t start = ticks;
    /* 100 Hz assumed */
    uint64_t target = start + ms / 10;
    while(ticks < target) {
        __asm__ volatile ("hlt" : : : "memory");
    }
}

void timer_tone(uint32_t frequency) {
    if (frequency == 0) {
        timer_no_tone();
        return;
    }
    uint32_t divisor = 1193180 / frequency;
    outb(0x43, 0xB6);
    outb(0x42, (uint8_t)(divisor & 0xFF));
    outb(0x42, (uint8_t)((divisor >> 8) & 0xFF));
    
    uint8_t tmp = inb(0x61);
    if (tmp != (tmp | 3)) {
        outb(0x61, tmp | 3);
    }
}

void timer_no_tone(void) {
    uint8_t tmp = inb(0x61);
    outb(0x61, tmp & 0xFC);
}

void timer_beep(uint32_t freq, uint32_t ms) {
    timer_tone(freq);
    /* Busy wait for sleep if timer_sleep logic relies on interrupts that might be same */
    /* Assuming timer_sleep works */
    uint64_t start = timer_get_ticks();
    /* 1 tick = 10ms usually (100Hz). ms / 10 is ticks */
    uint64_t target = start + (ms / 10);
    while (timer_get_ticks() < target) { __asm__("hlt"); }
    
    timer_tone(0);
}
