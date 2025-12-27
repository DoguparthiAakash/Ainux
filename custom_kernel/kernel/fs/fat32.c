#include "fat32.h"
#include "../drivers/ata.h"
#include "../libc/stdio.h"
#include "../libc/string.h"
#include "../libc/stdlib.h"

extern void *kmalloc(uint64_t size);
extern void kfree(void *ptr);

#define PARTITION_LBA 2048
#define SECTORS_PER_CLUSTER 8
#define RESERVED_SECTORS 32
#define FAT_COUNT 2
#define SECTORS_PER_FAT 1024 /* Covers enough for small disks */

static uint32_t root_cluster_lba = 0;
static uint32_t fat_start_lba = 0;
static uint32_t data_start_lba = 0;

int fat32_format(void) {
    printf("[FAT32] Formatting Partition 1 (LBA %d)...\n", PARTITION_LBA);
    
    /* Use a full sector buffer to avoid stack garbage/overflow */
    uint8_t buffer[512];
    memset(buffer, 0, 512);
    struct fat32_boot_sector *bs = (struct fat32_boot_sector *)buffer;
    
    bs->jump[0] = 0xEB; bs->jump[1] = 0x3C; bs->jump[2] = 0x90;
    memcpy(bs->oem_name, "MSWIN4.1", 8);
    bs->bytes_per_sector = 512;
    bs->sectors_per_cluster = SECTORS_PER_CLUSTER;
    bs->reserved_sectors = RESERVED_SECTORS;
    bs->num_fats = FAT_COUNT;
    bs->hidden_sectors = PARTITION_LBA;
    bs->total_sectors_32 = 120000;
    bs->fat_size_32 = SECTORS_PER_FAT;
    bs->root_cluster = 2;
    bs->fsinfo_sector = 1;
    bs->backup_boot_sector = 6;
    bs->drive_number = 0x80;
    bs->boot_signature = 0x29;
    bs->volume_id = 0x12345678;
    memcpy(bs->volume_label, "AINUIX_DISK", 11);
    memcpy(bs->fs_type, "FAT32   ", 8);
    
    bs->jump[0] = 0xEB;
    
    /* Write Boot Sector */
    ata_write_sectors(PARTITION_LBA, 1, buffer);
    
    /* Clear FATs */
    uint8_t zero_sector[512];
    memset(zero_sector, 0, 512);
    
    /* Only clearing first few sectors of FAT for speed, assumes clean disk image */
    /* Real formatting should clear all FAT sectors */
    printf("[FAT32] Clearing FAT tables...\n");
    for (int i=0; i<32; i++) {
        ata_write_sectors(PARTITION_LBA + RESERVED_SECTORS + i, 1, zero_sector);
    }
    
    /* Init FAT[0] and FAT[1] (Clusters 0 and 1 are reserved) */
    /* FAT[2] is Root Dir EOC */
    uint32_t fat_entries[128]; /* 128 * 4 = 512 bytes */
    memset(fat_entries, 0, 512);
    fat_entries[0] = 0x0FFFFFF8;
    fat_entries[1] = 0x0FFFFFFF; // EOC
    fat_entries[2] = 0x0FFFFFFF; // EOC for Root
    
    ata_write_sectors(PARTITION_LBA + RESERVED_SECTORS, 1, (uint8_t *)fat_entries);
    /* Mirror to FAT 2 */
    ata_write_sectors(PARTITION_LBA + RESERVED_SECTORS + SECTORS_PER_FAT, 1, (uint8_t *)fat_entries);
    
    /* Zero out Root Directory Cluster (Cluster 2) */
    uint32_t data_start = PARTITION_LBA + RESERVED_SECTORS + (FAT_COUNT * SECTORS_PER_FAT);
    printf("[FAT32] Clearing Root Directory at LBA %d...\n", data_start);
    for (int i=0; i<SECTORS_PER_CLUSTER; i++) {
        ata_write_sectors(data_start + i, 1, zero_sector);
    }
    
    printf("[FAT32] Format Complete.\n");
    return 0;
}

static uint32_t cluster_to_lba(uint32_t cluster) {
    return data_start_lba + (cluster - 2) * SECTORS_PER_CLUSTER;
}

int fat32_init(void) {
    /* Read BS to calc offsets */
    /* Use heap to avoid large stack allocs and ensure safe size */
    uint8_t *buffer = (uint8_t *)kmalloc(512);
    if (!buffer) return -1;
    
    ata_read_sectors(PARTITION_LBA, 1, buffer);
    struct fat32_boot_sector *bs = (struct fat32_boot_sector *)buffer;
    
    if (bs->bytes_per_sector != 512) {
        printf("[FAT32] Error: Not 512 BPS or not formatted.\n");
        kfree(buffer);
        return -1;
    }
    
    fat_start_lba = PARTITION_LBA + bs->reserved_sectors;
    data_start_lba = fat_start_lba + (bs->num_fats * bs->fat_size_32);
    root_cluster_lba = cluster_to_lba(bs->root_cluster);
    
    kfree(buffer);
    printf("[FAT32] Mounted. Data Start: %d\n", data_start_lba);
    return 0;
}

/* Simple WRITE FILE (Overwrite/Create in Root Dir) */
int fat32_write_file(const char *filename, const uint8_t *data, uint32_t size) {
    /* Limit: File must fit in one cluster (4KB) for this simple implementation */
    if (size > 4096) {
        printf("[FAT32] Error: File too large/fragmentation not impl.\n");
        return -1;
    }
    
    /* 1. Find free directory entry in Root */
    uint8_t buffer[512];
    uint32_t dir_lba = root_cluster_lba;
    
    /* Scan first sector of root dir */
    ata_read_sectors(dir_lba, 1, buffer);
    struct fat32_dir_entry *dir = (struct fat32_dir_entry *)buffer;
    
    int free_idx = -1;
    for (int i=0; i<16; i++) { /* 16 entries per sector */
        if (dir[i].name[0] == 0x00 || dir[i].name[0] == 0xE5) {
            free_idx = i;
            break;
        }
        /* Check if exists, overwrite? TODO */
    }
    
    if (free_idx == -1) {
        printf("[FAT32] Root dir full (1 sector limit in this stub).\n");
        return -1;
    }
    
    /* 2. Allocate a cluster */
    /* Naive: Hardcode use Cluster 3, then 4... need bitmap or scan FAT */
    /* Scanning FAT for free cluster */
    uint32_t fat_sector[128];
    ata_read_sectors(fat_start_lba, 1, (uint8_t *)fat_sector);
    
    uint32_t new_cluster = 0;
    for (int i=3; i<128; i++) {
        if (fat_sector[i] == 0) {
            new_cluster = i;
            fat_sector[i] = 0x0FFFFFFF; /* EOC */
            break;
        }
    }
    if (new_cluster == 0) {
        printf("[FAT32] Disk Full (FAT sector 0 full).\n");
        return -1;
    }
    
    /* Write FAT back */
    ata_write_sectors(fat_start_lba, 1, (uint8_t *)fat_sector);
    
    /* 3. Write Data */
    uint32_t file_lba = cluster_to_lba(new_cluster);
    /* Write partial sector or full? API takes byte buffer. 
       We align to sector for ATA. Need temp buf if size < 512? 
       For simplicity, assume 'data' is large enough or we copy to temp */
    /* Handling sector alignment safely */
    uint8_t write_buf[4096]; 
    memset(write_buf, 0, 4096);
    memcpy(write_buf, data, size);
    
    ata_write_sectors(file_lba, (size + 511)/512, write_buf);
    
    /* 4. Update Directory Entry */
    struct fat32_dir_entry *entry = &dir[free_idx];
    memset(entry->name, ' ', 11);
    /* Normalize name to 8.3 UPPERCASE */
    int len = strlen(filename);
    for(int i=0; i<len && i<8; i++) {
        char c = filename[i];
        if (c >= 'a' && c <= 'z') c -= 32;
        entry->name[i] = c;
    }
    /* Extension? Simplify to no extension or hardcoded for now */
    
    entry->file_size = size;
    entry->first_cluster_high = (new_cluster >> 16);
    entry->first_cluster_low = (new_cluster & 0xFFFF);
    entry->attributes = 0;
    
    /* Write Dir Back */
    ata_write_sectors(dir_lba, 1, buffer);
    printf("[FAT32] Wrote %s (%d bytes) to Cluster %d\n", filename, size, new_cluster);
    return 0;
}

void fat32_list_files(void) {
     printf("[FAT32] Listing Files...\n");
     uint8_t *buffer = (uint8_t *)kmalloc(512);
     if (!buffer) {
         printf("[FAT32] OOM\n");
         return;
     }
     
     printf("[FAT32] Reading LBA %d\n", root_cluster_lba);
     ata_read_sectors(root_cluster_lba, 1, buffer);
     
     struct fat32_dir_entry *dir = (struct fat32_dir_entry *)buffer;
     
     printf("[FAT32] Root Listing:\n");
     for(int i=0; i<16; i++) {
         if(dir[i].name[0] != 0x00 && dir[i].name[0] != 0xE5) {
             char name[12];
             memcpy(name, dir[i].name, 11);
             name[11] = 0;
             printf("  - %s (%d bytes)\n", name, dir[i].file_size);
         }
     }
     
     kfree(buffer);
     printf("[FAT32] Done Listing.\n");
}

int fat32_read_file(const char *filename, uint8_t **data, uint32_t *size) {
    (void)filename; (void)data; (void)size;
    return -1; /* TODO */
}
