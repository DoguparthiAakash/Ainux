#include <stdint.h>
#include <stddef.h>
#include <limine.h>
#include <gdt.h>
#include <idt.h>
#include "fs/initrd.h"
#include "mm/pmm.h"
#include "mm/heap.h"
#include "drivers/keyboard.h"
#include "drivers/mouse.h"
#include "drivers/ps2.h"
#include "drivers/ata.h"
#include "wm.h"
#include "shell.h"
#include "font.h"
#include "gfx.h"

/* Set the base revision to 3 */
__attribute__((used, section(".requests")))
static volatile LIMINE_BASE_REVISION(3);

/* Framebuffer request */
__attribute__((used, section(".requests")))
static volatile struct limine_framebuffer_request framebuffer_request = {
    .id = LIMINE_FRAMEBUFFER_REQUEST,
    .revision = 0
};

/* Global framebuffer pointer */
static struct limine_framebuffer *fb = NULL;
static int cursor_x = 0;
static int cursor_y = 0;

/* Render character using bitmap font */
static void draw_char(char c, int x, int y, uint32_t color) {
    if (c == '\n' || c == '\r') {
        return;
    }
    
    /* Get font data */
    int font_index = c - 32;
    if (font_index < 0 || font_index >= 95) {
        font_index = 0;
    }
    
    const uint8_t *glyph = font_8x8[font_index];
    
    /* Draw 8x8 character */
    for (int dy = 0; dy < 8; dy++) {
        uint8_t row = glyph[dy];
        for (int dx = 0; dx < 8; dx++) {
            int px = x * 8 + dx;
            int py = y * 12 + dy;
            if (px < (int)fb->width && py < (int)fb->height) {
                uint32_t *pixel = &((uint32_t *)fb->address)[py * (fb->pitch / 4) + px];
                if (row & (1 << dx)) {
                    *pixel = color;
                } else {
                    *pixel = 0;
                }
            }
        }
    }
}

#include "io.h"

#define COM1 0x3F8

static void serial_init(void) {
    outb(COM1 + 1, 0x00);    // Disable all interrupts
    outb(COM1 + 3, 0x80);    // Enable DLAB (set baud rate divisor)
    outb(COM1 + 0, 0x03);    // Set divisor to 3 (lo byte) 38400 baud
    outb(COM1 + 1, 0x00);    //                  (hi byte)
    outb(COM1 + 3, 0x03);    // 8 bits, no parity, one stop bit
    outb(COM1 + 2, 0xC7);    // Enable FIFO, clear them, with 14-byte threshold
    outb(COM1 + 4, 0x0B);    // IRQs enabled, RTS/DSR set
}

static int is_transmit_empty(void) {
    return inb(COM1 + 5) & 0x20;
}

static void serial_putc(char c) {
    while (is_transmit_empty() == 0);
    outb(COM1, c);
}

int serial_received(void) {
    return inb(COM1 + 5) & 1;
}

char serial_read(void) {
    while (serial_received() == 0);
    return inb(COM1);
}

void kprint(const char *msg) {
    const char *p = msg;
    while (*p) {
        serial_putc(*p);
        p++;
    }

    if (!fb) return;
    
    while (*msg) {
        if (*msg == '\n') {
            cursor_x = 0;
            cursor_y++;
            if (cursor_y >= (int)(fb->height / 12)) {
                /* Scrolling Logic */
                uint32_t *fb_ptr = (uint32_t *)fb->address;
                uint64_t pitch_u32 = fb->pitch / 4;
                uint64_t total_lines = fb->height;
                
                /* Shift copy */
                for (uint64_t y = 0; y < total_lines - 12; y++) {
                    for (uint64_t x = 0; x < fb->width; x++) {
                            fb_ptr[y * pitch_u32 + x] = fb_ptr[(y + 12) * pitch_u32 + x];
                    }
                }
                
                /* Clear the bottom 12 lines */
                for (uint64_t y = total_lines - 12; y < total_lines; y++) {
                    for (uint64_t x = 0; x < fb->width; x++) {
                        fb_ptr[y * pitch_u32 + x] = 0;
                    }
                }
                
                cursor_y--;
            }
        } else if (*msg == '\b') {
            /* Backspace */
            if (cursor_x > 0) {
                cursor_x--;
                draw_char(' ', cursor_x, cursor_y, 0xFFFFFF);
            }
        } else {
            draw_char(*msg, cursor_x, cursor_y, 0xFFFFFF);
            cursor_x++;
            if (cursor_x >= (int)(fb->width / 8)) {
                cursor_x = 0;
                cursor_y++;
                if (cursor_y >= (int)(fb->height / 12)) {
                    /* Scrolling Logic */
                    
                    /* 1. Move everything UP by 12 pixels */
                    uint32_t *fb_ptr = (uint32_t *)fb->address;
                    uint64_t pitch_u32 = fb->pitch / 4;
                    uint64_t total_lines = fb->height;
                    
                    /* Shift copy */
                    for (uint64_t y = 0; y < total_lines - 12; y++) {
                        /* Optimize this with a larger memcpy if we had libc, but loop is fine for now */
                        for (uint64_t x = 0; x < fb->width; x++) {
                             fb_ptr[y * pitch_u32 + x] = fb_ptr[(y + 12) * pitch_u32 + x];
                        }
                    }
                    
                    /* 2. Clear the bottom 12 lines */
                    for (uint64_t y = total_lines - 12; y < total_lines; y++) {
                        for (uint64_t x = 0; x < fb->width; x++) {
                            fb_ptr[y * pitch_u32 + x] = 0;
                        }
                    }
                    
                    cursor_y--; /* Keep cursor on the last line */
                }
            }
        }
        msg++;
    }
}

void kprint_buf(const char *buf, uint64_t len) {
    for (uint64_t i = 0; i < len; i++) {
        char str[2] = {buf[i], 0};
        kprint(str);
    }
}

/* NEW: TUI Functions */
void term_clear(void) {
    if (!fb) return;
    /* Clear entire framebuffer to black */
    for (uint64_t i = 0; i < fb->height * fb->pitch / 4; i++) {
        ((uint32_t *)fb->address)[i] = 0;
    }
    cursor_x = 0;
    cursor_y = 0;
}

void term_set_cursor(int x, int y) {
    if (!fb) return;
    /* Clamp to screen bounds */
    if (x < 0) x = 0;
    if (y < 0) y = 0;
    int max_x = (fb->width / 8);
    int max_y = (fb->height / 12);
    
    if (x >= max_x) x = max_x - 1;
    if (y >= max_y) y = max_y - 1;
    
    cursor_x = x;
    cursor_y = y;
}

void term_draw_cursor(void) {
    if (!fb) return;
    int x = cursor_x;
    int y = cursor_y;
    
    /* Draw a small rectangle at bottom of char cell (green) */
    for (int dy = 10; dy < 12; dy++) {
        for (int dx = 0; dx < 8; dx++) {
             int px = x * 8 + dx;
             int py = y * 12 + dy;
             if (px < (int)fb->width && py < (int)fb->height) {
                 uint32_t *pixel = &((uint32_t *)fb->address)[py * (fb->pitch / 4) + px];
                 *pixel = 0x00FF00; /* Green Cursor */
             }
        }
    }
}

void term_erase_cursor(void) {
    if (!fb) return;
    int x = cursor_x;
    int y = cursor_y;
    
    /* Erase the cursor (draw black) */
    for (int dy = 10; dy < 12; dy++) {
        for (int dx = 0; dx < 8; dx++) {
             int px = x * 8 + dx;
             int py = y * 12 + dy;
             if (px < (int)fb->width && py < (int)fb->height) {
                 uint32_t *pixel = &((uint32_t *)fb->address)[py * (fb->pitch / 4) + px];
                 *pixel = 0x000000; /* Black */
             }
        }
    }
}

static void hcf(void) {
    for (;;) {
        __asm__ ("hlt");
    }
}

void _start(void) {
    if (LIMINE_BASE_REVISION_SUPPORTED == false) {
        hcf();
    }

    /* Get framebuffer */
    if (framebuffer_request.response == NULL ||
        framebuffer_request.response->framebuffer_count < 1) {
        hcf();
    }

    fb = framebuffer_request.response->framebuffers[0];
    
    /* Initialize Graphics Subsystem */
    gfx_init(fb->address, fb->width, fb->height, fb->pitch);
    
    serial_init();

    /* Clear screen */
    term_clear();

    kprint("=== Ainux Kernel ===\n\n");
    
    kprint("Initializing GDT...\n");
    gdt_init();
    
    kprint("Initializing IDT...\n");
    idt_init();

    kprint("Initializing PMM...\n");
    pmm_init();

    kprint("Initializing Heap...\n");
    heap_init();

    kprint("Initializing InitRD...\n");
    if (initrd_init() != 0) {
        kprint("InitRD failed!\n");
    }

    kprint("Initializing Keyboard...\n");
    keyboard_init();

    kprint("Initializing Mouse...\n");
    mouse_init();

    kprint("Initializing ATA Disk...\n");
    ata_init();

    kprint("\nBoot complete!\n");

    /* Enter shell */
    shell_run();

    /* Should never reach here */
    hcf();
}
