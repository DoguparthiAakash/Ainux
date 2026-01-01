#include "ata.h"
#include "../io.h"

extern void kprint(const char *msg);

static uint16_t active_port = 0x1F0;
static uint8_t active_drive = 0xA0;
static int found_drive = 0;

static void ata_wait_bsy(void) {
    if (!found_drive) return;
    int timeout = 100000;
    while ((inb(active_port + 7) & ATA_SR_BSY) && timeout--);
}

static void ata_wait_drq(void) {
    if (!found_drive) return;
    int timeout = 100000;
    while (!(inb(active_port + 7) & ATA_SR_DRQ) && timeout--);
}

void ata_init(void) {
    kprint("[ATA] Scanning for drives...\\n");
    
    struct {
        uint16_t port;
        uint8_t drive;
        const char *name;
    } drives[] = {
        {0x1F0, 0xA0, "Primary Master"},
        {0x1F0, 0xB0, "Primary Slave"},
        {0x170, 0xA0, "Secondary Master"},
        {0x170, 0xB0, "Secondary Slave"}
    };
    
    for (int i=0; i<4; i++) {
        /* Check Floating Bus first */
        if (inb(drives[i].port + ATA_REG_STATUS) == 0xFF) continue;
        
        /* Select Drive */
        outb(drives[i].port + ATA_REG_DRIVE_HEAD, drives[i].drive);
        
        /* Reset counts */
        outb(drives[i].port + ATA_REG_SECTOR_COUNT, 0);
        outb(drives[i].port + ATA_REG_LBA_LOW, 0);
        outb(drives[i].port + ATA_REG_LBA_MID, 0);
        outb(drives[i].port + ATA_REG_LBA_HIGH, 0);
        
        /* 400ns delay */
        inb(drives[i].port + ATA_REG_STATUS);
        inb(drives[i].port + ATA_REG_STATUS);
        inb(drives[i].port + ATA_REG_STATUS);
        inb(drives[i].port + ATA_REG_STATUS);
        
        outb(drives[i].port + ATA_REG_COMMAND, ATA_CMD_IDENTIFY);
        
        uint8_t status = inb(drives[i].port + ATA_REG_STATUS);
        if (status == 0) continue;
        
        int timeout = 10000;
        while ((inb(drives[i].port + ATA_REG_STATUS) & ATA_SR_BSY) && timeout--);
        
        /* Check for ATAPI */
        uint8_t mid = inb(drives[i].port + ATA_REG_LBA_MID);
        uint8_t hi = inb(drives[i].port + ATA_REG_LBA_HIGH);
        
        if (mid != 0 || hi != 0) {
            kprint("[ATA] Found ATAPI Drive at ");
            kprint(drives[i].name);
            kprint("\\n");
            continue; /* Skip ATAPI (CD-ROM) for now */
        }
        
        timeout = 10000;
        while (!(inb(drives[i].port + ATA_REG_STATUS) & ATA_SR_DRQ) && timeout--);
        if (!(inb(drives[i].port + ATA_REG_STATUS) & ATA_SR_DRQ)) continue;
        
        /* Consume Identify */
        uint16_t buffer[256];
        insw(drives[i].port + ATA_REG_DATA, buffer, 256);
        
        kprint("[ATA] Found ATA Drive at ");
        kprint(drives[i].name);
        kprint("\\n");
        
        /* Set as global active drive (simplified for single disk support) */
        /* Currently ata.h/ata.c assumes pure macros. 
           In a real driver we would store the port/drive config.
           For this task, I'll update the global vars or macros... 
           WAIT, macros are compiled in. I can't change 0x1F0 to 0x170 easily without variables.
           I need to change ata.c to use variables for ports.
        */
        active_port = drives[i].port;
        active_drive = drives[i].drive;
        found_drive = 1;
        return;
    }
    
    kprint("[ATA] No ATA Drives found.\\n");
}

int ata_read_sectors(uint32_t lba, uint8_t count, uint8_t *buffer) {
    /* Select Drive (Master/Slave) + LBA High bits */
    outb(active_port + ATA_REG_DRIVE_HEAD,  0xE0 | (active_drive == 0xA0 ? 0 : 0x10) | ((lba >> 24) & 0x0F));
    /* Wait for selection? */
    
    outb(active_port + ATA_REG_SECTOR_COUNT, count);
    outb(active_port + ATA_REG_LBA_LOW, (uint8_t)lba);
    outb(active_port + ATA_REG_LBA_MID, (uint8_t)(lba >> 8));
    outb(active_port + ATA_REG_LBA_HIGH, (uint8_t)(lba >> 16));
    outb(active_port + ATA_REG_COMMAND, ATA_CMD_READ_PIO);
    
    for (int i = 0; i < count; i++) {
        ata_wait_bsy();
        ata_wait_drq();
        insw(active_port + ATA_REG_DATA, buffer + (i * 512), 256); /* 256 words = 512 bytes */
    }
    return 0;
}

int ata_write_sectors(uint32_t lba, uint8_t count, const uint8_t *buffer) {
    ata_wait_bsy();
    outb(active_port + ATA_REG_DRIVE_HEAD, 0xE0 | (active_drive == 0xA0 ? 0 : 0x10) | ((lba >> 24) & 0x0F));
    
    /* 400ns delay */
    inb(active_port + ATA_REG_STATUS);
    inb(active_port + ATA_REG_STATUS);
    inb(active_port + ATA_REG_STATUS);
    inb(active_port + ATA_REG_STATUS);

    outb(active_port + ATA_REG_SECTOR_COUNT, count);
    outb(active_port + ATA_REG_LBA_LOW, (uint8_t)lba);
    outb(active_port + ATA_REG_LBA_MID, (uint8_t)(lba >> 8));
    outb(active_port + ATA_REG_LBA_HIGH, (uint8_t)(lba >> 16));
    outb(active_port + ATA_REG_COMMAND, ATA_CMD_WRITE_PIO);
    
    for (int i = 0; i < count; i++) {
        ata_wait_bsy();
        ata_wait_drq();
        
        outsw(active_port + ATA_REG_DATA, buffer + (i * 512), 256);
        
        /* Small delay after write before checking status */
        asm volatile("pause");
        ata_wait_bsy();
    }
    
    /* Flush Cache Command (E7h) */
    outb(active_port + ATA_REG_COMMAND, 0xE7); 
    ata_wait_bsy();
    
    /* Check for errors */
    uint8_t status = inb(active_port + ATA_REG_STATUS);
    if (status & 0x01) { /* ERR bit */
        kprint("[ATA] Error during Write/Flush!\n");
        return -1;
    }

    return 0;
}
