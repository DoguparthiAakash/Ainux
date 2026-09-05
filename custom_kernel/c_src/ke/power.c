#include "power.h"
#include <stdint.h>
#include "io.h"
#include "../drivers/acpi.h"

extern void kprint(const char *msg);

/*
 * QEMU/Bochs Shutdown
 * QEMU supports a debug port for shutdown.
 * - Old QEMU/Bochs: 0xB004 (write 0x2000)
 * - Newer QEMU: 0x604 (write 0x2000)
 */
void sys_shutdown(void) {
    /* Try ACPI Shutdown (Real Hardware) */
    acpi_power_off();

    /* Try newer QEMU q35/acpi */
    outw(0x604, 0x2000);
    
    /* Try older QEMU/Bochs */
    outw(0xB004, 0x2000);
    
    /* VirtualBox Shutdown */
    outw(0x4004, 0x3400); 

    /* Fallback: Halt Loop */
    kprint("System Halted. It is safe to turn off your computer.\n");
    for(;;) {
        __asm__ volatile ("cli; hlt");
    }
}

/*
 * Reboot via PS/2 Keyboard Controller (8042)
 * Sending 0xFE to port 0x64 causes a CPU reset.
 */
void sys_reboot(void) {
    uint8_t good = 0x02;
    while (good & 0x02)
        good = inb(0x64);
    outb(0x64, 0xFE);
    
    /* Fallback: Triple Fault (LIDT with 0 limit) */
    /* __asm__ volatile ("lidt 0; int3"); */

    for(;;) {
        __asm__ volatile ("cli; hlt");
    }
}
