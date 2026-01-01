#ifndef MP4_H
#define MP4_H

#include <stdint.h>

/* MP4 Atom/Box Header */
typedef struct {
    uint32_t size;
    char type[4];
} MP4_Atom;

/* Parsed MP4 Info */
typedef struct {
    uint32_t width;
    uint32_t height;
    uint32_t duration;
    uint32_t timescale;
    uint32_t frame_count;
    char codec[5]; /* e.g. "avc1" */
    int is_supported;
} MP4_Info;

/* Parse an MP4 file from memory buffer */
/* Returns 0 on success, < 0 on failure */
int mp4_parse(const uint8_t *data, uint32_t size, MP4_Info *info);

/* Helper to convert Big Endian to Host (Little Endian) */
uint32_t be32_to_le32(uint32_t he);

#endif
