#ifndef VFS_H
#define VFS_H

#include <stdint.h>
#include <stddef.h>

#define VFS_FILE        0x01
#define VFS_DIRECTORY   0x02
#define VFS_CHAR_DEVICE 0x03
#define VFS_BLOCK_DEVICE 0x04
#define VFS_PIPE        0x05
#define VFS_SYMLINK     0x06
#define VFS_MOUNTPOINT  0x08

/* Open flags */
#define O_RDONLY    0x0000
#define O_WRONLY    0x0001
#define O_RDWR      0x0002
#define O_APPEND    0x0008
#define O_CREAT     0x0100
#define O_TRUNC     0x0200
#define O_EXCL      0x0400

/* Seek flags */
#define SEEK_SET    0
#define SEEK_CUR    1
#define SEEK_END    2

/* Maximum open file descriptors per process */
#define MAX_OPEN_FILES 64

struct vfs_node;

typedef uint64_t (*read_type_t)(struct vfs_node *, uint64_t, uint64_t, uint8_t *);
typedef uint64_t (*write_type_t)(struct vfs_node *, uint64_t, uint64_t, uint8_t *);
typedef void (*open_type_t)(struct vfs_node *);
typedef void (*close_type_t)(struct vfs_node *);
typedef struct w_dirent * (*readdir_type_t)(struct vfs_node *, uint32_t);
typedef struct vfs_node * (*finddir_type_t)(struct vfs_node *, const char *name);

typedef struct vfs_fs_ops {
    read_type_t read;
    write_type_t write;
    open_type_t open;
    close_type_t close;
    readdir_type_t readdir;
    finddir_type_t finddir;
    void (*mkdir)(struct vfs_node *, const char *, uint16_t);
    void (*create)(struct vfs_node *, const char *, uint16_t);
    void (*unlink)(struct vfs_node *, const char *);
} vfs_fs_ops_t;

typedef struct vfs_node {
    char name[128];
    uint32_t flags;
    uint32_t mask;
    uint32_t uid;
    uint32_t gid;
    uint32_t inode;
    uint64_t length; /* Size of file */
    vfs_fs_ops_t *ops;
    void *device; /* Private data for driver */
    struct vfs_node *ptr; /* Used for mount points and symlinks */
} vfs_node_t;

struct w_dirent {
    char name[128];
    uint32_t inode;
};

/* File stat structure */
struct vfs_stat {
    uint32_t mode;      /* File type and permissions */
    uint32_t uid;       /* Owner user ID */
    uint32_t gid;       /* Owner group ID */
    uint64_t size;      /* File size in bytes */
    uint64_t atime;     /* Last access time */
    uint64_t mtime;     /* Last modification time */
    uint64_t ctime;     /* Creation time */
    uint32_t inode;     /* Inode number */
};

/* File descriptor structure */
typedef struct file_descriptor {
    struct vfs_node *node;  /* The VFS node this fd points to */
    uint64_t offset;        /* Current read/write position */
    int flags;              /* Open flags */
    int refcount;           /* Reference count for dup/fork */
} file_descriptor_t;

/* Global Root */

/* Global Root */
extern vfs_node_t *fs_root;

/* VFS Initialization */
void vfs_init(void);

/* Mounting */
struct vfs_node *vfs_mount(const char *path, vfs_node_t *fs_root);

/* POSIX-like File Operations */
int vfs_open(const char *path, int flags);
int vfs_close(int fd);
int64_t vfs_read_fd(int fd, void *buffer, uint64_t size);
int64_t vfs_write_fd(int fd, const void *buffer, uint64_t size);
int64_t vfs_seek(int fd, int64_t offset, int whence);
int vfs_stat(const char *path, struct vfs_stat *st);
int vfs_mkdir(const char *path, uint16_t mode);
int vfs_create(const char *path, uint16_t mode);
int vfs_unlink(const char *path);

/* Node-based operations (internal) */
uint64_t vfs_read(vfs_node_t *node, uint64_t offset, uint64_t size, uint8_t *buffer);
uint64_t vfs_write(vfs_node_t *node, uint64_t offset, uint64_t size, uint8_t *buffer);
vfs_node_t *vfs_lookup(vfs_node_t *parent, const char *name);

/* Directory operations */
struct w_dirent *vfs_readdir(vfs_node_t *node, uint32_t index);
vfs_node_t *vfs_finddir(vfs_node_t *node, const char *name);

#endif
