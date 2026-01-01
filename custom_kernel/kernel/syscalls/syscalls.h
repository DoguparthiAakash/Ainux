#ifndef SYSCALLS_H
#define SYSCALLS_H

#include <stdint.h>
#include "idt.h" // For interrupt frame

// Define syscall numbers
#define SYS_READ 0
#define SYS_WRITE 1
#define SYS_OPEN 2
#define SYS_CLOSE 3
#define SYS_LSEEK 8

#define SYS_EXIT  60
#define SYS_YIELD 158
#define SYS_SLEEP 35
#define SYS_DRAW_RECT 400 // Arbitrary choice

void syscall_handler(struct interrupt_frame *frame);

#endif
