#ifndef SYSCALLS_H
#define SYSCALLS_H

#include <stdint.h>
#include "../arch/x86_64/idt.h" // For interrupt frame

#define SYS_WRITE 1
#define SYS_EXIT  60
#define SYS_YIELD 158
#define SYS_SLEEP 35
#define SYS_DRAW_RECT 400 // Arbitrary choice

void syscall_handler(struct interrupt_frame *frame);

#endif
