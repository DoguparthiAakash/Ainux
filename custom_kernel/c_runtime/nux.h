#ifndef NUX_H
#define NUX_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <unistd.h>
#include <math.h>

// Nux Types
typedef int64_t nux_int;
typedef double nux_float;
typedef bool nux_bool;
typedef void* nux_handle;

// Arrays (Simplified for C)
typedef struct {
    nux_int capacity;
    nux_int length;
    nux_int* data;
} nux_array;

// Graphics Buffer
typedef struct {
    int width;
    int height;
    uint32_t* pixels;
} nux_image;

// Standard Intrinsics
void print(const char* msg);
void print_int(nux_int val);

// Array Intrinsics
nux_array* Array_new(nux_int capacity);
void Array_set(nux_array* arr, nux_int index, nux_int val);
nux_int Array_get(nux_array* arr, nux_int index);
nux_int Array_len(nux_array* arr);

// Vision/Graphics Intrinsics
nux_image* img_alloc(nux_int width, nux_int height);
void img_set(nux_image* img, nux_int x, nux_int y, nux_int color);
void img_fill(nux_image* img, nux_int color);
void img_draw(nux_image* img, nux_int x, nux_int y);

// Input Intrinsics
bool is_key_down(nux_int key_code);

// Runtime Management
void nux_init();
void nux_main_loop_step(); // For keeping window alive
void nux_sleep(int ms);

#endif
