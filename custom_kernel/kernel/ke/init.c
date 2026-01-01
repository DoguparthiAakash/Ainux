#include <stdint.h>
#include <stddef.h>
#include "boot_info.h"
#include "cpu.h"
#include "gdt.h"
#include "pci.h"
#include "../net/netdev.h"
#include "../net/udp.h"
#include "../net/dns.h"
#include "../net/icmp.h"
#include "idt.h"
#include "io/initrd.h"
#include "mm/pmm.h"
#include "mm/heap.h"
#include "drivers/keyboard.h"
#include "drivers/mouse.h"
#include "drivers/ps2.h"
#include "drivers/ata.h"
#include "gfx/wm.h"
#include "shell.h"
#include "gfx/font.h"
#include "gfx/gfx.h"
#include "log.h"
#include "sched/sched.h"
#include "drivers/timer.h"
#include "mm/heap.h"
#include "mm/pmm.h"
#include "mm/vmm.h"
#include "io/vfs.h"
#include "io/mbr.h"
#include "io/fat32.h"
#include "io/elf.h"
#include "io/initrd.h"
#include "libc/stdio.h"
#include "libc/string.h"
#include "drivers/acpi.h"
#include "drivers/pci.h"
#include "drivers/net/rtl8139.h"

/* Cursor State */
static int cursor_x = 0;
static int cursor_y = 0;

/* Helper to get FB from GFX module */
static int get_fb_info(uint64_t *w, uint64_t *h, uint64_t *p, void **addr) {
    gfx_get_info(w, h, p, addr);
    return (*addr != NULL);
}

/* Render character using bitmap font - Optimized to use GFX module */
static void draw_char(char c, int x, int y, uint32_t color) {
    if (c == '\n' || c == '\r') return;
    
    uint64_t w, h, p; void *addr;
    if (!get_fb_info(&w, &h, &p, &addr)) return;
    
    int font_index = c - 32;
    if (font_index < 0 || font_index >= 95) font_index = 0;
    
    const uint8_t *glyph = font_8x8[font_index];
    
    for (int dy = 0; dy < 8; dy++) {
        uint8_t row = glyph[dy];
        for (int dx = 0; dx < 8; dx++) {
            int px = x * 8 + dx;
            int py = y * 12 + dy;
            if (px < (int)w && py < (int)h) {
                uint32_t *pixel = &((uint32_t *)addr)[py * (p / 4) + px];
                if (row & (1 << dx)) *pixel = color;
                else *pixel = 0;
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

/* Output a character to serial port */
void serial_putc(char c) {
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

#include "log.h"

void background_task(void) {
    while (1) {
        /* Busy wait / Yield */
        /* Poll Network */
        rtl8139_poll();
        
        for (volatile int i = 0; i < 100000; i++);
        // kprint("."); /* Silenced for usability */
    }
}

/* Terminal Buffer */
#define MAX_TERM_W 256
#define MAX_TERM_H 128

static char term_buffer[MAX_TERM_H][MAX_TERM_W];
static int term_cols = 0;
static int term_rows = 0;

void term_clear(void);

/* Helper: Move cursor and handle scrolling using Text Buffer */
/* Helper: Move cursor and handle scrolling using Text Buffer */
static void term_scroll_text(void) {
    /* Pixel Scroll: Move Framebuffer UP by 12 pixels */
    /* Preserves color attributes intrinsically */
    
    uint64_t w, h, pitch; void *addr;
    if (!get_fb_info(&w, &h, &pitch, &addr)) return;
    
    uint8_t *fb = (uint8_t*)addr;
    uint32_t line_h = 12; // Font height hardcoded
    uint64_t bytes_per_row = pitch * line_h;
    uint64_t total_bytes = pitch * h;
    
    /* Move (Line 1 to End) -> (Line 0) */
    /* memcpy handles overlapping? No, need memmove for safer, but here we copy UP so src > dst. */
    /* src=row1, dst=row0. src always ahead. standard memcpy (forward copy) is safe if src > dst? */
    /* Wait, standard memcpy behavior is undefined for overlap. memmove is safer. */
    /* Does our kernel have memmove? standard libc/string.c usually implements it. */
    /* Checking kprint dependencies: init.c includes "libc/string.h". */

    /* Let's assume standard memcpy is safe if we implemented it simply (forward). */
    /* But safer to use manual loop for large chunks if unsure about optimized memcpy overlap. */
    
    /* Dest: fb */
    /* Src: fb + bytes_per_row */
    /* Size: total_bytes - bytes_per_row */
    
    memcpy(fb, fb + bytes_per_row, total_bytes - bytes_per_row);
    
    /* Clear Last Row */
    /* memset (fb + (h - line_h)*pitch, 0, bytes_per_row) */
    /* But wait, 'h' might not be a multiple of 12. */
    /* The 'term_rows' is used for text limits. */
    /* We should clear the area corresponding to the last text row. */
    /* Last text row Y = (term_rows - 1) * 12 */
    
    uint64_t last_row_offset = (term_rows - 1) * 12 * pitch;
    /* Clear 12 lines */
    for(uint32_t i=0; i<12; i++) {
         memset(fb + last_row_offset + (i*pitch), 0, w*4); // Assuming 32bpp, w*4 is safe line width? 
         // Actually use pitch for robust clearing? 
         // pitch is bytes per line.
         memset(fb + last_row_offset + (i*pitch), 0, pitch);
    }

    /* Update Text Buffer Logic (Keep history valid for logical operations if needed, but not for display) */
    for (int y = 0; y < term_rows - 1; y++) {
        memcpy(term_buffer[y], term_buffer[y+1], term_cols);
    }
    memset(term_buffer[term_rows - 1], 0, term_cols);
}

/* ANSI Color Parser State */
static uint32_t term_color = 0xFFFFFF;

void kprint(const char *msg) {
    klog_write(msg); /* Hook for dmesg */
    
    const char *p = msg;
    while (*p) {
        serial_putc(*p);
        p++;
    }

    uint64_t w, h, pitch; void *addr;
    if (!get_fb_info(&w, &h, &pitch, &addr)) return;
    
    /* Init dims if needed */
    if (term_cols == 0) {
        term_cols = w / 8;
        term_rows = h / 12;
        if (term_cols > MAX_TERM_W) term_cols = MAX_TERM_W;
        if (term_rows > MAX_TERM_H) term_rows = MAX_TERM_H;
    }
    
    while (*msg) {
        if (*msg == '\033') {
            /* ANSI Escape Sequence */
            msg++;
            if (*msg == '[') {
                msg++;
                
                /* Parse Simple Color Codes */
                /* 1;31m Red, 1;32m Green, etc. */
                /* 0m Reset */
                
                /* Helper to check prefix */
                int match = 0;
                
                // Red
                if (msg[0] == '1' && msg[1] == ';' && msg[2] == '3' && msg[3] == '1' && msg[4] == 'm') {
                    term_color = 0xFF3333; // Red
                    msg += 5; match = 1;
                }
                // Green
                else if (msg[0] == '1' && msg[1] == ';' && msg[2] == '3' && msg[3] == '2' && msg[4] == 'm') {
                    term_color = 0x33FF33; // Green
                    msg += 5; match = 1;
                }
                // Yellow
                else if (msg[0] == '1' && msg[1] == ';' && msg[2] == '3' && msg[3] == '3' && msg[4] == 'm') {
                    term_color = 0xFFFF33; // Yellow
                    msg += 5; match = 1;
                }
                // Blue
                else if (msg[0] == '1' && msg[1] == ';' && msg[2] == '3' && msg[3] == '4' && msg[4] == 'm') {
                    term_color = 0x3388FF; // Blue
                    msg += 5; match = 1;
                }
                // Cyan
                else if (msg[0] == '1' && msg[1] == ';' && msg[2] == '3' && msg[3] == '6' && msg[4] == 'm') {
                    term_color = 0x33FFFF; // Cyan
                    msg += 5; match = 1;
                }
                // Reset
                else if (msg[0] == '0' && msg[1] == 'm') {
                    term_color = 0xFFFFFF; // White
                    msg += 2; match = 1;
                }
                
                if (!match) {
                    // Skip until 'm' or end
                    while (*msg && *msg != 'm') msg++;
                    if (*msg == 'm') msg++;
                }
                continue;
            }
        }
    
        if (*msg == '\n') {
            cursor_x = 0;
            cursor_y++;
            if (cursor_y >= term_rows) {
                cursor_y = term_rows - 1;
                term_scroll_text();
            }
        } else if (*msg == '\b') {
            /* Backspace */
            if (cursor_x > 0) {
                cursor_x--;
                term_buffer[cursor_y][cursor_x] = ' '; /* Update buffer */
                draw_char(' ', cursor_x, cursor_y, term_color);
            }
        } else {
            /* Normal char */
            if (cursor_x < term_cols && cursor_y < term_rows) {
                term_buffer[cursor_y][cursor_x] = *msg;
                draw_char(*msg, cursor_x, cursor_y, term_color);
                cursor_x++;
            }
            
            if (cursor_x >= term_cols) {
                cursor_x = 0;
                cursor_y++;
                if (cursor_y >= term_rows) {
                    cursor_y = term_rows - 1;
                    term_scroll_text();
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
    uint64_t w, h, p; void *addr;
    if (!get_fb_info(&w, &h, &p, &addr)) return;
    
    /* Clear entire framebuffer to black */
    for (uint64_t i = 0; i < h * p / 4; i++) {
        ((uint32_t *)addr)[i] = 0;
    }
    cursor_x = 0;
    cursor_y = 0;
    
    /* Clear Buffer */
    for(int i=0; i<MAX_TERM_H; i++) memset(term_buffer[i], 0, MAX_TERM_W);
}

void term_set_cursor(int x, int y) {
    uint64_t w, h, p; void *addr;
    if (!get_fb_info(&w, &h, &p, &addr)) return;

    /* Clamp to screen bounds */
    if (x < 0) x = 0;
    if (y < 0) y = 0;
    int max_x = (w / 8);
    int max_y = (h / 12);
    
    if (x >= max_x) x = max_x - 1;
    if (y >= max_y) y = max_y - 1;
    
    cursor_x = x;
    cursor_y = y;
}

void term_draw_cursor(void) {
    uint64_t w, h, p; void *addr;
    if (!get_fb_info(&w, &h, &p, &addr)) return;

    int x = cursor_x;
    int y = cursor_y;
    
    /* Draw a small rectangle at bottom of char cell (green) */
    for (int dy = 10; dy < 12; dy++) {
        for (int dx = 0; dx < 8; dx++) {
             int px = x * 8 + dx;
             int py = y * 12 + dy;
             if (px < (int)w && py < (int)h) {
                 uint32_t *pixel = &((uint32_t *)addr)[py * (p / 4) + px];
                 *pixel = 0x00FF00; /* Green Cursor */
             }
        }
    }
}

void term_erase_cursor(void) {
    uint64_t w, h, p; void *addr;
    if (!get_fb_info(&w, &h, &p, &addr)) return;

    int x = cursor_x;
    int y = cursor_y;
    
    /* Erase the cursor (draw black) */
    for (int dy = 10; dy < 12; dy++) {
        for (int dx = 0; dx < 8; dx++) {
             int px = x * 8 + dx;
             int py = y * 12 + dy;
             if (px < (int)w && py < (int)h) {
                 uint32_t *pixel = &((uint32_t *)addr)[py * (p / 4) + px];
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


#include "limine.h"

/* Limine framebuffer request */
static volatile struct limine_framebuffer_request framebuffer_request = {
    .id = LIMINE_FRAMEBUFFER_REQUEST,
    .revision = 0,
    .response = NULL
};

/* Limine Requests */
static volatile struct limine_memmap_request memmap_request = {
    .id = LIMINE_MEMMAP_REQUEST,
    .revision = 0
};

static volatile struct limine_module_request module_request = {
    .id = LIMINE_MODULE_REQUEST,
    .revision = 0
};

static volatile struct limine_hhdm_request hhdm_request = {
    .id = LIMINE_HHDM_REQUEST,
    .revision = 0
};

static volatile struct limine_rsdp_request rsdp_request = {
    .id = LIMINE_RSDP_REQUEST,
    .revision = 0
};


/* Limine base revision */
LIMINE_BASE_REVISION(2);

/* Local storage for translated boot info */
static struct boot_info limine_info_store;
static struct multiboot_mmap_entry limine_mmap_buffer[128];
uint64_t g_hhdm_offset = 0;

/* Early serial debug - before serial_init for debugging boot issues */
static void early_serial_putc(char c) {
    while ((inb(COM1 + 5) & 0x20) == 0);
    outb(COM1, c);
}

void early_serial_puts(const char *s) {
    while (*s) serial_putc(*s++);
}

void early_serial_hex64(uint64_t val) {
    char hex[17];
    for (int i = 15; i >= 0; i--) {
        int n = (val >> (i * 4)) & 0xF;
        hex[15 - i] = n < 10 ? '0' + n : 'A' + n - 10;
    }
    hex[16] = 0;
    early_serial_puts(hex);
}

void kmain(struct boot_info *info) {
    /* Initialize serial port early for debugging */
    serial_init();
    
    /* Check for Limine Boot */
    if (info == NULL) {
        early_serial_puts("[Limine] Detected Native Boot. Converting info...\n");
        
        /* 1. Framebuffer */
        if (framebuffer_request.response && framebuffer_request.response->framebuffer_count > 0) {
             struct limine_framebuffer *fb = framebuffer_request.response->framebuffers[0];
             limine_info_store.framebuffer_addr = (uint64_t)fb->address;
             limine_info_store.framebuffer_width = fb->width;
             limine_info_store.framebuffer_height = fb->height;
             limine_info_store.framebuffer_pitch = fb->pitch;
             limine_info_store.framebuffer_bpp = fb->bpp;
             early_serial_puts("[Limine] Framebuffer found.\n");
        } else {
             early_serial_puts("[Limine] No Framebuffer.\n");
        }
        
        /* 2. Memory Map */
        if (memmap_request.response) {
            uint64_t count = memmap_request.response->entry_count;
            if (count > 128) count = 128; /* Cap to buffer size */
            
            struct limine_memmap_entry **entries = memmap_request.response->entries;
            
            for (uint64_t i = 0; i < count; i++) {
                struct limine_memmap_entry *entry = entries[i];
                limine_mmap_buffer[i].addr = entry->base;
                limine_mmap_buffer[i].len = entry->length;
                /* Map Limine type to MB2 type (Limine + 1 matches standard types) */
                limine_mmap_buffer[i].type = entry->type + 1; 
                limine_mmap_buffer[i].zero = 0;
            }
            
            limine_info_store.memory_map_addr = (uint64_t)limine_mmap_buffer;
            limine_info_store.memory_map_entries = count;
            early_serial_puts("[Limine] Memory Map converted.\n");
        }
        
        /* 3. Modules (InitRD) */
        if (module_request.response && module_request.response->module_count > 0) {
             struct limine_file *mod = module_request.response->modules[0];
             limine_info_store.initrd_addr = (uint64_t)mod->address;
             limine_info_store.initrd_size = mod->size;
             early_serial_puts("[Limine] InitRD module found.\n");
        }
        
        /* 3.1 RSDP */
        if (rsdp_request.response) {
            limine_info_store.rsdp = (uint64_t)rsdp_request.response->address;
            early_serial_puts("[Limine] RSDP found.\n");
        }
        
    /* 4. HHDM (Higher Half Direct Map) */
        if (hhdm_request.response) {
            g_hhdm_offset = hhdm_request.response->offset;
            early_serial_puts("[Limine] HHDM Offset: 0x");
            early_serial_hex64(g_hhdm_offset);
            early_serial_puts("\n");
        } else {
             /* Fallback to default if not provided (shouldn't happen with Limine) */
             g_hhdm_offset = 0xFFFF800000000000ULL; 
             early_serial_puts("[Limine] HHDM Defaulting.\n");
        }
        
        info = &limine_info_store;
    }
    outb(COM1 + 3, 0x80);
    outb(COM1 + 0, 0x03);
    outb(COM1 + 1, 0x00);
    outb(COM1 + 3, 0x03);
    outb(COM1 + 2, 0xC7);
    outb(COM1 + 4, 0x0B);
    
    early_serial_puts("[DBG] boot_info ptr: 0x");
    early_serial_hex64((uint64_t)info);
    early_serial_puts("\n");
    
    early_serial_puts("[DBG] fb_addr: 0x");
    early_serial_hex64(info->framebuffer_addr);
    early_serial_puts(" w:");
    early_serial_hex64(info->framebuffer_width);
    early_serial_puts(" h:");
    early_serial_hex64(info->framebuffer_height);
    early_serial_puts(" p:");
    early_serial_hex64(info->framebuffer_pitch);
    early_serial_puts("\n");
    
    serial_init();
    klog_init();
    
    kprint_color(KLOG_COLOR_CYAN, "=== Ainux Kernel ===\n");
    kprint_color(KLOG_COLOR_YELLOW, "Booting...\n");

    /* Initialize CPU Support (Intel/AMD) */
    cpu_init();
    
    
    kprint_color(KLOG_COLOR_GREEN, "Initializing GDT...\n");
    gdt_init();
    
    kprint("Initializing IDT...\n");
    idt_init();

    kprint("Initializing PMM...\n");
    pmm_init(info);

    kprint("Initializing Heap...\n");
    heap_init(g_hhdm_offset);

    kprint("Initializing VMM...\n");
    vmm_init();
    
    kprint("Initializing VFS...\n");
    vfs_init();

    kprint("Initializing Scheduler...\n");
    sched_init();

    kprint("Initializing Timer...\n");
    timer_init(100);

    kprint("Starting Multitasking (creating background task)...\n");
    sched_create_task(background_task);
    
    /* Initialize ACPI (if RSDP found) */
    acpi_init(info->rsdp);

    /* Initialize GUI if available (moved after Heap/VMM) */
    if (info->framebuffer_addr != 0) {
        /* Update legacy fb struct for draw_char compatibility - REMOVED */
        /* fb->address = info->framebuffer_addr; */
        /* ... handled by gfx module internally now ... */

        kprint("Initializing Graphics...\n");
        gfx_init((void*)info->framebuffer_addr, info->framebuffer_width, info->framebuffer_height, info->framebuffer_pitch);
        
        kprint("Initializing Window Manager...\n");
        wm_init();
        
        term_clear();
    }

    kprint("Initializing InitRD...\n");
    if (info->initrd_addr != 0) {
         initrd_init_memory(info->initrd_addr, info->initrd_size);
         vfs_mount("/initrd", initrd_mount_vfs());
         kprint("VFS: Mounted InitRD at /initrd\n");
         
         kprint("--- InitRD Contents ---\n");
         initrd_list_files("/");
         kprint("-----------------------\n");
         
             /* Spawn Userspace Shell - DISABLED for now to fix Login/Panic conflict */
             /*
             elf_load_result_t res;
             if (elf_load_file("/initrd/shell.elf", &res) == 0) {
                 sched_create_user_task(res.entry_point, res.stack_top, res.address_space);
                 kprint_color(KLOG_COLOR_GREEN, "Spawned Userspace Shell (PID 1)!\n");
             } else {
                 kprint_color(KLOG_COLOR_RED, "Failed to spawn Userspace Shell!\n");
             }
             */
         
    } else {
        kprint_color(KLOG_COLOR_RED, "InitRD not found!\n");
    }

    kprint("Initializing Keyboard...\n");
    keyboard_init();

    kprint("Initializing Mouse...\n");
    mouse_init();

    kprint("Initializing ATA Disk...\n");
    ata_init();
    
    kprint("Initializing PCI and Networking...\n");
    pci_scan_bus();
    rtl8139_init();
    
    void e1000_init(void);
    e1000_init();
    
    void wifi_ath_init(void);
    wifi_ath_init();
    
    /* void sim_wifi_init(void); - declared in header usually, simplifying here */
    /* Networking */
    /* sim_wifi_init(); */
    
    udp_init();
    dns_init();
    
    /*
    e1000_init();
    wifi_ath_init();
    */ /* Disabled per user request (Remove Simulation) */
    
    /* Filesystem Mount */
    struct mbr sector;
    mbr_read(&sector);
    if (sector.signature != 0xAA55) {
        kprint("Disk not initialized. Formatting...\n");
        mbr_write_default();
        if (fat32_format() != 0) {
            kprint_color(KLOG_COLOR_RED, "Critical: Format Failed.\n");
        }
    } else {
        kprint("Valid MBR found.\n");
    }
    

    
    if (fat32_init() == 0) {
        kprint("FAT32 Filesystem Initialized.\n");
        vfs_mount("/", fat32_mount_vfs());
        kprint("VFS: Mounted FAT32 at /\n");
    } else {
        kprint_color(KLOG_COLOR_RED, "Failed to mount filesystem. Formatting disk...\n");
        /* Force re-format if mount fails (e.g. partition exists but is empty) */
        mbr_write_default();
        if (fat32_format() == 0) {
             if (fat32_init() == 0) {
                vfs_mount("/", fat32_mount_vfs());
                kprint_color(KLOG_COLOR_GREEN, "FAT32 Formatted & Mounted via VFS.\n");
             }
        } else {
             kprint_color(KLOG_COLOR_RED, "Critical: Filesystem Mount Failed after Format.\n");
        }
    }

    /* sched_init moved up */
    
    /* Create init process */
    kprint("Creating init process...\n");
    sched_create_task(shell_run); /* Use shell as init for now */
    
    kprint("Enabling Interrupts...\n");
    __asm__ volatile ("sti");

    kprint_color(KLOG_COLOR_GREEN, "\nBoot complete!\n");

    /* Enter idle loop */
    while(1) {
        __asm__ volatile ("hlt");
    }

    /* Should never reach here */
    hcf();
}
