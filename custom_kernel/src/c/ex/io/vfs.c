#include "vfs.h"
#include "../libc/string.h"
#include "../libc/stdlib.h"
#include "../libc/stdio.h"
#include "../mm/heap.h"
#include "../../ke/sched/sched.h" /* For current_task */

extern void kprint(const char *msg);

vfs_node_t *fs_root = 0;

static int str_cmp(const char *s1, const char *s2) {
    while (*s1 && *s2 && *s1 == *s2) { s1++; s2++; }
    return *s1 - *s2;
}
static void str_cpy(char *d, const char *s) {
    while(*s) *d++ = *s++;
    *d = 0;
}

void vfs_init(void) {
    fs_root = (vfs_node_t *)kmalloc(sizeof(vfs_node_t));
    memset(fs_root, 0, sizeof(vfs_node_t));
    strcpy(fs_root->name, "/");
    fs_root->flags = VFS_DIRECTORY;
    
    printf("[VFS] Initialized.\n");
}

/* Allocate a file descriptor in current process */
static int alloc_fd(void) {
    struct task_struct *current = sched_get_current();
    if (!current) return -1;
    
    for (int i = 3; i < TASK_MAX_FDS; i++) { /* 0-2 reserved */
        if (current->fd_table[i] == 0) {
            return i;
        }
    }
    return -1; /* No free fd in process */
}

/* Open a file and return file descriptor */
int vfs_open(const char *path, int flags) {
    if (!path || !fs_root) return -1;
    
    /* Handle creation flag */
    if (flags & O_CREAT) {
        /* TODO: Split path into parent and filename */
        /* For now, simplified: Assume file doesn't exist or we check */
        /* This logic needs a way to create the node if missing */
        /* vfs_create(path, 0); */
    }
    
    /* Find the node */
    vfs_node_t *node = vfs_lookup(fs_root, path);
    
    /* Auto-create if not found and requested */
    if (!node && (flags & O_CREAT)) {
        if (vfs_create(path, 0) == 0) {
            node = vfs_lookup(fs_root, path);
        }
    }
    
    if (!node) return -1;
    
    /* Allocate fd in process */
    int fd = alloc_fd();
    if (fd < 0) return -1;
    
    /* Allocate file descriptor structure */
    file_descriptor_t *desc = (file_descriptor_t *)kmalloc(sizeof(file_descriptor_t));
    if (!desc) return -1;
    
    desc->node = node;
    desc->offset = 0;
    desc->flags = flags;
    desc->refcount = 1;
    
    /* Assign to process */
    struct task_struct *current = sched_get_current();
    current->fd_table[fd] = (struct file_descriptor *)desc;
    
    /* Call node's open function if available */
    if (node->ops && node->ops->open) {
        node->ops->open(node);
    }
    
    return fd;
}

/* Close a file descriptor */
int vfs_close(int fd) {
    struct task_struct *current = sched_get_current();
    if (!current) return -1;
    
    if (fd < 0 || fd >= TASK_MAX_FDS) return -1;
    if (!current->fd_table[fd]) return -1;
    
    file_descriptor_t *desc = (file_descriptor_t *)current->fd_table[fd];
    desc->refcount--;
    
    if (desc->refcount <= 0) {
        /* Call node's close function if available */
        if (desc->node && desc->node->ops && desc->node->ops->close) {
            desc->node->ops->close(desc->node);
        }
        kfree(desc);
    }
    
    current->fd_table[fd] = 0;
    return 0;
}

/* Read from file descriptor */
int64_t vfs_read_fd(int fd, void *buffer, uint64_t size) {
    struct task_struct *current = sched_get_current();
    if (!current) return -1;

    if (fd < 0 || fd >= TASK_MAX_FDS) return -1;
    if (!current->fd_table[fd]) return -1;
    
    file_descriptor_t *desc = (file_descriptor_t *)current->fd_table[fd];
    if (!desc->node) return -1;
    
    if ((desc->flags & O_WRONLY) && !(desc->flags & O_RDWR)) {
        return -1; /* Write-only mode */
    }
    
    uint64_t bytes_read = vfs_read(desc->node, desc->offset, size, (uint8_t *)buffer);
    desc->offset += bytes_read;
    
    return bytes_read;
}

/* Write to file descriptor */
int64_t vfs_write_fd(int fd, const void *buffer, uint64_t size) {
    struct task_struct *current = sched_get_current();
    if (!current) return -1;

    if (fd < 0 || fd >= TASK_MAX_FDS) return -1;
    if (!current->fd_table[fd]) return -1;
    
    file_descriptor_t *desc = (file_descriptor_t *)current->fd_table[fd];
    if (!desc->node) return -1;
    
    if (!(desc->flags & O_WRONLY) && !(desc->flags & O_RDWR)) {
        return -1; /* Read-only mode */
    }
    
    if (desc->flags & O_APPEND) {
        desc->offset = desc->node->length;
    }
    
    uint64_t bytes_written = vfs_write(desc->node, desc->offset, size, (uint8_t *)buffer);
    desc->offset += bytes_written;
    
    return bytes_written;
}

/* Seek within file */
int64_t vfs_seek(int fd, int64_t offset, int whence) {
    struct task_struct *current = sched_get_current();
    if (!current) return -1;

    if (fd < 0 || fd >= TASK_MAX_FDS) return -1;
    if (!current->fd_table[fd]) return -1;
    
    file_descriptor_t *desc = (file_descriptor_t *)current->fd_table[fd];
    if (!desc->node) return -1;
    
    int64_t new_offset;
    
    switch (whence) {
        case SEEK_SET:
            new_offset = offset;
            break;
        case SEEK_CUR:
            new_offset = (int64_t)desc->offset + offset;
            break;
        case SEEK_END:
            new_offset = (int64_t)desc->node->length + offset;
            break;
        default:
            return -1;
    }
    
    if (new_offset < 0) return -1;
    
    desc->offset = (uint64_t)new_offset;
    return new_offset;
}

/* Get file status */
int vfs_stat(const char *path, struct vfs_stat *st) {
    if (!path || !st || !fs_root) return -1;
    
    vfs_node_t *node = vfs_lookup(fs_root, path);
    if (!node) return -1;
    
    st->mode = node->flags;
    st->uid = node->uid;
    st->gid = node->gid;
    st->size = node->length;
    st->inode = node->inode;
    st->atime = 0;
    st->mtime = 0;
    st->ctime = 0;
    
    return 0;
}

/* Mkdir */
int vfs_mkdir(const char *path, uint16_t mode) {
    // 1. Find parent directory
    // For simplicity, assume path is "/name" (root child) or "name" (cwd child)
    // Needs proper path parsing.
    
    // Quick hack: find parent by stripping last component
    // If path is "dir", parent is CWD (or root for now).
    // If path is "/dir", parent is root.
    
    vfs_node_t *parent;
    char *name;
    
    // Temporary: Only support root-level creation
    // TODO: Implement proper path splitting helper
    if (path[0] == '/') {
         parent = fs_root;
         name = (char*)path + 1;
    } else {
         parent = fs_root; // Fallback to root for relative
         name = (char*)path;
    }
    
    if (parent && parent->ops && parent->ops->mkdir) {
        parent->ops->mkdir(parent, name, mode);
        return 0;
    }
    return -1;
}

/* Create File */
int vfs_create(const char *path, uint16_t mode) {
    vfs_node_t *parent;
    char *name;
    
    // Temporary: Only support root-level creation
    if (path[0] == '/') {
         parent = fs_root;
         name = (char*)path + 1;
    } else {
         parent = fs_root; // Fallback to root
         name = (char*)path;
    }
    
    if (parent && parent->ops && parent->ops->create) {
        parent->ops->create(parent, name, mode);
        return 0;
    }
    return -1;
}

/* Unlink */
int vfs_unlink(const char *path) {
    vfs_node_t *parent;
    char *name;
    
    if (path[0] == '/') {
         parent = fs_root;
         name = (char*)path + 1;
    } else {
         parent = fs_root;
         name = (char*)path;
    }
    
    if (parent && parent->ops && parent->ops->unlink) {
        parent->ops->unlink(parent, name);
        return 0;
    }
    return -1;
}

/* Node-based read (internal) */
uint64_t vfs_read(vfs_node_t *node, uint64_t offset, uint64_t size, uint8_t *buffer) {
    if (node && node->ops && node->ops->read)
        return node->ops->read(node, offset, size, buffer);
    return 0;
}

/* Node-based write (internal) */
uint64_t vfs_write(vfs_node_t *node, uint64_t offset, uint64_t size, uint8_t *buffer) {
    if (node && node->ops && node->ops->write)
        return node->ops->write(node, offset, size, buffer);
    return 0;
}

/* Path lookup - traverse from parent to find node by path */
/* Mount Point Table */
#define MAX_MOUNTS 16
struct mount_point {
    char path[64];
    vfs_node_t *root;
} g_mounts[MAX_MOUNTS];

/* Helper to register mount */
static void register_mount(const char *path, vfs_node_t *root) {
    for(int i=0; i<MAX_MOUNTS; i++) {
        if (g_mounts[i].root == 0) {
            str_cpy(g_mounts[i].path, path);
            g_mounts[i].root = root;
            return;
        }
    }
}

/* Helper to check mounts */
static vfs_node_t *check_mount(const char *name) {
    /* Name is a single component e.g. "initrd" */
    /* We expect mount paths like "/initrd" */
    /* So we compare "/" + name */
    /* Or keep it simple: just compare name vs mount path (stripped) */
    for(int i=0; i<MAX_MOUNTS; i++) {
        if (g_mounts[i].root) {
            const char *mpath = g_mounts[i].path;
            if (mpath[0] == '/') mpath++;
            if (str_cmp(name, mpath) == 0) return g_mounts[i].root;
        }
    }
    return 0;
}

vfs_node_t *vfs_lookup(vfs_node_t *parent, const char *name) {
    if (!parent || !name) return 0;
    
    /* kprint("[VFS] Lookup Start: "); kprint(name); kprint("\n"); */
    
    /* Handle absolute path if parent is root */
    const char *path = name;
    vfs_node_t *curr = parent;
    
    if (path[0] == '/') {
        curr = fs_root;
        path++;
    }
    
    if (path[0] == '\0') return curr;
    
    char component[128];
    int i = 0; // path index
    
    while (path[i] != '\0') {
        int j = 0;
        while (path[i] != '/' && path[i] != '\0' && j < 127) {
            component[j++] = path[i++];
        }
        component[j] = '\0';
        
        if (path[i] == '/') i++;
        
        /* Check for Mount Point Redirect */
        vfs_node_t *mount_node = check_mount(component);
        if (mount_node) {
             /* kprint("[VFS] Mount Hit: "); kprint(component); kprint("\n"); */
             curr = mount_node;
             continue; 
        }
        
        /* kprint("[VFS] Comp: "); kprint(component); kprint("\n"); */
        
        /* Regular Lookup */
        if (curr->ops && curr->ops->finddir) {
            vfs_node_t *next = curr->ops->finddir(curr, component);
            if (!next) return 0;
            curr = next;
        } else {
            return 0;
        }
    }
    
    return curr;
}


/* Read directory entry */
struct w_dirent *vfs_readdir(vfs_node_t *node, uint32_t index) {
    if (node && node->ops && node->ops->readdir) {
        return node->ops->readdir(node, index);
    }
    return 0;
}

/* Find directory entry */
vfs_node_t *vfs_finddir(vfs_node_t *node, const char *name) {
    if (node && node->ops && node->ops->finddir) {
        return node->ops->finddir(node, name);
    }
    return 0;
}

/* Placeholder for full functionality */
struct vfs_node *vfs_mount(const char *path, vfs_node_t *new_fs_root) {
    if (strcmp(path, "/") == 0) {
        fs_root = new_fs_root;
        printf("[VFS] Mounted root at /\n");
        return fs_root;
    }
    
    /* Register mount point */
    register_mount(path, new_fs_root);
    printf("[VFS] Mounted %s\n", path);
    
    return new_fs_root;
}
