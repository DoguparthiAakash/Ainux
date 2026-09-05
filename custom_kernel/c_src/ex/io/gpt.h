#ifndef GPT_H
#define GPT_H

#include <stdint.h>

/* GPT Header (LBA 1) */
struct gpt_header {
    uint64_t signature;          /* "EFI PART" = 0x5452415020494645 */
    uint32_t revision;
    uint32_t header_size;
    uint32_t header_crc32;
    uint32_t reserved;
    uint64_t current_lba;
    uint64_t backup_lba;
    uint64_t first_usable_lba;
    uint64_t last_usable_lba;
    uint8_t  disk_guid[16];
    uint64_t partition_entries_lba;
    uint32_t num_partition_entries;
    uint32_t partition_entry_size;
    uint32_t partition_array_crc32;
} __attribute__((packed));

/* GPT Partition Entry (128 bytes) */
struct gpt_partition_entry {
    uint8_t  type_guid[16];
    uint8_t  unique_guid[16];
    uint64_t first_lba;
    uint64_t last_lba;
    uint64_t attributes;
    uint16_t name[36];  /* UTF-16LE partition name */
} __attribute__((packed));

/* Common Partition Type GUIDs */
/* EFI System Partition: C12A7328-F81F-11D2-BA4B-00A0C93EC93B */
#define GPT_TYPE_EFI_SYSTEM \
    {0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, \
     0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B}

/* Microsoft Basic Data: EBD0A0A2-B9E5-4433-87C0-68B6B72699C7 */
#define GPT_TYPE_BASIC_DATA \
    {0xA2, 0xA0, 0xD0, 0xEB, 0xE5, 0xB9, 0x33, 0x44, \
     0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26, 0x99, 0xC7}

/* Function Prototypes */
int gpt_read_header(struct gpt_header *header);
int gpt_find_fat32_partition(uint32_t *lba_start);

#endif
