#include "axfs.h"
#include "drivers/ata.h"
#include "libc/string.h"
#include "libc/stdio.h"
#include "mm/heap.h"
#include "log.h"

/* Hardcoded Format Offset: 100MB (Sector 204800) to avoid FAT32 collision */
#define AXFS_START_LBA  204800
#define AXFS_TOTAL_BLOCKS 10000 /* 40MB for test */

/* Helpers */
static int write_block(uint64_t block_id, void *data) {
    /* Translate FS Block (4096) to ATA Sectors (512) */
    /* 1 Block = 8 Sectors */
    uint32_t start_lba = AXFS_START_LBA + (block_id * 8);
    return ata_write_sectors(start_lba, 8, (uint8_t*)data);
}

int axfs_format(void) {
    kprint("[AxFS] Formatting Partition (Start LBA: 204800)...\n");

    uint8_t *buffer = (uint8_t*)kmalloc(AXFS_BLOCK_SIZE);
    if (!buffer) return -1;

    /* 1. Clear Superblock area */
    memset(buffer, 0, AXFS_BLOCK_SIZE);
    write_block(0, buffer);

    /* 2. Setup Superblock */
    axfs_superblock_t *sb = (axfs_superblock_t*)buffer;
    sb->magic = AXFS_MAGIC;
    sb->version = AXFS_VERSION;
    sb->total_blocks = AXFS_TOTAL_BLOCKS;
    sb->free_blocks = AXFS_TOTAL_BLOCKS - 2; /* SB + Root */
    sb->root_node = 1; /* Root DNode at Block 1 */
    sb->bitmap_block = 2; /* Bitmap at Block 2 */
    sb->block_size = AXFS_BLOCK_SIZE;
    strcpy(sb->label, "Ainux_Data_Pool");
    
    /* Write Superblock */
    if (write_block(0, buffer) != 0) {
        kprint("[AxFS] Failed to write Superblock!\n");
        kfree(buffer);
        return -1;
    }

    /* 3. Setup Root DNode (Directory) */
    memset(buffer, 0, AXFS_BLOCK_SIZE);
    axfs_node_t *root = (axfs_node_t*)buffer;
    root->id = 1;
    root->type = VFS_DIRECTORY;
    root->perms = 0755;
    root->size = 0;
    root->created = 0; /* TODO: RTC */
    root->flags = 1;   /* Inline Extents */
    
    /* Write Root Node */
    if (write_block(1, buffer) != 0) {
        kprint("[AxFS] Failed to write Root Node!\n");
        kfree(buffer);
        return -1;
    }

    /* 4. Setup Allocation Bitmap (Empty) */
    /* Mark Block 0 (SB), 1 (Root), 2 (Bitmap itself) as used */
    memset(buffer, 0, AXFS_BLOCK_SIZE);
    buffer[0] = 0x07; /* Binary 00000111 -> Blocks 0, 1, 2 used */
    
    /* Write Bitmap */
    write_block(2, buffer);

    kfree(buffer);
    kprint("[AxFS] Format Complete. 40MB Volume Created.\n");
    return 0;
}
