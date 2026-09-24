pub mod keyboard;
pub mod mouse;
pub mod video;
pub mod ata;
pub mod timer;
pub mod rtc;
pub mod serial;
pub mod audio;
pub mod manager;
pub mod power;
pub fn init() {
    audio::ac97::init();
    partition::init();
    drm::init();
}
pub mod iokit;
pub mod pci;
pub mod net;
pub mod mbr;
pub mod partition;
pub mod storage;
pub mod usb;
pub mod klog;
pub mod sovereign_io;
pub mod drm;
pub mod irq;
