#include "idt.h"
#include <stddef.h>
#include "gfx/gfx.h" /* Visual Debug */
#include "sched/sched.h"
#include "drivers/gfx/wm.h"
#include "log.h"

struct idt_entry idt[IDT_ENTRIES];
struct idt_ptr idtr;

extern void kprint(const char *msg);

/* Helper: I/O Port Output */
static inline void outb(uint16_t port, uint8_t val) {
    __asm__ volatile ( "outb %0, %1" : : "a"(val), "Nd"(port) );
}

/* Helper: I/O Port Input */
static inline uint8_t inb(uint16_t port) {
    uint8_t ret;
    __asm__ volatile ( "inb %1, %0" : "=a"(ret) : "Nd"(port) );
    return ret;
}

/* Helper: Wait for I/O */
static inline void io_wait(void) {
    outb(0x80, 0);
}

/* PIC Constants */
#define PIC1		0x20		/* IO base address for master PIC */
#define PIC2		0xA0		/* IO base address for slave PIC */
#define PIC1_COMMAND	PIC1
#define PIC1_DATA	(PIC1+1)
#define PIC2_COMMAND	PIC2
#define PIC2_DATA	(PIC2+1)

#define ICW1_ICW4	0x01		/* ICW4 (not) needed */
#define ICW1_INIT	0x10		/* Initialization - required! */
#define ICW4_8086	0x01		/* 8086/88 (MCS-80/85) mode */

/* Remap PIC to avoid conflicts with CPU exceptions (0-31) */
void pic_remap(int offset1, int offset2) {
	unsigned char a1, a2;

	a1 = inb(PIC1_DATA);                        // save masks
	a2 = inb(PIC2_DATA);

	outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);  // starts the initialization sequence (in cascade mode)
	io_wait();
	outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
	io_wait();
	outb(PIC1_DATA, offset1);                 // ICW2: Master PIC vector offset
	io_wait();
	outb(PIC2_DATA, offset2);                 // ICW2: Slave PIC vector offset
	io_wait();
	outb(PIC1_DATA, 4);                       // ICW3: tell Master PIC that there is a slave PIC at IRQ2 (0000 0100)
	io_wait();
	outb(PIC2_DATA, 2);                       // ICW3: tell Slave PIC its cascade identity (0000 0010)
	io_wait();

	outb(PIC1_DATA, ICW4_8086);
	io_wait();
	outb(PIC2_DATA, ICW4_8086);
	io_wait();

	outb(PIC1_DATA, a1);   // restore saved masks.
	outb(PIC2_DATA, a2);
}

/* Send End-of-Interrupt to PIC */
void pic_eoi(unsigned char irq) {
	if(irq >= 8) outb(PIC2_COMMAND, 0x20);
	outb(PIC1_COMMAND, 0x20);
}

/* Forward declarations for drivers */
extern void keyboard_handler(void);
extern void mouse_handler(void);
extern void timer_isr_stub(void);
extern void syscall_isr_stub(void);

/* Generic exception handler */
__attribute__((interrupt))
void exception_handler(struct interrupt_frame *frame) {
    (void)frame;
    kprint("\n[CPU EXCEPTION] Halting...\n");
     __asm__ volatile ("cli; hlt");
}

/* Helper: Print Hex */
static void kprint_hex(uint64_t val) {
    char hex[] = "0123456789ABCDEF";
    char str[19]; /* 0x + 16 chars + null */
    str[0] = '0'; str[1] = 'x';
    for (int i = 0; i < 16; i++) {
        str[17-i] = hex[val & 0xF];
        val >>= 4;
    }
    str[18] = 0;
    kprint(str);
}

/* Division by Zero (Vector 0) */
__attribute__((interrupt))
void div_zero_handler(struct interrupt_frame *frame) {
    (void)frame;
    kprint("\n[INT] Division by Zero Caught!\n");
    __asm__ volatile ("cli; hlt");
}

__attribute__((interrupt))
void gpf_handler(struct interrupt_frame *frame, uint64_t error_code) {
    (void)frame;
    (void)error_code;
    
    /* Security Check for Encrypted Engine */
    struct task_struct *curr = sched_get_current();
    if (curr && curr->output_window) {
        Window *win = (Window*)curr->output_window;
        /* Wipe Window to White - "White Photo" effect */
        wm_fill_rect(win, 0, 0, win->width, win->height, 0xFFFFFF);
        kprint("\n[SECURITY] Encrypted Engine Tamper Detected. Session Wiped.\n");
        sched_exit(-1); 
        /* Unreachable */
    }

    kprint("\n[CPU EXCEPTION] GENERAL PROTECTION FAULT (13)\n");
    
    /* Check if it's a User Task */
    /* We can't rely on error_code for CPL in GPF reliably, use sched state */
    if (curr && (curr->flags & TASK_USER)) {
        kprint_color(KLOG_COLOR_RED, "[CRASH] Userspace General Protection Fault. Terminating...\n");
        sched_exit(-11); /* SIGSEGV */
    }

    kprint("[KERNEL PANIC] System Halted.\n");
    
    /* Dump Stack Trace */
    uint64_t rbp;
    __asm__ volatile("mov %%rbp, %0" : "=r"(rbp));
    
    kprint("Stack Trace:\n");
    struct stack_frame {
        struct stack_frame *next;
        uint64_t rip;
    };
    struct stack_frame *st_frame = (struct stack_frame*)rbp;
    for (int i = 0; i < 10; i++) {
        if (!st_frame || (uint64_t)st_frame < 0xFFFF800000000000) break;
        kprint("  RIP: "); kprint_hex(st_frame->rip); kprint("\n");
        st_frame = st_frame->next;
    }

    __asm__ volatile ("cli; hlt");
}

__attribute__((interrupt))
void pf_handler(struct interrupt_frame *frame, uint64_t error_code) {
    (void)frame;
    
    /* Security Check for Encrypted Engine */
    struct task_struct *curr = sched_get_current();
    if (curr && curr->output_window) {
        Window *win = (Window*)curr->output_window;
        /* Wipe Window to White - "White Photo" effect */
        wm_fill_rect(win, 0, 0, win->width, win->height, 0xFFFFFF);
        kprint("\n[SECURITY] Encrypted Engine Tamper Detected. Session Wiped.\n");
        sched_exit(-1); 
        /* Unreachable */
    }

    kprint("\n[CPU EXCEPTION] PAGE FAULT (14)\n");
    
    /* Read CR2 to find the address */
    uint64_t cr2;
    __asm__ volatile ("mov %%cr2, %0" : "=r"(cr2));
    
    kprint("Fault Address (CR2): ");
    kprint_hex(cr2);
    kprint("\nError Code: ");
    kprint_hex(error_code);
    kprint("\n");
    
    kprint("Flags: ");
    if (error_code & 1) kprint("PRESENT "); else kprint("NOT-PRESENT ");
    if (error_code & 2) kprint("WRITE "); else kprint("READ ");
    if (error_code & 4) kprint("USER "); else kprint("KERNEL ");
    if (error_code & 8) kprint("RESERVED ");
    if (error_code & 16) kprint("FETCH ");
    kprint("\n");

    /* Crash Recovery for Userspace */
    uint64_t USER_SPACE_LIMIT = 0x00007FFFFFFFFFFF;
    if ((error_code & 4) || (cr2 < USER_SPACE_LIMIT)) { 
        /* Either User Mode Fault, OR Kernel accessing User Address (Syscall Copy) */
        kprint_color(KLOG_COLOR_RED, "[CRASH] Page Fault at User Addr. Terminating Process.\n");
        sched_exit(-11); /* SIGSEGV */
        /* Unreachable */
    }

    /* Kernel Panic */
    /* Kernel Panic */
    kprint("[KERNEL PANIC] System Halted.\n");
    
    /* Dump Stack Trace */
    uint64_t rbp;
    __asm__ volatile("mov %%rbp, %0" : "=r"(rbp));
    
    kprint("Stack Trace:\n");
    struct stack_frame {
        struct stack_frame *next;
        uint64_t rip;
    };
    
    struct stack_frame *st_frame = (struct stack_frame*)rbp;
    for (int i = 0; i < 10; i++) {
        if (!st_frame) break;
        // Simple validation: must be canonical kernel address
        if ((uint64_t)st_frame < 0xFFFF800000000000) break; 
        
        kprint("  ["); kprint_hex(i); kprint("] RIP: "); kprint_hex(st_frame->rip); kprint("\n");
        
        st_frame = st_frame->next;
    }

    __asm__ volatile ("cli; hlt");
}

/* Keyboard IRQ Handler (IRQ 1 -> Vector 33) */
__attribute__((interrupt))
void irq1_handler(struct interrupt_frame *frame) {
    (void)frame;
    /* outb(0xE9, 'K'); */
    /* gfx_put_pixel_safe(0, 0, 0x00FF00); */
    keyboard_handler();
    pic_eoi(1);
}

/* Mouse IRQ Handler (IRQ 12 -> Vector 44) */
__attribute__((interrupt))
void irq12_handler(struct interrupt_frame *frame) {
    (void)frame;
    /* outb(0xE9, 'M'); */
    /* gfx_put_pixel_safe(10, 0, 0x0000FF); */
    /* mouse_handler(); */ /* Disable IRQ mouse handler too */
    pic_eoi(12);
}

void idt_set_descriptor(uint8_t vector, void *isr, uint8_t flags) {
    struct idt_entry *descriptor = &idt[vector];

    descriptor->isr_low    = (uint64_t)isr & 0xFFFF;
    descriptor->kernel_cs  = 0x08; 
    descriptor->ist        = 0;
    descriptor->attributes = flags;
    descriptor->isr_mid    = ((uint64_t)isr >> 16) & 0xFFFF;
    descriptor->isr_high   = ((uint64_t)isr >> 32) & 0xFFFFFFFF;
    descriptor->reserved   = 0;
}

void idt_init(void) {
    idtr.base = (uint64_t)&idt;
    idtr.limit = (uint16_t)(sizeof(struct idt_entry) * IDT_ENTRIES - 1);

    /* Set up Exception Handlers */
    for (int i = 0; i < 32; i++) {
        idt_set_descriptor(i, exception_handler, 0x8E);
    }
    idt_set_descriptor(0, div_zero_handler, 0x8E);
    idt_set_descriptor(13, gpf_handler, 0x8E);
    idt_set_descriptor(14, pf_handler, 0x8E);

    /* Remap PIC to 32..39 and 40..47 */
    pic_remap(32, 40);

    /* Register IRQ handlers */
    idt_set_descriptor(32, timer_isr_stub, 0x8E); /* IRQ 0: Timer */
    idt_set_descriptor(33, irq1_handler, 0x8E);   /* IRQ 1: Keyboard */
    idt_set_descriptor(44, irq12_handler, 0x8E);  /* IRQ 12: Mouse */
    
    /* Syscall Handler (0x80) */
    /* DPL 3 (User executable) - For now 0x8E is DPL 0 (Kernel), use 0xEE for DPL 3 */
    /* But since we don't have Ring 3 yet, 0x8E is fine. For "Advanced" later, we might change to 0xEE. */
    idt_set_descriptor(0x80, syscall_isr_stub, 0x8E);
    
    /* Unmask IRQ 1 (Keyboard) and IRQ 12 (Mouse) */
    /* Read OCW1 (Mask), clear bits 1 and 4 (IRQ12=bit 4 on slave? No IRQ12 is Slave IRQ4) */
    /* Master Mask */
    uint8_t mask1 = inb(PIC1_DATA);
    mask1 &= ~(1 << 0); /* Enable IRQ 0 (Timer) */
    mask1 &= ~(1 << 1); /* Enable IRQ 1 */
    mask1 &= ~(1 << 2); /* Enable IRQ 2 (Cascade for slave) */
    outb(PIC1_DATA, mask1);
    
    /* Slave Mask */
    uint8_t mask2 = inb(PIC2_DATA);
    mask2 &= ~(1 << 4); /* Enable IRQ 12 (Slave IRQ 4) */
    outb(PIC2_DATA, mask2);

    __asm__ volatile ("lidt %0" : : "m"(idtr));
    /* Interrupts are NOT enabled here. Enable them manually in kernel.c after all subsystems are ready. */
}
