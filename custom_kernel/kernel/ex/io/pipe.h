#ifndef PIPE_H
#define PIPE_H

#include <stdint.h>
#include <stddef.h>

/* Pipe buffer size */
#define PIPE_BUF_SIZE 4096

/* Pipe flags */
#define PIPE_READ  0x01
#define PIPE_WRITE 0x02

/* Pipe structure */
typedef struct pipe {
    uint8_t buffer[PIPE_BUF_SIZE];  /* Circular buffer */
    uint32_t read_pos;               /* Read position */
    uint32_t write_pos;              /* Write position */
    uint32_t count;                  /* Bytes in buffer */
    int readers;                     /* Number of readers */
    int writers;                     /* Number of writers */
    int read_fd;                     /* Read end file descriptor */
    int write_fd;                    /* Write end file descriptor */
    /* TODO: Add wait queue for blocking I/O */
} pipe_t;

/* Pipe operations */

/* Create a pipe and return file descriptors in pipefd[0] (read) and pipefd[1] (write) */
int pipe_create(int pipefd[2]);

/* Read from pipe */
int64_t pipe_read(pipe_t *p, void *buf, size_t count);

/* Write to pipe */
int64_t pipe_write(pipe_t *p, const void *buf, size_t count);

/* Close pipe end */
int pipe_close(pipe_t *p, int flags);

/* Check if pipe has data available */
int pipe_available(pipe_t *p);

/* Get pipe from file descriptor */
pipe_t *pipe_from_fd(int fd);

#endif
