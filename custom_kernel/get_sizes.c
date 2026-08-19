#include <stdio.h>
#include <drm/drm.h>

int main() {
    printf("sizeof(struct drm_version): %zu\n", sizeof(struct drm_version));
    printf("sizeof(struct drm_get_cap): %zu\n", sizeof(struct drm_get_cap));
    printf("sizeof(struct drm_mode_card_res): %zu\n", sizeof(struct drm_mode_card_res));
    printf("sizeof(struct drm_mode_crtc): %zu\n", sizeof(struct drm_mode_crtc));
    printf("sizeof(struct drm_mode_get_encoder): %zu\n", sizeof(struct drm_mode_get_encoder));
    printf("sizeof(struct drm_mode_get_connector): %zu\n", sizeof(struct drm_mode_get_connector));
    printf("sizeof(struct drm_mode_create_dumb): %zu\n", sizeof(struct drm_mode_create_dumb));
    printf("sizeof(struct drm_mode_map_dumb): %zu\n", sizeof(struct drm_mode_map_dumb));
    printf("sizeof(struct drm_mode_fb_cmd): %zu\n", sizeof(struct drm_mode_fb_cmd));
    printf("sizeof(struct drm_mode_fb_dirty_cmd): %zu\n", sizeof(struct drm_mode_fb_dirty_cmd));
    return 0;
}
