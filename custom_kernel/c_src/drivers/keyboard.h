#ifndef KEYBOARD_H
#define KEYBOARD_H

#include <stdint.h>

/* Initialize keyboard driver */
void keyboard_init(void);

/* Get a character (blocking) */
char keyboard_getchar(void);

/* Check if key is available (non-blocking) */
int keyboard_available(void);

/* Poll hardware (manual IRQ check) */
void keyboard_poll(void);

/* Check global Control key state */
int keyboard_is_ctrl_active(void);

/* Special Keys */
#define KEY_UP    128
#define KEY_DOWN  129
#define KEY_LEFT  130
#define KEY_RIGHT 131

#endif
