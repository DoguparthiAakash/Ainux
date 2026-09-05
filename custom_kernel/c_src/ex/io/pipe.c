#include "pipe.h"
#include "vfs.h"
#include "../mm/heap.h"
#include "../libc/string.h"
#include "../../ke/sched/sched.h"

extern void kprint(const char *msg);

/* Maximum number of pipes */
#define MAX_PIPES 64

/* Global pipe table */
static pipe_t *pipe_table[MAX_PIPES] = {0};

/* VFS operations for pipes */
static uint64_t pipe_vfs_read(vfs_node_t *node, uint64_t offset, uint64_t size, uint8_t *buffer);
static uint64_t pipe_vfs_write(vfs_node_t *node, uint64_t offset, uint64_t size, uint8_t *buffer);
static void pipe_vfs_close(vfs_node_t *node);

static vfs_fs_ops_t pipe_ops = {
    .read = pipe_vfs_read,
    .write = pipe_vfs_write,
    .close = pipe_vfs_close,
    .open = 0,
    .readdir = 0,
    .finddir = 0
};

/* Find a free pipe slot */
static int alloc_pipe_slot(void) {
    for (int i = 0; i < MAX_PIPES; i++) {
        if (pipe_table[i] == 0) {
            return i;
        }
    }
    return -1;
}

/* Find free file descriptor in current process */
static int alloc_pipe_fd(void) {
    struct task_struct *current = sched_get_current();
    if (!current) return -1;
    
    for (int i = 3; i < TASK_MAX_FDS; i++) {
        if (current->fd_table[i] == 0) {
            return i;
        }
    }
    return -1;
}

int pipe_create(int pipefd[2]) {
    struct task_struct *current = sched_get_current();
    if (!current) return -1;

    /* Allocate pipe structure */
    pipe_t *p = (pipe_t *)kmalloc(sizeof(pipe_t));
    if (!p) {
        kprint("[PIPE] Out of memory\n");
        return -1;
    }
    
    memset(p, 0, sizeof(pipe_t));
    p->readers = 1;
    p->writers = 1;
    
    /* Find slot in pipe table */
    int slot = alloc_pipe_slot();
    if (slot < 0) {
        kfree(p);
        kprint("[PIPE] No free pipe slots\n");
        return -1;
    }
    pipe_table[slot] = p;
    
    /* Create VFS nodes for read and write ends */
    vfs_node_t *read_node = (vfs_node_t *)kmalloc(sizeof(vfs_node_t));
    vfs_node_t *write_node = (vfs_node_t *)kmalloc(sizeof(vfs_node_t));
    
    if (!read_node || !write_node) {
        if (read_node) kfree(read_node);
        if (write_node) kfree(write_node);
        kfree(p);
        pipe_table[slot] = 0;
        return -1;
    }
    
    memset(read_node, 0, sizeof(vfs_node_t));
    memset(write_node, 0, sizeof(vfs_node_t));
    
    strcpy(read_node->name, "pipe_read");
    read_node->flags = VFS_PIPE;
    read_node->device = p;
    read_node->ops = &pipe_ops;
    
    strcpy(write_node->name, "pipe_write");
    write_node->flags = VFS_PIPE;
    write_node->device = p;
    write_node->ops = &pipe_ops;
    
    /* Allocate file descriptors */
    int read_fd = alloc_pipe_fd();
    if (read_fd < 0) {
        kfree(read_node);
        kfree(write_node);
        kfree(p);
        pipe_table[slot] = 0;
        return -1;
    }
    
    /* Reserve first FD to prevent re-allocation */
    /* Hack: Temporarily mark it used, will overwrite later */
    /* Ideally alloc_pipe_fd should take a mask or start index */
    /* Since alloc_pipe_fd scans from beginning, we just need to scan again from read_fd + 1 */
    /* Or just manually loop */
    
    int write_fd = -1;
    for (int i = read_fd + 1; i < TASK_MAX_FDS; i++) {
        if (current->fd_table[i] == 0) {
            write_fd = i;
            break;
        }
    }
    
    if (write_fd < 0) {
        kfree(read_node);
        kfree(write_node);
        kfree(p);
        pipe_table[slot] = 0;
        return -1;
    }
    
    /* Create file descriptors */
    file_descriptor_t *read_desc = (file_descriptor_t *)kmalloc(sizeof(file_descriptor_t));
    file_descriptor_t *write_desc = (file_descriptor_t *)kmalloc(sizeof(file_descriptor_t));
    
    if (!read_desc || !write_desc) {
        if (read_desc) kfree(read_desc);
        if (write_desc) kfree(write_desc);
        kfree(read_node);
        kfree(write_node);
        kfree(p);
        pipe_table[slot] = 0;
        return -1;
    }
    
    read_desc->node = read_node;
    read_desc->offset = 0;
    read_desc->flags = O_RDONLY;
    read_desc->refcount = 1;
    
    write_desc->node = write_node;
    write_desc->offset = 0;
    write_desc->flags = O_WRONLY;
    write_desc->refcount = 1;
    
    current->fd_table[read_fd] = (struct file_descriptor *)read_desc;
    current->fd_table[write_fd] = (struct file_descriptor *)write_desc;
    
    p->read_fd = read_fd;
    p->write_fd = write_fd;
    
    pipefd[0] = read_fd;
    pipefd[1] = write_fd;
    
    kprint("[PIPE] Created pipe\n");
    
    return 0;
}

int64_t pipe_read(pipe_t *p, void *buf, size_t count) {
    if (!p || !buf) return -1;
    
    /* If no data and no writers, return 0 (EOF) */
    if (p->count == 0 && p->writers == 0) {
        return 0;
    }
    
    /* TODO: Block if no data and writers exist */
    /* For now, return what we have */
    
    uint8_t *dest = (uint8_t *)buf;
    size_t bytes_read = 0;
    
    while (bytes_read < count && p->count > 0) {
        dest[bytes_read] = p->buffer[p->read_pos];
        p->read_pos = (p->read_pos + 1) % PIPE_BUF_SIZE;
        p->count--;
        bytes_read++;
    }
    
    return bytes_read;
}

int64_t pipe_write(pipe_t *p, const void *buf, size_t count) {
    if (!p || !buf) return -1;
    
    /* If no readers, return error (SIGPIPE should be sent) */
    if (p->readers == 0) {
        return -1;
    }
    
    const uint8_t *src = (const uint8_t *)buf;
    size_t bytes_written = 0;
    
    while (bytes_written < count) {
        if (p->count >= PIPE_BUF_SIZE) {
            /* Buffer full */
            /* TODO: Block until space available */
            break;
        }
        
        p->buffer[p->write_pos] = src[bytes_written];
        p->write_pos = (p->write_pos + 1) % PIPE_BUF_SIZE;
        p->count++;
        bytes_written++;
    }
    
    return bytes_written;
}

int pipe_close(pipe_t *p, int flags) {
    if (!p) return -1;
    
    if (flags & PIPE_READ) {
        p->readers--;
    }
    if (flags & PIPE_WRITE) {
        p->writers--;
    }
    
    /* If no more readers or writers, free the pipe */
    if (p->readers <= 0 && p->writers <= 0) {
        /* Find and remove from table */
        for (int i = 0; i < MAX_PIPES; i++) {
            if (pipe_table[i] == p) {
                pipe_table[i] = 0;
                break;
            }
        }
        kfree(p);
        kprint("[PIPE] Pipe destroyed\n");
    }
    
    return 0;
}

int pipe_available(pipe_t *p) {
    if (!p) return 0;
    return p->count;
}

pipe_t *pipe_from_fd(int fd) {
    struct task_struct *current = sched_get_current();
    if (!current) return 0;

    if (fd < 0 || fd >= TASK_MAX_FDS) return 0;
    if (!current->fd_table[fd]) return 0;
    
    file_descriptor_t *desc = (file_descriptor_t *)current->fd_table[fd];
    vfs_node_t *node = desc->node;
    if (!node || node->flags != VFS_PIPE) return 0;
    
    return (pipe_t *)node->device;
}

/* VFS Callbacks */

static uint64_t pipe_vfs_read(vfs_node_t *node, uint64_t offset, uint64_t size, uint8_t *buffer) {
    (void)offset;
    pipe_t *p = (pipe_t *)node->device;
    if (!p) return 0;
    return pipe_read(p, buffer, size);
}

static uint64_t pipe_vfs_write(vfs_node_t *node, uint64_t offset, uint64_t size, uint8_t *buffer) {
    (void)offset;
    pipe_t *p = (pipe_t *)node->device;
    if (!p) return 0;
    return pipe_write(p, buffer, size);
}

static void pipe_vfs_close(vfs_node_t *node) {
    pipe_t *p = (pipe_t *)node->device;
    if (!p) return;
    
    /* Determine which end this is */
    if (strcmp(node->name, "pipe_read") == 0) {
        pipe_close(p, PIPE_READ);
    } else {
        pipe_close(p, PIPE_WRITE);
    }
}
