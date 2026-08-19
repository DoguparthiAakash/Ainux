#include <stdio.h>
#include <stddef.h>
#include <drm/drm.h>

int main() {
    printf("drm_mode_create_dumb:\n");
    printf("height: %zu, width: %zu, bpp: %zu, flags: %zu, handle: %zu, pitch: %zu, size: %zu\n",
           offsetof(struct drm_mode_create_dumb, height),
           offsetof(struct drm_mode_create_dumb, width),
           offsetof(struct drm_mode_create_dumb, bpp),
           offsetof(struct drm_mode_create_dumb, flags),
           offsetof(struct drm_mode_create_dumb, handle),
           offsetof(struct drm_mode_create_dumb, pitch),
           offsetof(struct drm_mode_create_dumb, size));

    printf("drm_mode_map_dumb:\n");
    printf("handle: %zu, pad: %zu, offset: %zu\n",
           offsetof(struct drm_mode_map_dumb, handle),
           offsetof(struct drm_mode_map_dumb, pad),
           offsetof(struct drm_mode_map_dumb, offset));

    printf("drm_mode_fb_cmd:\n");
    printf("fb_id: %zu, width: %zu, height: %zu, pitch: %zu, bpp: %zu, depth: %zu, handle: %zu\n",
           offsetof(struct drm_mode_fb_cmd, fb_id),
           offsetof(struct drm_mode_fb_cmd, width),
           offsetof(struct drm_mode_fb_cmd, height),
           offsetof(struct drm_mode_fb_cmd, pitch),
           offsetof(struct drm_mode_fb_cmd, bpp),
           offsetof(struct drm_mode_fb_cmd, depth),
           offsetof(struct drm_mode_fb_cmd, handle));

    printf("drm_version:\n");
    printf("version_major: %zu, name_len: %zu, name: %zu\n",
           offsetof(struct drm_version, version_major),
           offsetof(struct drm_version, name_len),
           offsetof(struct drm_version, name));

    return 0;
}
