#ifndef AXFS_H
#define AXFS_H

#include <stdint.h>
#include "vfs.h"

/* 
 * Ainux File System (AxFS)
 * A modern, extent-based, copy-on-write friendly filesystem.
 * Inspired by Linux Ext4 (Extents) and ZFS (Object-based Dnodes).
 * 
 * Goals: High Performance, Large File Support, Memory Efficiency.
 */

#define AXFS_MAGIC 0x41584653 /* "AXFS" */
#define AXFS_VERSION 1
#define AXFS_BLOCK_SIZE 4096

/* 1. Superblock: Global FS State */
typedef struct {
    uint32_t magic;         /* AXFS_MAGIC */
    uint32_t version;       /* AXFS_VERSION */
    uint64_t total_blocks;  /* FS Size */
    uint64_t free_blocks;   /* Free Space */
    uint64_t root_node;     /* Block ID of Root Directory DNode */
    uint64_t bitmap_block;  /* Block ID of Allocation Bitmap */
    uint32_t block_size;    /* 4096 normally */
    char label[32];         /* Volume Name */
    uint8_t uuid[16];       /* Unique ID */
    uint64_t generation;    /* Transaction ID (for COW future) */
} axfs_superblock_t;

/* 2. Extents: Efficient Data Mapping (Space/Time Optimized)
 * Instead of block lists (FAT: O(N), Inode: O(tree)), we use Extents.
 * A 1GB contiguous file needs ONLY ONE extent entry.
 * Time Complexity: O(1) for sequential usage.
 * Space Complexity: 16 bytes per fragmentation.
 */
typedef struct {
    uint64_t file_offset;   /* Logical Offset in file */
    uint64_t start_block;   /* Physical Block on Disk */
    uint64_t length;        /* Number of Blocks */
} axfs_extent_t;

/* 3. DNode: Generic Object (Inode equivalent) 
 * Can represent File, Directory, Symlink.
 * Uses Inline Extents for small/contiguous files (fast access).
 * Falls back to B-Tree for fragmented files.
 */
#define AXFS_INLINE_EXTENTS 4

typedef struct {
    uint32_t id;            /* Object ID */
    uint32_t type;          /* File Type (File, Dir) */
    uint32_t perms;         /* Permissions */
    uint64_t size;          /* File Size */
    uint64_t created;       /* Timestamp */
    uint64_t modified;      /* Timestamp */
    
    uint32_t extent_count;  /* Number of extents */
    uint32_t flags;         /* (1=Inline, 2=BTree Root) */

    /* Data Map */
    union {
        axfs_extent_t inline_extents[AXFS_INLINE_EXTENTS]; /* Fast Path */
        uint64_t btree_root;                               /* Slow Path (Large/Fragmented) */
    } data;
    
    uint8_t padding[64];    /* Align to power of 2 */
} axfs_node_t;

/* 4. Directory Entry */
typedef struct {
    uint32_t node_id;       /* DNode ID */
    uint16_t type;          /* Cache type for ls optimization */
    uint16_t name_len;
    char name[64];          /* Fixed size for simplicity or inline */
} axfs_dirent_t;

/* API */
int axfs_init(void);
vfs_node_t *axfs_mount_vfs(void);
int axfs_format(void); /* Create empty FS */

#endif
