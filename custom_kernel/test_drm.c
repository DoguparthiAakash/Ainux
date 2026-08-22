#include <stdio.h>
#include <stdint.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <unistd.h>
#include <string.h>

#define DRM_IOCTL_VERSION 0xC0406400
#define DRM_IOCTL_MODE_CREATE_DUMB 0xC02064B2
#define DRM_IOCTL_MODE_MAP_DUMB 0xC01064B3
#define DRM_IOCTL_MODE_DIRTYFB 0xC01864B1

struct drm_version {
    int version_major;
    int version_minor;
    int version_patchlevel;
    size_t name_len;
    char *name;
    size_t date_len;
    char *date;
    size_t desc_len;
    char *desc;
};

struct drm_mode_create_dumb {
    uint32_t height;
    uint32_t width;
    uint32_t bpp;
    uint32_t flags;
    uint32_t handle;
    uint32_t pitch;
    uint64_t size;
};

struct drm_mode_map_dumb {
    uint32_t handle;
    uint32_t pad;
    uint64_t offset;
};

struct drm_mode_fb_dirty_cmd {
    uint32_t fb_id;
    uint32_t flags;
    uint32_t color;
    uint32_t num_clips;
    uint64_t clips_ptr;
};
int main() {
    printf("Starting DRM test...\n");
    int fd = open("/dev/dri/card0", O_RDWR);
    if (fd < 0) {
        perror("open /dev/dri/card0 failed");
        return 1;
    }
    printf("Opened /dev/dri/card0 (fd=%d)\n", fd);

    struct drm_version ver = {0};
    if (ioctl(fd, DRM_IOCTL_VERSION, &ver) < 0) {
        perror("ioctl DRM_IOCTL_VERSION failed");
        return 1;
    }
    printf("DRM Version: %d.%d.%d\n", ver.version_major, ver.version_minor, ver.version_patchlevel);

    struct drm_mode_create_dumb create = {
        .width = 1024,
        .height = 768,
        .bpp = 32
    };
    if (ioctl(fd, DRM_IOCTL_MODE_CREATE_DUMB, &create) < 0) {
        perror("ioctl CREATE_DUMB failed");
        return 1;
    }
    printf("CREATE_DUMB success: handle=%u, pitch=%u, size=%llu\n", create.handle, create.pitch, create.size);

    struct drm_mode_map_dumb map = {
        .handle = create.handle
    };
    if (ioctl(fd, DRM_IOCTL_MODE_MAP_DUMB, &map) < 0) {
        perror("ioctl MAP_DUMB failed");
        return 1;
    }
    printf("MAP_DUMB success: fake offset=0x%llX\n", map.offset);

    void *ptr = mmap(NULL, create.size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, map.offset);
    if (ptr == MAP_FAILED) {
        perror("mmap failed");
        return 1;
    }
    printf("mmap success: ptr=%p\n", ptr);

    // Write some colors!
    unsigned int *fb = (unsigned int *)ptr;
    for (int y = 0; y < 768; y++) {
        for (int x = 0; x < 1024; x++) {
            // Blue gradient
            fb[y * 1024 + x] = 0xFF0000 | ((x % 256) << 8) | (y % 256); 
        }
    }

    struct drm_mode_fb_dirty_cmd dirty = {0};
    if (ioctl(fd, DRM_IOCTL_MODE_DIRTYFB, &dirty) < 0) {
        perror("ioctl DIRTYFB failed");
        // Don't fail the test, as it's not strictly necessary for simple framebuffer test.
    }
    printf("DIRTYFB success (or ignored). Check screen for color!\n");
    sleep(2);

    return 42;
}
