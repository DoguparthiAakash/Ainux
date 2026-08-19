#define MINIMP3_IMPLEMENTATION
#include "minimp3_ex.h"
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char** argv) {
    if (argc < 3) {
        printf("Usage: %s <input.mp3> <output.raw>\n", argv[0]);
        return 1;
    }

    FILE* fin = fopen(argv[1], "rb");
    if (!fin) {
        perror("Failed to open input");
        return 1;
    }

    fseek(fin, 0, SEEK_END);
    long size = ftell(fin);
    fseek(fin, 0, SEEK_SET);

    unsigned char* buf = malloc(size);
    fread(buf, 1, size, fin);
    fclose(fin);

    mp3dec_t mp3d;
    mp3dec_init(&mp3d);
    mp3dec_file_info_t info;
    
    // minimp3 provides a high-level function to decode the whole file
    int res = mp3dec_load_buf(&mp3d, buf, size, &info, 0, 0);
    free(buf);
    
    if (res != 0) {
        printf("Failed to decode mp3: %d\n", res);
        return 1;
    }

    printf("Decoded: %d hz, %d channels, %ld samples\n", info.hz, info.channels, (long)info.samples);

    FILE* fout = fopen(argv[2], "wb");
    if (!fout) {
        perror("Failed to open output");
        free(info.buffer);
        return 1;
    }

    fwrite(info.buffer, sizeof(mp3d_sample_t), info.samples, fout);
    fclose(fout);
    free(info.buffer);

    return 0;
}
