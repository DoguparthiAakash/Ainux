#ifndef TIMER_H
#define TIMER_H

#include <stdint.h>

void timer_init(uint32_t frequency);
void timer_sleep(uint32_t ms);
uint64_t timer_get_ticks(void);
void timer_handler_callback(void);

/* Speaker */
void timer_tone(uint32_t frequency);
void timer_no_tone(void);
void timer_beep(uint32_t freq, uint32_t ms);

#endif
