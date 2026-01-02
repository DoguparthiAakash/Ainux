#include "ext2.h"
#include "../libc/stdio.h"
#include "../libc/string.h"
#include "../libc/stdlib.h"
#include "../mm/heap.h"
#include "../drivers/ata.h"

static struct ext2_superblock sb;
static uint32_t block_size = 0;
static uint32_t partition_offset = 0; /* In Sectors */
static uint32_t bg_descriptors_count = 0;
static struct ext2_bg_descriptor *bg_descriptors = NULL;

/* Helpers */
static void read_block(uint32_t block_idx, uint8_t *buffer) {
    uint32_t sectors_per_block = block_size / 512;
    uint32_t lba = partition_offset + (block_idx * sectors_per_block);
    ata_read_sectors(lba, sectors_per_block, buffer);
}

static struct ext2_inode read_inode(uint32_t inode_idx) {
    struct ext2_inode inode;
    uint32_t bg_idx = (inode_idx - 1) / sb.inodes_per_group;
    uint32_t index_in_bg = (inode_idx - 1) % sb.inodes_per_group;
    
    uint32_t inode_table_block = bg_descriptors[bg_idx].inode_table;
    uint32_t block_offset = (index_in_bg * sb.inode_size) / block_size;
    uint32_t offset_in_block = (index_in_bg * sb.inode_size) % block_size;
    
    uint8_t *buf = kmalloc(block_size);
    read_block(inode_table_block + block_offset, buf);
    
    memcpy(&inode, buf + offset_in_block, sizeof(struct ext2_inode));
    kfree(buf);
    
    return inode;
}

/* VFS Operations */
static uint64_t ext2_read(vfs_node_t *node, uint64_t offset, uint64_t size, uint8_t *buffer) {
    /* TODO: Only supporting direct blocks for now */
    struct ext2_inode inode = read_inode(node->inode);
    
    uint64_t bytes_read = 0;
    uint32_t block_idx_in_file = offset / block_size;
    uint32_t offset_in_block = offset % block_size;
    
    while (bytes_read < size && bytes_read < inode.size) {
        if (block_idx_in_file >= 12) break; /* Indirect not impl */
        
        uint32_t phys_block = inode.block[block_idx_in_file];
        if (phys_block == 0) {
             /* Sparse hole - return 0s */
             buffer[bytes_read++] = 0;
             /* Optimize: skip whole block logic */
             offset_in_block++;
             if(offset_in_block >= block_size) {
                 offset_in_block = 0;
                 block_idx_in_file++;
             }
             continue;
        }
        
        uint8_t *blk_buf = kmalloc(block_size);
        read_block(phys_block, blk_buf);
        
        uint32_t chunk = block_size - offset_in_block;
        if (chunk > (size - bytes_read)) chunk = size - bytes_read;
        
        memcpy(buffer + bytes_read, blk_buf + offset_in_block, chunk);
        kfree(blk_buf);
        
        bytes_read += chunk;
        offset_in_block = 0;
        block_idx_in_file++;
    }
    return bytes_read;
}

static struct w_dirent *ext2_readdir(vfs_node_t *node, uint32_t index) {
    /* Iterating directory is complex in EXT2 (variable length entries) */
    /* Implementation deferred to full read logic */
    (void)node; (void)index;
    return NULL;
}

static vfs_node_t *ext2_finddir(vfs_node_t *node, const char *name) {
    struct ext2_inode inode = read_inode(node->inode);
    if (!((inode.mode & 0xF000) == 0x4000)) return NULL; /* Not directory */
    
    /* Search blocks */
    for (int i=0; i<12; i++) {
        if (inode.block[i] == 0) break;
        
        uint8_t *blk = kmalloc(block_size);
        read_block(inode.block[i], blk);
        
        uint32_t offset = 0;
        while (offset < block_size) {
            struct ext2_dir_entry *entry = (struct ext2_dir_entry *)(blk + offset);
            
            if (entry->inode != 0) {
                 char entry_name[256];
                 memcpy(entry_name, entry->name, entry->name_len);
                 entry_name[entry->name_len] = 0;
                 
                 if (strcmp(name, entry_name) == 0) {
                     /* Found! Create Node */
                     vfs_node_t *ret = kmalloc(sizeof(vfs_node_t));
                     memset(ret, 0, sizeof(vfs_node_t));
                     strcpy(ret->name, entry_name);
                     ret->inode = entry->inode;
                     
                     /* Read Inode to get type/size */
                     struct ext2_inode t_ino = read_inode(entry->inode);
                     ret->length = t_ino.size;
                     
                     if ((t_ino.mode & 0xF000) == 0x4000) {
                         ret->flags = VFS_DIRECTORY;
                     } else {
                         ret->flags = VFS_FILE;
                     }
                     
                     /* Inherit Ops */
                     ret->ops = kmalloc(sizeof(vfs_fs_ops_t));
                     ret->ops->read = ext2_read;
                     ret->ops->finddir = ext2_finddir;
                     /* ... */
                     
                     kfree(blk);
                     return ret;
                 }
            }
            offset += entry->rec_len;
        }
        kfree(blk);
    }
    return NULL;
}

vfs_node_t *ext2_mount(uint32_t offset_sectors) {
    partition_offset = offset_sectors;
    
    /* 1. Read Superblock (Block 1, or offset 1024 bytes) */
    /* If block size 1024, it's block 1. If larger, it's inside block 0. */
    /* Generally safe to read Sector 2 of partition (Sector 0=Boot, 1=SB potentially if 1024, or fixed 1024 byte offset) */
    /* 1024 bytes offset = Sector 2 (512*2) */
    
    uint8_t *buf = kmalloc(1024);
    ata_read_sectors(partition_offset + 2, 2, buf); /* Read 1024 bytes */
    
    memcpy(&sb, buf, sizeof(struct ext2_superblock));
    kfree(buf);
    
    if (sb.magic != EXT2_MAGIC) {
        printf("[EXT2] Invalid Magic: %x\n", sb.magic);
        return NULL;
    }
    
    block_size = 1024 << sb.log_block_size;
    printf("[EXT2] Mounted. Ver: %d.%d, BlockSize: %d\n", sb.rev_level, sb.minor_rev_level, block_size);
    
    /* 2. Read Block Group Descriptors */
    bg_descriptors_count = sb.blocks_count / sb.blocks_per_group;
    if (sb.blocks_count % sb.blocks_per_group) bg_descriptors_count++;
    
    /* BGD Table starts at Block 2 (if 1k blocks) or Block 1 (if >1k) */
    uint32_t bgd_block = (block_size == 1024) ? 2 : 1;
    
    uint32_t bgd_size_bytes = bg_descriptors_count * sizeof(struct ext2_bg_descriptor);
    uint32_t bgd_sectors = (bgd_size_bytes + 511) / 512;
    
    bg_descriptors = kmalloc(bgd_size_bytes);
    /* Read sectors for BGD. Naive loop or huge read. */
    /* Need proper block->sector conversion for BGD location */
    /* BGD is contiguous? Yes usually. */
    /* Let's assume contiguous for now */
    
    /* Calculation: BGD starts at partition_offset + (bgd_block * sectors_per_block) */
    uint32_t sectors_per_block = block_size / 512;
    uint32_t bgd_start_lba = partition_offset + (bgd_block * sectors_per_block);
    
    ata_read_sectors(bgd_start_lba, bgd_sectors, (uint8_t*)bg_descriptors);
    
    /* 3. Create Root Node */
    vfs_node_t *root = kmalloc(sizeof(vfs_node_t));
    memset(root, 0, sizeof(vfs_node_t));
    strcpy(root->name, "/");
    root->inode = 2; /* Root Inode is 2 */
    root->flags = VFS_DIRECTORY;
    
    /* Ops */
    root->ops = kmalloc(sizeof(vfs_fs_ops_t));
    memset(root->ops, 0, sizeof(vfs_fs_ops_t));
    root->ops->read = ext2_read;
    root->ops->finddir = ext2_finddir;
    
    return root;
}
