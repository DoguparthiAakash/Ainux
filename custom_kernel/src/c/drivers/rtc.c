#include "rtc.h"
#include "kernel/io.h" /* for outb, inb */

#define CMOS_ADDRESS 0x70
#define CMOS_DATA    0x71

static int bcd2bin(int bcd) {
    return ((bcd >> 4) * 10) + (bcd & 0x0F);
}

static uint8_t read_register(uint8_t reg) {
    outb(CMOS_ADDRESS, reg);
    return inb(CMOS_DATA);
}

static int get_update_in_progress_flag() {
    outb(CMOS_ADDRESS, 0x0A);
    return (inb(CMOS_DATA) & 0x80);
}

void rtc_init(void) {
    /* No init technically required for basic reading */
}

void rtc_get_time(rtc_time_t *t) {
    uint8_t last_second;
    uint8_t last_minute;
    uint8_t last_hour;
    uint8_t last_day;
    uint8_t last_month;
    uint8_t last_year;
    uint8_t century;
    uint8_t registerB;

    /* Wait for update to complete */
    while (get_update_in_progress_flag());

    t->second = read_register(0x00);
    t->minute = read_register(0x02);
    t->hour = read_register(0x04);
    t->day = read_register(0x07);
    t->month = read_register(0x08);
    t->year = read_register(0x09);
    
    /* Optional: Centure register? usually not standard. assume 20xx */
    
    do {
        last_second = t->second;
        last_minute = t->minute;
        last_hour = t->hour;
        last_day = t->day;
        last_month = t->month;
        last_year = t->year;

        while (get_update_in_progress_flag());

        t->second = read_register(0x00);
        t->minute = read_register(0x02);
        t->hour = read_register(0x04);
        t->day = read_register(0x07);
        t->month = read_register(0x08);
        t->year = read_register(0x09);
    } while ((last_second != t->second) || (last_minute != t->minute) || (last_hour != t->hour) ||
               (last_day != t->day)     || (last_month != t->month)   || (last_year != t->year));

    registerB = read_register(0x0B);

    /* Convert BCD to binary values if necessary */
    if (!(registerB & 0x04)) {
        t->second = bcd2bin(t->second);
        t->minute = bcd2bin(t->minute);
        t->hour   = bcd2bin(t->hour & 0x7F); /* Mask PM bit if present for safety before conv? */
        t->day    = bcd2bin(t->day);
        t->month  = bcd2bin(t->month);
        t->year   = bcd2bin(t->year);
    }

    /* Convert 12 hour to 24 hour if necessary */
    if (!(registerB & 0x02) && (t->hour & 0x80)) {
        t->hour = ((t->hour & 0x7F) + 12) % 24;
    }

    /* Calculate full year */
    /* Assume 20th century if > 90? Or just assume 2000s for now */
    t->year += 2000;
}
