#ifndef INITRD_H
#define INITRD_H

#include <stdint.h>
#include <stddef.h>

/* TAR header structure (POSIX ustar format) */
struct tar_header {
    char filename[100];
    char mode[8];
    char uid[8];
    char gid[8];
    char size[12];
    char mtime[12];
    char checksum[8];
    char typeflag;
    char linkname[100];
    char magic[6];
    char version[2];
    char uname[32];
    char gname[32];
    char devmajor[8];
    char devminor[8];
    char prefix[155];
    char padding[12];
} __attribute__((packed));

/* File entry in InitRD */
struct initrd_file {
    char name[256];
    uint64_t size;
    void *data;
};

/* Initialize InitRD with specific memory location */
int initrd_init_memory(uint64_t address, uint64_t size);

/* List files in a directory (NULL or "/" for root) */
void initrd_list_files(const char *path);

/* Check if a path (directory) exists */
int initrd_is_dir(const char *path);

/* Find file data */
struct initrd_file *initrd_find_file(const char *name);

/* Create a file (write new TAR header + data) */
int initrd_create_file(const char *name, const char *data, uint64_t len);

/* Create a directory */
int initrd_create_dir(const char *name);

/* Delete a file (simulated) */
int initrd_delete_file(const char *name);

/* Mount InitRD as VFS Node */
struct vfs_node;
struct vfs_node *initrd_mount_vfs(void);

#endif
