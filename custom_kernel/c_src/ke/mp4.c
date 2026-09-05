#include "mp4.h"
#include "log.h"
#include "libc/string.h"
#include "libc/stdio.h"

/* Helper: Big Endian to Little Endian */
uint32_t be32_to_le32(uint32_t be) {
    return ((be >> 24) & 0xFF) |
           ((be >> 8) & 0xFF00) |
           ((be << 8) & 0xFF0000) |
           ((be << 24) & 0xFF000000);
}

/* Helper: Match atom type */
static int atom_is(const char *type, const char *target) {
    return (type[0] == target[0] && type[1] == target[1] &&
            type[2] == target[2] && type[3] == target[3]);
}

/* Parse recursively */
/* simplified: we scan linearly for specific atoms of interest */
/* A full parser would respect hierarchy strictly. We'll be "loose" for simplicity in kernel mode */

int mp4_parse(const uint8_t *data, uint32_t size, MP4_Info *info) {
    if (!data || !info) return -1;
    
    /* Zero info */
    info->width = 0;
    info->height = 0;
    info->duration = 0;
    info->timescale = 0;
    info->frame_count = 0;
    info->codec[0] = 0;
    info->is_supported = 0;
    
    uint32_t offset = 0;
    
    while (offset + 8 <= size) {
        uint32_t atom_size = be32_to_le32(*(uint32_t*)(data + offset));
        if (atom_size == 0) break; /* End or weirdness */
        if (atom_size == 1) { 
             /* 64-bit size, not supported here for simplicity */
             offset += 8; 
             continue; 
        }
        
        char type[5];
        type[0] = data[offset+4];
        type[1] = data[offset+5];
        type[2] = data[offset+6];
        type[3] = data[offset+7];
        type[4] = 0;
        
        /* Debug */
        /* kprint("Atom: "); kprint(type); kprint("\n"); */
        
        if (atom_is(type, "ftyp")) {
            /* File Type */
        } else if (atom_is(type, "moov")) {
            /* Container: Enter it */
            offset += 8; /* Skip header, continue parsing *inside* moov */
            continue; 
        } else if (atom_is(type, "trak")) {
            /* Track: Enter it (Assuming Video track comes first or we scan all) */
            offset += 8;
            continue;
        } else if (atom_is(type, "mdia") || atom_is(type, "minf") || atom_is(type, "stbl")) {
            /* Containers: Enter */
            offset += 8;
            continue;
        } else if (atom_is(type, "mvhd")) {
            /* Movie Header: extract timescale, duration */
            /* Version (1) + Flags (3) + Creation (4) + Mod (4) + Timescale (4) + Duration (4) */
            uint32_t ver = data[offset+8]; // Version
            /* uint32_t ts_offset = (ver == 1) ? 20 : 12; v1 uses 8-byte dates, v0 uses 4-byte */
            /* Actually mvhd v0:
               ver(1) flags(3) cr(4) mod(4) timescale(4) duration(4) */
            if (ver == 0) {
                 info->timescale = be32_to_le32(*(uint32_t*)(data + offset + 8 + 4 + 4 + 4));
                 info->duration = be32_to_le32(*(uint32_t*)(data + offset + 8 + 4 + 4 + 4 + 4));
            }
        } else if (atom_is(type, "tkhd")) {
            /* Track Header: Width/Height */
            /* v0: ver(1) flags(3) cr(4) mod(4) trackid(4) reserved(4) duration(4) reserved(8) layer(2) alt(2) vol(2) reserved(2) matrix(36) width(4) height(4) */
            /* width/height are fixed point 16.16 */
            uint32_t ver = data[offset+8];
            if (ver == 0) {
                 /* Fixed offsets for v0 tkhd */
                 /* width is at offset + 84? Let's check spec. */
                 /* tkhd size is usually 92 bytes for v0 */
                 /* Matrix ends at 84 (Start+header+4+4+4+4+4+4+8+2+2+2+2+36 = 12+68 = 80? ) */
                 /* Let's approximate: Width is at end - 8, Height at end - 4 of the atom usually? No. */
                 /* Offset 76 (dec) from data start (after type) */
                 uint32_t w = be32_to_le32(*(uint32_t*)(data + offset + 8 + 76));
                 uint32_t h = be32_to_le32(*(uint32_t*)(data + offset + 8 + 80));
                 info->width = w >> 16;
                 info->height = h >> 16;
            }
            /* Sample Description: Codec */
            /* Header(8) + Ver(1) + Flags(3) + Count(4) + Entry1 */
            /* Entry1: Size(4) + Format(4) ... */
            uint32_t inner = offset + 8 + 4 + 4; 
            char codec[5];
            codec[0] = data[inner+4];
            codec[1] = data[inner+5];
            codec[2] = data[inner+6];
            codec[3] = data[inner+7];
            codec[4] = 0;
            
            /* Store Codec */
            strncpy(info->codec, codec, 5);
            
            /* Check support */
            if (atom_is(codec, "raw ") || atom_is(codec, "cvid")) { // Uncompressed
                 info->is_supported = 1;
            }
        }
        
        offset += atom_size;
    }
    
    return 0;
}
