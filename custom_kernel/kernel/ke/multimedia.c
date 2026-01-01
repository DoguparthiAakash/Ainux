#include "shell.h"
#include "drivers/timer.h"
#include "drivers/gfx/gfx.h"
#include "drivers/keyboard.h"
#include "drivers/mouse.h"
#include "libc/stdio.h"
#include "libc/stdlib.h"
#include "libc/string.h"
#include "ex/io/vfs.h"
#include "io/initrd.h"
#include "io/fat32.h"
#include "mm/heap.h"
#include "cursor_icon.h"
#include "mp4.h"
#include "log.h"

extern void kprint(const char *msg);
extern void term_clear(void);
extern char cwd[256];

/* --- Local Graphics Context --- */
static uint32_t *framebuffer = NULL;
static uint64_t screen_w = 0;
static uint64_t screen_h = 0;
static uint64_t screen_pitch = 0; 
static uint32_t *frontbuffer_addr = NULL;

static void put_pixel_buf(int x, int y, uint32_t color) {
    if (x < 0 || x >= (int)screen_w || y < 0 || y >= (int)screen_h) return;
    int idx = y * screen_w + x; 
    framebuffer[idx] = color;
}

static uint32_t get_pixel_buf(int x, int y) {
    if (x < 0 || x >= (int)screen_w || y < 0 || y >= (int)screen_h) return 0;
    return framebuffer[y * screen_w + x];
}

static void draw_rect_buf(int x, int y, int w, int h, uint32_t color) {
    if (x >= (int)screen_w || y >= (int)screen_h) return;
    if (x + w < 0 || y + h < 0) return;
    int cx = x, cy = y, cw = w, ch = h;
    if (cx < 0) { cw += cx; cx = 0; }
    if (cy < 0) { ch += cy; cy = 0; }
    if (cx + cw > (int)screen_w) cw = (int)screen_w - cx;
    if (cy + ch > (int)screen_h) ch = (int)screen_h - cy;
    for (int j = 0; j < ch; j++) {
        uint32_t *row = &framebuffer[(cy + j) * screen_w + cx];
        for (int i = 0; i < cw; i++) {
            row[i] = color;
        }
    }
}

static void draw_frame_buf(int x, int y, int w, int h, int active) {
    uint32_t border = 0x000000;
    uint32_t title_bg = active ? 0x000080 : 0x404040;
    uint32_t body_bg  = 0xC0C0C0;
    draw_rect_buf(x, y, w, h, body_bg);
    draw_rect_buf(x, y, w, 1, border);
    draw_rect_buf(x, y + h - 1, w, 1, border);
    draw_rect_buf(x, y, 1, h, border);
    draw_rect_buf(x + w - 1, y, 1, h, border);
    draw_rect_buf(x + 1, y + 1, w - 2, 20, title_bg);
}

static void draw_cursor_buf(int mx, int my) {
    mx -= CURSOR_X_HOT;
    my -= CURSOR_Y_HOT;
    for (int r = 0; r < CURSOR_HEIGHT; r++) {
        for (int c = 0; c < CURSOR_WIDTH; c++) {
            int screen_x = mx + c;
            int screen_y = my + r;
            if (screen_x < 0 || screen_x >= (int)screen_w || screen_y < 0 || screen_y >= (int)screen_h) continue;
            uint32_t src_pixel = cursor_icon[r * CURSOR_WIDTH + c];
            uint32_t alpha = (src_pixel >> 24) & 0xFF;
            if (alpha == 0) continue; 
            if (alpha == 255) {
                put_pixel_buf(screen_x, screen_y, src_pixel & 0xFFFFFF);
            } else {
                uint32_t bg_pixel = get_pixel_buf(screen_x, screen_y);
                uint32_t rb = bg_pixel & 0xFF00FF;
                uint32_t g  = bg_pixel & 0x00FF00;
                uint32_t s_rb = src_pixel & 0xFF00FF;
                uint32_t s_g  = src_pixel & 0x00FF00;
                uint32_t d_rb = (s_rb * alpha + rb * (255 - alpha)) >> 8;
                uint32_t d_g  = (s_g  * alpha + g  * (255 - alpha)) >> 8;
                put_pixel_buf(screen_x, screen_y, (d_rb & 0xFF00FF) | (d_g & 0x00FF00));
            }
        }
    }
}

static uint8_t *read_file_to_buffer(const char *filename, uint32_t *size_out) {
    char full_path[256];
    if (filename[0] == '/') {
        strcpy(full_path, filename);
    } else {
        strcpy(full_path, cwd);
        if (strcmp(cwd, "/") != 0) strcat(full_path, "/");
        strcat(full_path, filename);
    }
    struct initrd_file *file = initrd_find_file(full_path);
    if (!file && full_path[0] == '/') file = initrd_find_file(full_path + 1);
    if (file) {
        *size_out = file->size;
        uint8_t *buf = (uint8_t*)kmalloc(file->size);
        if (buf) memcpy(buf, file->data, file->size);
        return buf;
    }
    uint8_t *data;
    uint32_t size;
    if (fat32_read_file(filename, &data, &size) == 0) {
        *size_out = size;
        return data; 
    }
    return NULL;
}

/* --- Structures --- */
typedef struct __attribute__((packed)) {
    uint16_t bfType;
    uint32_t bfSize;
    uint16_t bfReserved1;
    uint16_t bfReserved2;
    uint32_t bfOffBits;
} BITMAPFILEHEADER;

typedef struct __attribute__((packed)) {
    uint32_t biSize;
    int32_t  biWidth;
    int32_t  biHeight;
    uint16_t biPlanes;
    uint16_t biBitCount;
    uint32_t biCompression;
    uint32_t biSizeImage;
    int32_t  biXPelsPerMeter;
    int32_t  biYPelsPerMeter;
    uint32_t biClrUsed;
    uint32_t biClrImportant;
} BITMAPINFOHEADER;

/* --- Video Engine --- */
typedef struct __attribute__((packed)) {
    char signature[4]; /* "VID1" */
    uint16_t width;
    uint16_t height;
    uint32_t frames;
    uint16_t fps;
} VideoHeader;

void cmd_view(char *filename) {
    if (!filename) { kprint("Usage: view <file>\n"); return; }
    
    void *addr;
    gfx_get_info(&screen_w, &screen_h, &screen_pitch, &addr);
    frontbuffer_addr = (uint32_t *)addr;
    
    framebuffer = (uint32_t *)kmalloc(screen_w * screen_h * 4);
    if (!framebuffer) { kprint("OOM!\n"); return; }
    
    int is_video = 0;
    int len = strlen(filename);
    if (len > 4 && (strcmp(filename + len - 4, ".vid") == 0 || strcmp(filename + len - 4, ".mp4") == 0)) is_video = 1;
    
    uint8_t *file_data = NULL;
    uint32_t file_size = 0;
    int img_w=0, img_h=0, img_bpp=0;
    uint8_t *img_pixels = NULL;
    
    /* Video Props */
    VideoHeader *vid_hdr = NULL;
    uint8_t *vid_frames = NULL;
    int vid_frame_size = 0;
    
    file_data = read_file_to_buffer(filename, &file_size);
    if (!file_data) {
        /* Fallback for "video.mp4" not found -> check if user meant .vid */
        if (is_video && strcmp(filename, "video.mp4") == 0) {
             /* Try video.vid instead (Generated) */
             file_data = read_file_to_buffer("video.vid", &file_size);
        }
        if (!file_data) {
            kprint("File not found.\n");
            kfree(framebuffer);
            return;
        }
    }
    
    if (!is_video) {
        BITMAPFILEHEADER *bf = (BITMAPFILEHEADER*)file_data;
        BITMAPINFOHEADER *bi = (BITMAPINFOHEADER*)(file_data + sizeof(BITMAPFILEHEADER));
        if (bf->bfType == 0x4D42) {
            img_w = bi->biWidth; img_h = bi->biHeight; 
            img_bpp = bi->biBitCount / 8;
            img_pixels = file_data + bf->bfOffBits;
        } else {
             kprint("Not a BMP.\n"); kfree(file_data); kfree(framebuffer); return;
        }
    } else {
        /* Check VID1 Header */
        if (file_size > sizeof(VideoHeader) && file_data[0] == 'V' && file_data[1] == 'I' && file_data[2] == 'D' && file_data[3] == '1') {
            vid_hdr = (VideoHeader*)file_data;
            img_w = vid_hdr->width;
            img_h = vid_hdr->height;
            img_bpp = 3; /* RGB888 */
            vid_frames = file_data + sizeof(VideoHeader);
            vid_frame_size = img_w * img_h * 3;
            if (vid_hdr->frames * vid_frame_size + sizeof(VideoHeader) > file_size) {
                 kprint("Corrupt Video File (Size Check Failed).\n"); kfree(file_data); kfree(framebuffer); return;
            }

/* ... existing code ... */ /* handled by context usually */

/* In cmd_view: */
        } else {
             /* Check if it was .mp4 that failed VID1 check, try fallback to video.vid */
             if (len > 4 && strcmp(filename + len - 4, ".mp4") == 0) {
                 
                 /* Try to PARSE MP4 Metadata first */
                 MP4_Info mp4_info;
                 kprint("[MP4] Parsing container...\n");
                 if (mp4_parse(file_data, file_size, &mp4_info) == 0) {
                      char buf[32];
                      kprint("[MP4] Info:\n");
                      kprint(" - Resolution: "); itoa(mp4_info.width, buf, 10); kprint(buf); 
                      kprint("x"); itoa(mp4_info.height, buf, 10); kprint(buf); kprint("\n");
                      
                      kprint(" - Codec: "); kprint(mp4_info.codec); kprint("\n");
                      
                      if (mp4_info.timescale > 0) {
                          uint32_t sec = mp4_info.duration / mp4_info.timescale;
                          kprint(" - Duration: "); itoa(sec, buf, 10); kprint(buf); kprint("s\n");
                      }
                      
                      if (!mp4_info.is_supported) {
                           kprint_color(KLOG_COLOR_RED, "[MP4] Error: Codec not supported. Attempting fallback...\n");
                      }
                 } else {
                      kprint("[MP4] Parse Failed. Attempting fallback...\n");
                 }
                 
                 kfree(file_data);
                 file_data = read_file_to_buffer("libs/video.vid", &file_size);
                 if (!file_data) file_data = read_file_to_buffer("video.vid", &file_size); /* Try root too */
                 
                 if (file_data && file_size > sizeof(VideoHeader) && file_data[0] == 'V' && file_data[1] == 'I') {
                      vid_hdr = (VideoHeader*)file_data;
                      img_w = vid_hdr->width;
                      img_h = vid_hdr->height;
                      img_bpp = 3;
                      vid_frames = file_data + sizeof(VideoHeader);
                      vid_frame_size = img_w * img_h * 3;
                 } else {
                      kprint("Format not supported (MP4 decode unavailable, fallback not found).\n");
                      if(file_data) kfree(file_data); kfree(framebuffer); return;
                 }
             } else {
                 kprint("Format not supported (Need .vid)\n");
                 kfree(file_data); kfree(framebuffer); return;
             }
        }
    }
    
    int win_w = screen_w - 40;
    int win_h = screen_h - 60;
    int win_x = 20;
    int win_y = 30;
    int zoom = 100;
    int pan_x = (win_w - 20 - 30 - img_w) / 2;
    int pan_y = (win_h - 40 - img_h) / 2;
    if (pan_x < 0) pan_x = 0; if (pan_y < 0) pan_y = 0;
    
    MouseState initial_ms = mouse_get_state();
    int last_scroll_z = initial_ms.scroll_z;
    uint32_t frame_idx = 0;
    int vid_playing = 1; 
    
    int dragging_win = 0, dragging_pan = 0, dragging_zoom = 0;
    int drag_ox = 0, drag_oy = 0, drag_sx = 0, drag_sy = 0;
    uint32_t desktop_color = 0x101010; /* Dark Mode */
    int exit_loop = 0;
    
    while(!exit_loop) {
        for(size_t i=0; i<screen_w*screen_h; i++) framebuffer[i] = desktop_color;
        /* Window Title Bar */
        draw_rect_buf(win_x, win_y, win_w, 30, 0x303030); 
        /* Ideally draw text "Media Player" here, but no text_buf function yet easily accessible without font ptr */
        draw_rect_buf(win_x, win_y+30, win_w, win_h-30, 0x101010); /* Inner Body */
        
        // draw_frame_buf(win_x, win_y, win_w, win_h, 1); // Replaced by above
        
        int zoom_bar_w = 30;
        int ctx = win_x + 10 + zoom_bar_w;
        int cty = win_y + 30;
        int ctw = win_w - 20 - zoom_bar_w;
        int cth = win_h - 40;
        
        int zbar_x = win_x + 10;
        int zbar_y = cty;
        int zbar_h = cth;
        draw_rect_buf(zbar_x, zbar_y, zoom_bar_w, zbar_h, 0x202020); /* Dark Bar BG */
        draw_rect_buf(zbar_x, zbar_y, zoom_bar_w, 1, 0x404040);
        draw_rect_buf(zbar_x + zoom_bar_w - 1, zbar_y, 1, zbar_h, 0x404040);
        
        int track_x = zbar_x + zoom_bar_w / 2;
        draw_rect_buf(track_x, zbar_y + 20, 2, zbar_h - 40, 0x101010); /* Track Darker */
        int handle_h = 10;
        int range = zbar_h - 40 - handle_h;
        int z_norm = 0;
        if (range > 0) {
            z_norm = ((zoom - 10) * range) / (500 - 10);
            z_norm = range - z_norm; 
        }
        draw_rect_buf(zbar_x + 5, zbar_y + 20 + z_norm, zoom_bar_w - 10, handle_h, 0xFF8800); /* Orange Handle */
        draw_rect_buf(track_x - 4, zbar_y + 10, 10, 2, 0x808080); 
        draw_rect_buf(track_x, zbar_y + 6, 2, 10, 0x808080);
        draw_rect_buf(track_x - 4, zbar_y + zbar_h - 15, 10, 2, 0x808080);
        
        draw_rect_buf(ctx, cty, 1, cth, 0x303030);

        /* Content Draw */
        uint8_t *curr_pixels = img_pixels;
        
        if (is_video) {
             if (vid_playing) {
                 frame_idx++;
                 if (frame_idx >= vid_hdr->frames) frame_idx = 0;
             }
             curr_pixels = vid_frames + (frame_idx * vid_frame_size);
             draw_rect_buf(ctx, cty, ctw, cth, 0x000000); /* BG */
        }
        
        if (curr_pixels) {
            int top_down = is_video ? 1 : (img_h < 0); 
            int abs_h = abs(img_h);
            int row_size = (img_bpp == 3) ? (img_w * 3) : (((img_w * img_bpp * 8 + 31) / 32) * 4);
            if (is_video) row_size = img_w * 3;

            for (int y = 0; y < abs_h; y++) {
                int sc_y_start = (y * zoom) / 100;
                int sc_y_end = ((y + 1) * zoom) / 100;
                int draw_y = cty + pan_y + sc_y_start;
                int p_h = sc_y_end - sc_y_start;
                if (p_h < 1) p_h = 1;

                if (draw_y >= cty + cth) break; 
                if (draw_y + p_h <= cty) continue; 
                
                int vis_y = draw_y;
                int vis_h = p_h;
                if (vis_y < cty) { vis_h -= (cty - vis_y); vis_y = cty; }
                if (vis_y + vis_h > cty + cth) vis_h = (cty + cth) - vis_y;
                if (vis_h <= 0) continue;
                
                int src_y = top_down ? y : (abs_h - 1 - y);
                uint8_t *row = curr_pixels + (src_y * row_size);
                
                for (int x = 0; x < img_w; x++) {
                    int sc_x_start = (x * zoom) / 100;
                    int sc_x_end = ((x + 1) * zoom) / 100;
                    int draw_x = ctx + pan_x + sc_x_start;
                    int p_w = sc_x_end - sc_x_start;
                    if (p_w < 1) p_w = 1;
                    
                    if (draw_x >= ctx + ctw) break;
                    if (draw_x + p_w <= ctx) continue;
                    
                    int vis_x = draw_x;
                    int vis_w = p_w;
                    if (vis_x < ctx) { vis_w -= (ctx - vis_x); vis_x = ctx; }
                    if (vis_x + vis_w > ctx + ctw) vis_w = (ctx + ctw) - vis_x;
                    if (vis_w <= 0) continue;
                    
                    uint8_t *px = &row[x * 3];
                    if (!is_video && img_bpp==4) px = &row[x*4];
                    else if (!is_video && img_bpp==3) px = &row[x*3];
                    
                    uint32_t color = 0;
                    if (is_video) color = (px[0] << 16) | (px[1] << 8) | px[2]; /* Raw RGB -> BGR for FB usually? FB is usually BGR or RGB */
                    /* Assuming FB is BGR (0xRRGGBB in u32 means BB on byte 0?). 
                       Usually framebuffer[i] = 0xRRGGBB. 
                       If u32 is 0xRRGGBB, then written to memory (LE) is BB GG RR. 
                       If raw data is RGB (bytes), we want R->16, G->8, B->0. 
                       My gen_video writes RGB. 0->R.
                       So color = (px[0] << 16) | (px[1] << 8) | px[2].
                    */
                    else {
                        /* BMP BGR */
                         if (img_bpp == 3) color = (px[2] << 16) | (px[1] << 8) | px[0];
                         if (img_bpp == 4) color = (px[3] << 24) | (px[2] << 16) | (px[1] << 8) | px[0];
                    }
                    
                    draw_rect_buf(vis_x, vis_y, vis_w, vis_h, color);
                }
            }
        }
        
        if (is_video && vid_hdr) {
             /* UI Overlay */
             int bar_y = cty + cth - 30;
             draw_rect_buf(ctx, bar_y, ctw, 30, 0x202020); /* Dark Control Bar */
             int prog_w = 0;
             if (vid_hdr->frames > 0) prog_w = (ctw * frame_idx) / vid_hdr->frames;
             draw_rect_buf(ctx, bar_y, prog_w, 30, 0xFF8800); /* Orange Progress */
             
             /* Play/Pause Icon - Center */
             int icx = ctx + ctw/2 - 5;
             int icy = bar_y + 5;
             if (vid_playing) {
                 /* Pause Symbol (Double Bar) */
                  draw_rect_buf(icx, icy, 4, 20, 0xFFFFFF);
                  draw_rect_buf(icx + 6, icy, 4, 20, 0xFFFFFF);
             } else {
                 /* Play Symbol (Triangle-ish block for now, or just Yellow square) */
                 /* Let's make it a solid block for specific color */
                 draw_rect_buf(icx, icy, 10, 20, 0xFFFF00); /* Yellow for Stopped/Paused? No. */
                 /* Use White/Orange. */
                 draw_rect_buf(icx, icy, 10, 20, 0xFFFFFF); 
             }
        }
        
        MouseState ms = mouse_get_state();
        draw_cursor_buf(ms.x, ms.y);
        
        if (ms.scroll_z != last_scroll_z) {
            int diff = ms.scroll_z - last_scroll_z;
            int old_zoom = zoom;
            if (diff > 0 && zoom < 500) zoom += 10 * diff;
            else if (diff < 0 && zoom > 10) zoom += 10 * diff;
            if (zoom < 10) zoom = 10; if (zoom > 500) zoom = 500;
            int cx = ctw / 2; int cy = cth / 2;
            int img_cx = (cx - pan_x) * 100 / old_zoom; int img_cy = (cy - pan_y) * 100 / old_zoom;
            pan_x = cx - (img_cx * zoom / 100); pan_y = cy - (img_cy * zoom / 100);
            last_scroll_z = ms.scroll_z;
        }
        
        if (ms.left_btn) {
           if (!dragging_win && !dragging_pan && !dragging_zoom) {
                 if (ms.x >= win_x + 10 && ms.x < win_x + win_w - 10 && ms.y >= win_y && ms.y < win_y + 30) {
                    dragging_win = 1; drag_ox = ms.x; drag_oy = ms.y; drag_sx = win_x; drag_sy = win_y;
                 }
                 else if (ms.x >= win_x + 10 && ms.x < win_x + 10 + zoom_bar_w && ms.y >= cty && ms.y < cty + cth) {
                     dragging_zoom = 1;
                     int range = cth - 40 - 10;
                     if (range > 0) {
                         int local_y = ms.y - (cty + 20); local_y = range - local_y; 
                         if (local_y < 0) local_y = 0; if (local_y > range) local_y = range;
                         int old_zoom = zoom; zoom = 10 + (local_y * (500 - 10)) / range;
                         int cx = ctw / 2; int cy = cth / 2;
                         int img_cx = (cx - pan_x) * 100 / old_zoom; int img_cy = (cy - pan_y) * 100 / old_zoom;
                         pan_x = cx - (img_cx * zoom / 100); pan_y = cy - (img_cy * zoom / 100);
                     }
                 }
                 else if (is_video && ms.y >= cty + cth - 30) {
                    int cBarY = cty + cth - 30;
                    if (ms.y >= cBarY && ms.y < cBarY + 30 && ms.x >= ctx && ms.x < ctx + ctw) {
                         int local_x = ms.x - ctx; int w = ctw; 
                         if (w > 0 && vid_hdr) frame_idx = (vid_hdr->frames * local_x) / w;
                    }
                 }
                 else if (ms.x >= ctx && ms.x < ctx + ctw && ms.y >= cty && ms.y < cty + cth) {
                    dragging_pan = 1; drag_ox = ms.x; drag_oy = ms.y; drag_sx = pan_x; drag_sy = pan_y;
                 }
            } else {
                if (dragging_win) { win_x = drag_sx + (ms.x - drag_ox); win_y = drag_sy + (ms.y - drag_oy); }
                else if (dragging_pan) { pan_x = drag_sx + (ms.x - drag_ox); pan_y = drag_sy + (ms.y - drag_oy); }
                else if (dragging_zoom) {
                     int range = cth - 40 - 10;
                     if (range > 0) {
                         int local_y = ms.y - (cty + 20); local_y = range - local_y; 
                         if (local_y < 0) local_y = 0; if (local_y > range) local_y = range;
                         int old_zoom = zoom; zoom = 10 + (local_y * (500 - 10)) / range;
                         int cx = ctw / 2; int cy = cth / 2;
                         int img_cx = (cx - pan_x) * 100 / old_zoom; int img_cy = (cy - pan_y) * 100 / old_zoom;
                         pan_x = cx - (img_cx * zoom / 100); pan_y = cy - (img_cy * zoom / 100);
                     }
                }
            }
        } else {
            dragging_win = 0; dragging_pan = 0; dragging_zoom = 0;
        }

        if (keyboard_available()) {
            unsigned char c = (unsigned char)keyboard_getchar();
            if (c == 27) exit_loop = 1;
            else if (c == ' ') { if (is_video) vid_playing = !vid_playing; }
        }
        
        if (screen_pitch == screen_w * 4) {
             memcpy(frontbuffer_addr, framebuffer, screen_w * screen_h * 4);
        } else {
             for (uint64_t y = 0; y < screen_h; y++) {
                 uint32_t *dst = (uint32_t*)((uint8_t*)frontbuffer_addr + y * screen_pitch);
                 uint32_t *src = &framebuffer[y * screen_w];
                 memcpy(dst, src, screen_w * 4);
             }
        }
        
        timer_sleep(16);
    }
    
    if (file_data) kfree(file_data);
    if (framebuffer) kfree(framebuffer);
    term_clear(); 
}

void cmd_beep(char *args) { (void)args; kprint("Beep.\n"); timer_beep(1000, 200); }
void cmd_file(char *filename) { (void)filename; } /* Stub */
