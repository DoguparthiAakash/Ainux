#ifndef MOUSE_H
#define MOUSE_H

#include <stdint.h>

typedef struct {
    int x;
    int y;
    uint8_t left_btn;
    uint8_t right_btn;
    uint8_t middle_btn;
} MouseState;

void mouse_init(void);
void mouse_handler(void); /* Called by ISR */
MouseState mouse_get_state(void);

#endif
