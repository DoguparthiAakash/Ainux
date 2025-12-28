#ifndef ATA_H
#define ATA_H

#include <stdint.h>
#include <stddef.h>

/* Register Offsets are defined below */
/* We will use dynamic active_port + offset */

/* Commands */
#define ATA_CMD_IDENTIFY    0xEC
#define ATA_CMD_READ_PIO    0x20
#define ATA_CMD_WRITE_PIO   0x30

/* Register Offsets */
#define ATA_REG_DATA         0x00
#define ATA_REG_ERROR        0x01
#define ATA_REG_FEATURES     0x01
#define ATA_REG_SECTOR_COUNT 0x02
#define ATA_REG_LBA_LOW      0x03
#define ATA_REG_LBA_MID      0x04
#define ATA_REG_LBA_HIGH     0x05
#define ATA_REG_DRIVE_HEAD   0x06
#define ATA_REG_STATUS       0x07
#define ATA_REG_COMMAND      0x07

/* Status Register Bits */
#define ATA_SR_BSY      0x80    /* Busy */
#define ATA_SR_DRDY 0x40
#define ATA_SR_DRQ  0x08
#define ATA_SR_ERR  0x01

/* Functions */
void ata_init(void);
int ata_read_sectors(uint32_t lba, uint8_t count, uint8_t *buffer);
int ata_write_sectors(uint32_t lba, uint8_t count, const uint8_t *buffer);

#endif
