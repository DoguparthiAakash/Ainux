#include "fat32.h"
#include "../drivers/ata.h"
#include "../libc/stdio.h"
#include "../libc/string.h"
#include "../libc/stdlib.h"

extern void *kmalloc(uint64_t size);
extern void kfree(void *ptr);

#include "mbr.h"
#include "gpt.h"

static uint32_t fat32_partition_lba = 2048; /* Default fallback */
#define SECTORS_PER_CLUSTER 8
#define RESERVED_SECTORS 32
#define FAT_COUNT 2
#define SECTORS_PER_FAT 1024 /* Covers enough for small disks */

static uint32_t root_cluster_lba = 0;
static uint32_t fat_start_lba = 0;
static uint32_t data_start_lba = 0;
static uint32_t current_dir_cluster = 0; /* 0 means Root */
static uint32_t parent_dir_cluster = 0;  /* For cd .. support */

static uint32_t cluster_to_lba(uint32_t cluster);

static uint32_t get_current_dir_lba(void) {
    if (current_dir_cluster == 0 || current_dir_cluster == 2) return root_cluster_lba;
    return cluster_to_lba(current_dir_cluster);
}

int fat32_change_dir(const char *path) {
    /* Handle Root Reset */
    if (strcmp(path, "/") == 0) {
        current_dir_cluster = 2; /* Root */
        parent_dir_cluster = 0;
        printf("[FAT32] CD to Root\n");
        return 0;
    }
    
    /* Handle Parent Directory */
    if (strcmp(path, "..") == 0) {
        if (current_dir_cluster == 2 || current_dir_cluster == 0) {
            printf("[FAT32] Already at root\n");
            return 0;
        }
        current_dir_cluster = parent_dir_cluster;
        /* Need to read .. entry to get grandparent, but for now just go to root */
        if (current_dir_cluster == 0) current_dir_cluster = 2;
        printf("[FAT32] CD to parent\n");
        return 0;
    }

    uint8_t *buffer = (uint8_t *)kmalloc(512);
    if (!buffer) return -1;
    
    uint32_t dir_lba = get_current_dir_lba();
    ata_read_sectors(dir_lba, 1, buffer);
    struct fat32_dir_entry *dir = (struct fat32_dir_entry *)buffer;
    
    /* Parse matching name */
    char fat_name[11];
    memset(fat_name, ' ', 11);
    
    int len = strlen(path);
    int dot_pos = -1;
    for(int i=0; i<len; i++) if(path[i] == '.') dot_pos = i;
    
    int name_part_len = (dot_pos == -1) ? len : dot_pos;
    if (name_part_len > 8) name_part_len = 8;
    
    for(int i=0; i<name_part_len; i++) {
        char c = path[i];
        if (c >= 'a' && c <= 'z') c -= 32;
        fat_name[i] = c;
    }
    
    int found_cluster = -1;
    
    for(int i=0; i<16; i++) {
        if(dir[i].name[0] != 0x00 && dir[i].name[0] != 0xE5) {
            /* Check if directory */
            if ((dir[i].attributes & 0x10)) { /* Directory */
                if (memcmp(dir[i].name, fat_name, 11) == 0) {
                    found_cluster = (dir[i].first_cluster_high << 16) | dir[i].first_cluster_low;
                    if (found_cluster == 0) found_cluster = 2; /* Point back to root */
                    break;
                }
            }
        }
    }
    
    kfree(buffer);
    
    if (found_cluster != -1) {
        parent_dir_cluster = current_dir_cluster;
        current_dir_cluster = found_cluster;
        return 0;
    }
    
    return -1;
}

int fat32_format(void) {
    printf("[FAT32] Formatting Partition at LBA %d...\n", fat32_partition_lba);
    
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
    bs->hidden_sectors = fat32_partition_lba;
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
    ata_write_sectors(fat32_partition_lba, 1, buffer);
    
    /* Clear FATs */
    uint8_t zero_sector[512];
    memset(zero_sector, 0, 512);
    
    printf("[FAT32] Clearing FAT tables...\n");
    for (int i=0; i<32; i++) {
        ata_write_sectors(fat32_partition_lba + RESERVED_SECTORS + i, 1, zero_sector);
    }
    
    /* Init FAT[0] and FAT[1] (Clusters 0 and 1 are reserved) */
    uint32_t fat_entries[128];
    memset(fat_entries, 0, 512);
    fat_entries[0] = 0x0FFFFFF8;
    fat_entries[1] = 0x0FFFFFFF; // EOC
    fat_entries[2] = 0x0FFFFFFF; // EOC for Root
    
    ata_write_sectors(fat32_partition_lba + RESERVED_SECTORS, 1, (uint8_t *)fat_entries);
    ata_write_sectors(fat32_partition_lba + RESERVED_SECTORS + SECTORS_PER_FAT, 1, (uint8_t *)fat_entries);
    
    /* Zero out Root Directory Cluster (Cluster 2) */
    uint32_t data_start = fat32_partition_lba + RESERVED_SECTORS + (FAT_COUNT * SECTORS_PER_FAT);
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
    /* Try GPT first (modern drives) */
    uint32_t gpt_lba = 0;
    if (gpt_find_fat32_partition(&gpt_lba) == 0) {
        fat32_partition_lba = gpt_lba;
        printf("[FAT32] Using GPT partition at LBA %d\n", fat32_partition_lba);
    } else {
        /* Fall back to MBR (legacy drives) */
        struct mbr mbr_buf;
        mbr_read(&mbr_buf);
        
        if (mbr_buf.signature == 0xAA55) {
            int found = 0;
            for (int i=0; i<4; i++) {
                /* Check for FAT32 types: 0x0B (CHS), 0x0C (LBA), 0x07 (ExFAT/Win), 0xEF (EFI) */
                uint8_t t = mbr_buf.partitions[i].type;
                if (t == 0x0B || t == 0x0C || t == 0x07 || t == 0xEF || t == 0x0E) {
                    fat32_partition_lba = mbr_buf.partitions[i].lba_start;
                    printf("[FAT32] Found MBR Partition %d (Type %02X) at LBA %d\n", i+1, t, fat32_partition_lba);
                    found = 1;
                    break;
                }
            }
            if (!found) {
                printf("[FAT32] No standard partition found. Trying generic LBA 2048.\n");
                fat32_partition_lba = 2048;
            }
        } else {
            printf("[FAT32] Invalid MBR. Assuming LBA 2048.\n");
            fat32_partition_lba = 2048;
        }
    }

    /* Read BS to calc offsets */
    uint8_t *buffer = (uint8_t *)kmalloc(512);
    if (!buffer) return -1;
    
    ata_read_sectors(fat32_partition_lba, 1, buffer);
    struct fat32_boot_sector *bs = (struct fat32_boot_sector *)buffer;
    
    /* Validate Boot Sector */
    if (bs->bytes_per_sector != 512) {
        printf("[FAT32] Error: Invalid sector size %d (expected 512)\n", bs->bytes_per_sector);
        kfree(buffer);
        return -1;
    }
    
    /* Check for boot signature (0xAA55 at offset 510) */
    uint16_t boot_sig = *(uint16_t*)(buffer + 510);
    if (boot_sig != 0xAA55) {
        printf("[FAT32] Error: Invalid boot signature 0x%04X\n", boot_sig);
        kfree(buffer);
        return -1;
    }
    
    /* Sanity checks */
    if (bs->sectors_per_cluster == 0 || bs->sectors_per_cluster > 128) {
        printf("[FAT32] Error: Invalid cluster size\n");
        kfree(buffer);
        return -1;
    }
    
    if (bs->num_fats == 0 || bs->num_fats > 2) {
        printf("[FAT32] Error: Invalid FAT count\n");
        kfree(buffer);
        return -1;
    }
    
    fat_start_lba = fat32_partition_lba + bs->reserved_sectors;
    data_start_lba = fat_start_lba + (bs->num_fats * bs->fat_size_32);
    root_cluster_lba = cluster_to_lba(bs->root_cluster);
    
    kfree(buffer);
    printf("[FAT32] Mounted. Data Start: %d, Root: %d\n", data_start_lba, root_cluster_lba);
    return 0;
}

/* Simple WRITE FILE (Overwrite/Create in Root Dir) */
int fat32_write_file(const char *filename, const uint8_t *data, uint32_t size) {
    /* Limit: File must fit in one cluster (4KB) for this simple implementation */
    /* Limit: File must fit in one cluster (4KB) for this simple implementation */
    if (size > 4096) {
        printf("[FAT32] Error: File too large/fragmentation not impl.\n");
        return -1;
    }
    
    /* Allocate buffers on Heap to verify stack safety and persistence */
    uint8_t *dir_sector = (uint8_t *)kmalloc(512);
    if (!dir_sector) return -1;
    
    /* 1. Find free directory entry in Current Dir */
    uint32_t dir_lba = get_current_dir_lba();
    
    ata_read_sectors(dir_lba, 1, dir_sector);
    struct fat32_dir_entry *dir = (struct fat32_dir_entry *)dir_sector;
    
    /* PARSE FILENAME FIRST to check existence */
    char fat_name[11];
    memset(fat_name, ' ', 11);
    int len = strlen(filename);
    int dot_pos = -1;
    for(int i=0; i<len; i++) if(filename[i] == '.') dot_pos = i;
    
    int name_part_len = (dot_pos == -1) ? len : dot_pos;
    if (name_part_len > 8) name_part_len = 8;
    
    for(int i=0; i<name_part_len; i++) {
        char c = filename[i];
        if (c >= 'a' && c <= 'z') c -= 32;
        fat_name[i] = c;
    }
    
    if (dot_pos != -1) {
        int ext_len = len - dot_pos - 1;
        if (ext_len > 3) ext_len = 3;
        for(int i=0; i<ext_len; i++) {
            char c = filename[dot_pos + 1 + i];
            if (c >= 'a' && c <= 'z') c -= 32;
            fat_name[8+i] = c;
        }
    }
    
    /* Check if file exists */
    int existing_idx = -1;
    for(int i=0; i<16; i++) {
        if(dir[i].name[0] != 0x00 && dir[i].name[0] != 0xE5) {
            if (memcmp(dir[i].name, fat_name, 11) == 0) {
                 existing_idx = i;
                 break;
            }
        }
    }
    
    uint32_t target_cluster = 0;
    int target_idx = -1;
    
    if (existing_idx != -1) {
         printf("[FAT32] Overwriting existing file...\n");
         target_idx = existing_idx;
         target_cluster = (dir[existing_idx].first_cluster_high << 16) | dir[existing_idx].first_cluster_low;
    } else {
        /* Find Free Slot */
        for (int i=0; i<16; i++) { 
            if (dir[i].name[0] == 0x00 || dir[i].name[0] == 0xE5) {
                target_idx = i;
                break;
            }
        }
        if (target_idx == -1) {
             printf("[FAT32] Root dir full.\n");
             kfree(dir_sector);
             return -1;
        }
        
        /* Allocate New Cluster */
        uint8_t *fat_table = (uint8_t *)kmalloc(512);
        if (!fat_table) { kfree(dir_sector); return -1; }
        
        for (int sector_offset = 0; sector_offset < 128; sector_offset++) {
            ata_read_sectors(fat_start_lba + sector_offset, 1, fat_table);
            uint32_t *fat32 = (uint32_t *)fat_table;
            
            int start_idx = (sector_offset == 0) ? 3 : 0;
            
            int found_in_sector = -1;
            for (int i = start_idx; i < 128; i++) {
                if (fat32[i] == 0) {
                    found_in_sector = i;
                    fat32[i] = 0x0FFFFFFF; /* EOC */
                    break;
                }
            }
            
            if (found_in_sector != -1) {
                target_cluster = (sector_offset * 128) + found_in_sector;
                ata_write_sectors(fat_start_lba + sector_offset, 1, fat_table);
                break;
            }
        }
        
        kfree(fat_table);
        
        if (target_cluster == 0) {
            printf("[FAT32] Disk Full (Checked 128 FAT sectors).\n");
            kfree(dir_sector);
            return -1;
        }
    }
    
    /* Write Data */
    uint32_t file_lba = cluster_to_lba(target_cluster);
    uint8_t *write_buf = (uint8_t *)kmalloc(4096);
    if (!write_buf) { kfree(dir_sector); return -1; }
    
    memset(write_buf, 0, 4096);
    memcpy(write_buf, data, size);
    
    ata_write_sectors(file_lba, (size + 511)/512, write_buf);
    kfree(write_buf);
    
    /* Update Directory Entry */
    struct fat32_dir_entry *entry = &dir[target_idx];
    memcpy(entry->name, fat_name, 11);
    entry->file_size = size;
    entry->first_cluster_high = (target_cluster >> 16);
    entry->first_cluster_low = (target_cluster & 0xFFFF);
    entry->attributes = 0x20; /* ATTR_ARCHIVE */
    
    /* Write Dir Back */
    ata_write_sectors(dir_lba, 1, dir_sector);
    kfree(dir_sector);
    
    printf("[FAT32] Wrote %s (%d bytes) to Cluster %d\n", filename, size, target_cluster);
    return 0;
}

void fat32_list_files(void) {
     printf("[FAT32] Listing Files...\n");
     uint8_t *buffer = (uint8_t *)kmalloc(512);
     if (!buffer) {
         printf("[FAT32] OOM\n");
         return;
     }
     
     uint32_t dir_lba = get_current_dir_lba();
     printf("[FAT32] Reading LBA %d\n", dir_lba);
     ata_read_sectors(dir_lba, 1, buffer);
     
     struct fat32_dir_entry *dir = (struct fat32_dir_entry *)buffer;
     
     printf("[FAT32] Directory Listing:\n");
     for(int i=0; i<16; i++) {
         if(dir[i].name[0] != 0x00 && dir[i].name[0] != 0xE5 && dir[i].attributes != 0x0F) {
             /* Convert 8.3 to Name.Ext */
             char name[13];
             int len = 0;
             /* Extract Name */
             for(int j=0; j<8; j++) {
                 if(dir[i].name[j] != ' ') name[len++] = dir[i].name[j];
             }
             /* Extract Ext */
             if(dir[i].name[8] != ' ') {
                 name[len++] = '.';
                 for(int j=0; j<3; j++) {
                     if(dir[i].name[8+j] != ' ') name[len++] = dir[i].name[8+j];
                 }
             }
             name[len] = '\0';
             
             char type = (dir[i].attributes & 0x10) ? 'D' : 'F';
             printf("  - [%c] %s (%d bytes)\n", type, name, dir[i].file_size);
         }
     }
     
     kfree(buffer);
     printf("[FAT32] Done Listing.\n");
}

int fat32_read_file(const char *filename, uint8_t **data, uint32_t *size) {
    /* 1. Find directory entry */
    uint8_t *buffer = (uint8_t *)kmalloc(512);
    if (!buffer) return -1;

    uint32_t dir_lba = get_current_dir_lba();
    ata_read_sectors(dir_lba, 1, buffer);
    struct fat32_dir_entry *dir = (struct fat32_dir_entry *)buffer;
    
    int found_idx = -1;
    /* Normalize input to 8.3 upper */
    /* Naive match for now: Just match first 8 chars case insensitive */
    
    for(int i=0; i<16; i++) {
        if(dir[i].name[0] != 0x00 && dir[i].name[0] != 0xE5) {
             /* Check name match */
             char entry_name[12];
             memcpy(entry_name, dir[i].name, 11);
             entry_name[11] = 0;
             /* Clean comparison TODO */
             /* For now, just require exact upper match of mapped name */
             /* Hack: Assume user types "TEST    TXT" format? No. */
             /* Let's just return first file for testing or fix comparison */
             /* Fix comparison later. */
             
             /* Simple substring match? */
             /* Comparing normalized input */
             /* Assume filename is mapped to "NAME    EXT" */
        }
    }
    /* Re-implementing search with cleaner logic */
    /* Convert filename to FAT format "NAME    " */
    char fat_name[11];
    memset(fat_name, ' ', 11);
    int fn_len = strlen(filename);
    int dot_pos = -1;
    for(int j=0; j<fn_len; j++) if(filename[j]=='.') dot_pos=j;
    
    int name_len = (dot_pos == -1) ? fn_len : dot_pos;
    if (name_len > 8) name_len=8;
    
    for(int j=0; j<name_len; j++) {
        char c = filename[j];
        if(c >= 'a' && c <= 'z') c -= 32;
        fat_name[j] = c;
    }
    
    // Ext
    if (dot_pos != -1) {
        int ext_len = fn_len - dot_pos - 1;
        if (ext_len > 3) ext_len = 3;
        for(int j=0; j<ext_len; j++) {
            char c = filename[dot_pos+1+j];
            if(c >= 'a' && c <= 'z') c -= 32;
            fat_name[8+j] = c;
        }
    }
    
    /* Search */
    for(int i=0; i<16; i++) {
        if(dir[i].name[0] != 0x00 && dir[i].name[0] != 0xE5) {
            if (memcmp(dir[i].name, fat_name, 11) == 0) {
                found_idx = i;
                break;
            }
        }
    }
    
    if (found_idx == -1) {
        kfree(buffer);
        return -1;
    }
    
    /* Found! Read Cluster */
    struct fat32_dir_entry *entry = &dir[found_idx];
    uint32_t cluster = (entry->first_cluster_high << 16) | entry->first_cluster_low;
    uint32_t f_size = entry->file_size;
    
    uint32_t lba = cluster_to_lba(cluster);
    
    /* Allocate Data Limit 4KB for now (1 cluster8 sectors) */
    /* Our cluster calc assumes 8 sectors per cluster in format */
    /* Actually init says bs->sectors_per_cluster */
    /* Let's safeguard */
    if (f_size == 0) f_size = 512; /* Empty file? */
    
    uint8_t *file_data = (uint8_t *)kmalloc(f_size + 512); /* Align padding */
    if (!file_data) {
        kfree(buffer);
        return -1;
    }
    
    /* Read Sectors */
    int sectors = (f_size + 511) / 512;
    ata_read_sectors(lba, sectors, file_data);
    
    *data = file_data;
    *size = f_size;
    
    kfree(buffer);
    return 0;
}

int fat32_create_dir(const char *dirname) {
    /* Allocate buffers on Heap */
    uint8_t *dir_sector = (uint8_t *)kmalloc(512);
    if (!dir_sector) return -1;
    
    /* 1. Find free directory entry in Current Dir to store the new folder info */
    uint32_t dir_lba = get_current_dir_lba();
    
    ata_read_sectors(dir_lba, 1, dir_sector);
    struct fat32_dir_entry *dir = (struct fat32_dir_entry *)dir_sector;
    
    /* Parse Name */
    char fat_name[11];
    memset(fat_name, ' ', 11);
    int len = strlen(dirname);
    int dot_pos = -1; /* Directories usually don't have extensions in standard usage but FAT supports it */
    /* Treat creating "bin" as Name "BIN     " Ext "   " */
    
    /* Simple normalization */
    int name_part_len = len;
    if (name_part_len > 8) name_part_len = 8;
    
    for(int i=0; i<name_part_len; i++) {
        char c = dirname[i];
        if (c >= 'a' && c <= 'z') c -= 32;
        fat_name[i] = c;
    }
    
    /* Check Existence */
    for(int i=0; i<16; i++) {
        if(dir[i].name[0] != 0x00 && dir[i].name[0] != 0xE5) {
            if (memcmp(dir[i].name, fat_name, 11) == 0) {
                 printf("[FAT32] Directory/File exists.\n");
                 kfree(dir_sector);
                 return -1;
            }
        }
    }
    
    /* Find Free Slot */
    int target_idx = -1;
    for (int i=0; i<16; i++) { 
        if (dir[i].name[0] == 0x00 || dir[i].name[0] == 0xE5) {
            target_idx = i;
            break;
        }
    }
    if (target_idx == -1) {
         printf("[FAT32] Current Dir full.\n");
         kfree(dir_sector);
         return -1;
    }
    
    /* 2. Allocate New Cluster for the Directory Content */
    /* Find Free Cluster in FAT - SCANNING UP TO 128 SECTORS (65536 clusters ~ 256MB) */
    uint32_t target_cluster = 0;
    
    /* Allocate buffer for FAT sector */
    uint8_t *fat_table = (uint8_t *)kmalloc(512);
    if (!fat_table) { kfree(dir_sector); return -1; }
    
    for (int sector_offset = 0; sector_offset < 128; sector_offset++) {
        ata_read_sectors(fat_start_lba + sector_offset, 1, fat_table);
        uint32_t *fat32 = (uint32_t *)fat_table;
        
        int start_idx = (sector_offset == 0) ? 3 : 0; /* Skip init entries on first sector */
        
        int found_in_sector = -1;
        for (int i = start_idx; i < 128; i++) {
            if (fat32[i] == 0) {
                found_in_sector = i;
                fat32[i] = 0x0FFFFFFF; /* EOC */
                break;
            }
        }
        
        if (found_in_sector != -1) {
            /* Found one! */
            target_cluster = (sector_offset * 128) + found_in_sector;
            /* Write back specific sector */
            ata_write_sectors(fat_start_lba + sector_offset, 1, fat_table);
            break;
        }
    }
    
    kfree(fat_table);
    
    if (target_cluster == 0) {
        printf("[FAT32] Disk Full (Checked 128 FAT sectors).\n");
        kfree(dir_sector);
        return -1;
    }
    
    /* 3. Init New Directory Cluster (Clear it and add . and ..) */
    uint32_t new_dir_lba = cluster_to_lba(target_cluster);
    uint8_t *new_dir_buf = (uint8_t *)kmalloc(512); /* One sector init */
    memset(new_dir_buf, 0, 512);
    struct fat32_dir_entry *new_entries = (struct fat32_dir_entry *)new_dir_buf;
    
    /* Entry 1: . (Self) */
    memset(new_entries[0].name, ' ', 11);
    new_entries[0].name[0] = '.';
    new_entries[0].attributes = 0x10; /* Directory */
    new_entries[0].first_cluster_high = (target_cluster >> 16);
    new_entries[0].first_cluster_low = (target_cluster & 0xFFFF);
    
    /* Entry 2: .. (Parent) */
    memset(new_entries[1].name, ' ', 11);
    new_entries[1].name[0] = '.';
    new_entries[1].name[1] = '.';
    new_entries[1].attributes = 0x10;
    /* If parent is Root (LBA 2), FAT32 convention says use cluster 0 in .. entry usually, but 2 works too. 
       Let's use current_dir_cluster */
    uint32_t parent_cluster = current_dir_cluster;
    if (parent_cluster == 2) parent_cluster = 0; /* Standard says 0 for Root */
    
    new_entries[1].first_cluster_high = (parent_cluster >> 16);
    new_entries[1].first_cluster_low = (parent_cluster & 0xFFFF);
    
    /* Write initialized directory sector */
    ata_write_sectors(new_dir_lba, 1, new_dir_buf); /* Only writing 1st sector of cluster, assumes others act as zeroes */
    /* Ideally clear all sectors in cluster */
    kfree(new_dir_buf);
    
    /* 4. Update Parent Directory Entry */
    struct fat32_dir_entry *entry = &dir[target_idx];
    memcpy(entry->name, fat_name, 11);
    entry->file_size = 0;
    entry->first_cluster_high = (target_cluster >> 16);
    entry->first_cluster_low = (target_cluster & 0xFFFF);
    entry->attributes = 0x10; /* ATTR_DIRECTORY */
    
    /* Write Parent Dir Back */
    ata_write_sectors(dir_lba, 1, dir_sector);
    kfree(dir_sector);
    
    printf("[FAT32] Created Directory %s at Cluster %d\n", dirname, target_cluster);
    return 0;
}

int fat32_delete_file(const char *filename) {
    uint8_t *dir_sector = (uint8_t *)kmalloc(512);
    if (!dir_sector) return -1;
    
    uint32_t dir_lba = get_current_dir_lba();
    ata_read_sectors(dir_lba, 1, dir_sector);
    struct fat32_dir_entry *dir = (struct fat32_dir_entry *)dir_sector;
    
    /* Parse filename to FAT format */
    char fat_name[11];
    memset(fat_name, ' ', 11);
    int len = strlen(filename);
    int dot_pos = -1;
    for(int i=0; i<len; i++) if(filename[i] == '.') dot_pos = i;
    
    int name_part_len = (dot_pos == -1) ? len : dot_pos;
    if (name_part_len > 8) name_part_len = 8;
    
    for(int i=0; i<name_part_len; i++) {
        char c = filename[i];
        if (c >= 'a' && c <= 'z') c -= 32;
        fat_name[i] = c;
    }
    
    if (dot_pos != -1) {
        int ext_len = len - dot_pos - 1;
        if (ext_len > 3) ext_len = 3;
        for(int i=0; i<ext_len; i++) {
            char c = filename[dot_pos + 1 + i];
            if (c >= 'a' && c <= 'z') c -= 32;
            fat_name[8+i] = c;
        }
    }
    
    /* Find file */
    int found_idx = -1;
    uint32_t file_cluster = 0;
    
    for(int i=0; i<16; i++) {
        if(dir[i].name[0] != 0x00 && dir[i].name[0] != 0xE5) {
            if (memcmp(dir[i].name, fat_name, 11) == 0) {
                found_idx = i;
                file_cluster = (dir[i].first_cluster_high << 16) | dir[i].first_cluster_low;
                break;
            }
        }
    }
    
    if (found_idx == -1) {
        printf("[FAT32] File not found\n");
        kfree(dir_sector);
        return -1;
    }
    
    /* Mark directory entry as deleted */
    dir[found_idx].name[0] = 0xE5;
    ata_write_sectors(dir_lba, 1, dir_sector);
    kfree(dir_sector);
    
    /* Free cluster in FAT */
    if (file_cluster >= 3) {
        uint8_t *fat_table = (uint8_t *)kmalloc(512);
        if (fat_table) {
            uint32_t fat_sector = file_cluster / 128;
            uint32_t fat_offset = file_cluster % 128;
            
            ata_read_sectors(fat_start_lba + fat_sector, 1, fat_table);
            uint32_t *fat32 = (uint32_t *)fat_table;
            fat32[fat_offset] = 0;  /* Mark as free */
            ata_write_sectors(fat_start_lba + fat_sector, 1, fat_table);
            kfree(fat_table);
        }
    }
    
    printf("[FAT32] Deleted %s\n", filename);
    return 0;
}

