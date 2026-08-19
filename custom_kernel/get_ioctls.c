#include <stdio.h>
#include <sys/ioctl.h>
#include <drm/drm.h>

int main() {
    printf("DRM_IOCTL_VERSION: 0x%lX\n", (unsigned long)DRM_IOCTL_VERSION);
    printf("DRM_IOCTL_GET_CAP: 0x%lX\n", (unsigned long)DRM_IOCTL_GET_CAP);
    printf("DRM_IOCTL_MODE_GETRESOURCES: 0x%lX\n", (unsigned long)DRM_IOCTL_MODE_GETRESOURCES);
    printf("DRM_IOCTL_MODE_GETCRTC: 0x%lX\n", (unsigned long)DRM_IOCTL_MODE_GETCRTC);
    printf("DRM_IOCTL_MODE_SETCRTC: 0x%lX\n", (unsigned long)DRM_IOCTL_MODE_SETCRTC);
    printf("DRM_IOCTL_MODE_GETENCODER: 0x%lX\n", (unsigned long)DRM_IOCTL_MODE_GETENCODER);
    printf("DRM_IOCTL_MODE_GETCONNECTOR: 0x%lX\n", (unsigned long)DRM_IOCTL_MODE_GETCONNECTOR);
    printf("DRM_IOCTL_MODE_CREATE_DUMB: 0x%lX\n", (unsigned long)DRM_IOCTL_MODE_CREATE_DUMB);
    printf("DRM_IOCTL_MODE_MAP_DUMB: 0x%lX\n", (unsigned long)DRM_IOCTL_MODE_MAP_DUMB);
    printf("DRM_IOCTL_MODE_ADDFB: 0x%lX\n", (unsigned long)DRM_IOCTL_MODE_ADDFB);
    printf("DRM_IOCTL_MODE_DIRTYFB: 0x%lX\n", (unsigned long)DRM_IOCTL_MODE_DIRTYFB);
    return 0;
}
