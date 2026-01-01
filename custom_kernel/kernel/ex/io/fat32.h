#ifndef FAT32_H
#define FAT32_H

#include <stdint.h>
#include <stddef.h>

/* FAT32 Boot Sector */
struct fat32_boot_sector {
    uint8_t  jump[3];
    uint8_t  oem_name[8];
    uint16_t bytes_per_sector;
    uint8_t  sectors_per_cluster;
    uint16_t reserved_sectors;
    uint8_t  num_fats;
    uint16_t root_entries;          /* 0 for FAT32 */
    uint16_t total_sectors_16;      /* 0 for FAT32 */
    uint8_t  media_type;
    uint16_t fat_size_16;           /* 0 for FAT32 */
    uint16_t sectors_per_track;
    uint16_t num_heads;
    uint32_t hidden_sectors;
    uint32_t total_sectors_32;
    
    /* FAT32 specific */
    uint32_t fat_size_32;
    uint16_t flags;
    uint16_t version;
    uint32_t root_cluster;
    uint16_t fsinfo_sector;
    uint16_t backup_boot_sector;
    uint8_t  reserved[12];
    uint8_t  drive_number;
    uint8_t  reserved1;
    uint8_t  boot_signature;
    uint32_t volume_id;
    uint8_t  volume_label[11];
    uint8_t  fs_type[8];
} __attribute__((packed));

/* FAT32 Directory Entry */
struct fat32_dir_entry {
    uint8_t  name[11];              /* 8.3 filename */
    uint8_t  attributes;
    uint8_t  reserved;
    uint8_t  creation_time_tenth;
    uint16_t creation_time;
    uint16_t creation_date;
    uint16_t last_access_date;
    uint16_t first_cluster_high;
    uint16_t last_write_time;
    uint16_t last_write_date;
    uint16_t first_cluster_low;
    uint32_t file_size;
} __attribute__((packed));

/* File attributes */
#define FAT_ATTR_READ_ONLY  0x01
#define FAT_ATTR_HIDDEN     0x02
#define FAT_ATTR_SYSTEM     0x04
#define FAT_ATTR_VOLUME_ID  0x08
#define FAT_ATTR_DIRECTORY  0x10
#define FAT_ATTR_ARCHIVE    0x20

/* Special cluster values */
#define FAT32_EOC           0x0FFFFFF8  /* End of chain */
#define FAT32_BAD_CLUSTER   0x0FFFFFF7

/* Initialize FAT32 filesystem (Mount) */
int fat32_init(void);

/* Format Partition 1 */
int fat32_format(void);

/* List files in root directory */
void fat32_list_files(void);

struct vfs_node;
struct vfs_node *fat32_mount_vfs(void);

/* Read a file by name */
int fat32_read_file(const char *filename, uint8_t **data, uint32_t *size);

/* Write/Create a file in root directory */
int fat32_write_file(const char *filename, const uint8_t *data, uint32_t size);

/* Change current directory */
int fat32_change_dir(const char *path);

/* Create Directory */
int fat32_create_dir(const char *dirname);

/* Delete File */
int fat32_delete_file(const char *filename);
int fat32_rename(const char *oldname, const char *newname);

#endif
