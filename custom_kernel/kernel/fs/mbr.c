#include <stdint.h>
#include <string.h>
#include "../drivers/ata.h"
#include "../libc/stdio.h"
#include "mbr.h"

/* Structs moved to mbr.h */

void mbr_read(struct mbr *buffer) {
    ata_read_sectors(0, 1, (uint8_t *)buffer);
}

void mbr_print_map(void) {
    struct mbr sector;
    mbr_read(&sector);
    
    if (sector.signature != 0xAA55) {
        printf("[MBR] Invalid Signature (0x%x). Disk likely uninitialized.\n", sector.signature);
        return;
    }
    
    printf("\n=== Disk Partition Map ===\n");
    printf("Total Disk Size: 64 MB (Hardcoded in emulator)\n");
    printf("ID | Status | Type | Start LBA | Size (Sectors) | Size (MB)\n");
    printf("---|--------|------|-----------|----------------|----------\n");
    
    uint32_t last_end = 1; /* MBR is at 0, usually start partitions after reserved area */
    /* Typical first partition starts at 2048 */
    
    for (int i=0; i<4; i++) {
        struct mbr_entry *e = &sector.partitions[i];
        if (e->type != 0) {
            /* Check for unallocated space before this partition */
             if (e->lba_start > last_end && (e->lba_start - last_end) > 2048) {
                uint32_t gap = e->lba_start - last_end;
                printf(" - | Unallocated   | %d      | %d           | %d MB\n", 
                       last_end, gap, (gap * 512) / 1024 / 1024);
            }
            
            printf(" %d | %s   | 0x%02x | %d      | %d         | %d MB\n",
                   i+1, 
                   (e->status & 0x80) ? "Boot  " : "Active",
                   e->type,
                   e->lba_start,
                   e->lba_length,
                   (e->lba_length * 512) / 1024 / 1024);
                   
            last_end = e->lba_start + e->lba_length;
        }
    }
    
    /* Check for trailing unallocated space */
    /* 64MB = 131072 sectors */
    uint32_t total_sectors = 131072;
    if (last_end < total_sectors) {
        uint32_t gap = total_sectors - last_end;
        printf(" - | Unallocated   | %d      | %d           | %d MB\n", 
               last_end, gap, (gap * 512) / 1024 / 1024);
    }
    printf("==========================\n\n");
}

int mbr_new_partition(uint32_t size_mb) {
    struct mbr sector;
    mbr_read(&sector);
    
    if (sector.signature != 0xAA55) {
        /* Initialize MBR if missing */
        printf("[MBR] Initializing new MBR...\n");
        memset(&sector, 0, sizeof(struct mbr));
        sector.signature = 0xAA55;
    }
    
    /* Find empty slot */
    int slot = -1;
    uint32_t start_lba = 2048; /* Default start for empty disk */
    
    for (int i=0; i<4; i++) {
        if (sector.partitions[i].type == 0) {
             if (slot == -1) slot = i;
        } else {
            /* If partitions exist, try to append? */
            /* Simple strategy: Look at the end of the last used partition */
            uint32_t end = sector.partitions[i].lba_start + sector.partitions[i].lba_length;
            if (end > start_lba) start_lba = end + 2048; /* Alignment padding */
        }
    }
    
    if (slot == -1) {
        printf("[MBR] Error: No free partition slots (Max 4).\n");
        return -1;
    }
    
    uint32_t size_sectors = (size_mb * 1024 * 1024) / 512;
    uint32_t total_sectors = 131072; /* 64MB */
    
    if (start_lba + size_sectors > total_sectors) {
        printf("[MBR] Error: Not enough space. Need %d sectors, have %d.\n", 
               size_sectors, total_sectors - start_lba);
        return -1;
    }
    
    printf("[MBR] Creating Partition %d: Start=%d, Size=%d MB\n", 
           slot+1, start_lba, size_mb);
           
    sector.partitions[slot].status = 0x80; /* Bootable by default */
    sector.partitions[slot].type = 0x0C;   /* FAT32 LBA */
    sector.partitions[slot].lba_start = start_lba;
    sector.partitions[slot].lba_length = size_sectors;
    
    ata_write_sectors(0, 1, (uint8_t *)&sector);
    printf("[MBR] Partition Table Updated.\n");
    return slot;
}

void mbr_write_default(void) {
    printf("[MBR] Resetting to default layout...\n");
    struct mbr mbr_sector;
    memset(&mbr_sector, 0, sizeof(struct mbr));
    
    mbr_sector.partitions[0].status = 0x80;
    mbr_sector.partitions[0].type = 0x0C; /* FAT32 LBA */
    mbr_sector.partitions[0].lba_start = 2048;
    mbr_sector.partitions[0].lba_length = 120000; /* ~60MB */
    
    mbr_sector.signature = 0xAA55;
    
    ata_write_sectors(0, 1, (uint8_t *)&mbr_sector);
    printf("[MBR] Written.\n");
}
