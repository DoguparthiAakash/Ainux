#ifndef MBR_H
#define MBR_H

#include <stdint.h>

struct mbr_entry {
    uint8_t  status;
    uint8_t  start_head;
    uint8_t  start_sector;
    uint8_t  start_cylinder;
    uint8_t  type;
    uint8_t  end_head;
    uint8_t  end_sector;
    uint8_t  end_cylinder;
    uint32_t lba_start;
    uint32_t lba_length;
} __attribute__((packed));

struct mbr {
    uint8_t  bootstrap[446];
    struct mbr_entry partitions[4];
    uint16_t signature;
} __attribute__((packed));

void mbr_read(struct mbr *buffer);
void mbr_write_default(void);
void mbr_print_map(void);
int mbr_new_partition(uint32_t size_mb);

#endif
