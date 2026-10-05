use alloc::sync::Arc;
use spin::Mutex;
use crate::fs::vfs::{VfsResult, VfsError};
use core::fmt::Write;

pub mod kms;
pub mod nvidia;

// DRM IOCTL Constants
pub const DRM_IOCTL_VERSION: u64 = 0xC0406400;
pub const DRM_IOCTL_GET_CAP: u64 = 0xC010640C;
pub const DRM_IOCTL_MODE_GETRESOURCES: u64 = 0xC04064A0;
pub const DRM_IOCTL_MODE_GETCRTC: u64 = 0xC06864A1;
pub const DRM_IOCTL_MODE_SETCRTC: u64 = 0xC06864A2;
pub const DRM_IOCTL_MODE_GETENCODER: u64 = 0xC01464A6;
pub const DRM_IOCTL_MODE_GETCONNECTOR: u64 = 0xC05064A7;
pub const DRM_IOCTL_MODE_CREATE_DUMB: u64 = 0xC02064B2;
pub const DRM_IOCTL_MODE_MAP_DUMB: u64 = 0xC01064B3;
pub const DRM_IOCTL_MODE_ADDFB: u64 = 0xC01C64AE;
pub const DRM_IOCTL_MODE_DIRTYFB: u64 = 0xC01864B1;

// Core DRM Structs
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmVersion {
    pub version_major: i32,
    pub version_minor: i32,
    pub version_patchlevel: i32,
    pub name_len: usize,
    pub name: u64, // char __user *
    pub date_len: usize,
    pub date: u64, // char __user *
    pub desc_len: usize,
    pub desc: u64, // char __user *
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeCreateDumb {
    pub height: u32,
    pub width: u32,
    pub bpp: u32,
    pub flags: u32,
    pub handle: u32,
    pub pitch: u32,
    pub size: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrmModeMapDumb {
    pub handle: u32,
    pub pad: u32,
    pub offset: u64,
}

pub struct DrmDevice {
    pub name: &'static str,
    pub card_no: usize,
    pub dumb_buffer_addr: u64, // Virtual address for simpledrm kernel copy
    pub dumb_buffer_phys: u64, // Physical address for mmap
    pub dumb_buffer_size: usize,
    pub dumb_buffer_handle: u32,
}

lazy_static::lazy_static! {
    pub static ref CARD0: Arc<Mutex<DrmDevice>> = Arc::new(Mutex::new(DrmDevice {
        name: "ainux-dummy-drm",
        card_no: 0,
        dumb_buffer_addr: 0,
        dumb_buffer_phys: 0,
        dumb_buffer_size: 0,
        dumb_buffer_handle: 1,
    }));
}

pub fn init() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "DRM: Initializing Direct Rendering Manager subsystem...\n");
    kms::init();
    let _ = write!(serial, "DRM: Card0 /dev/dri/card0 stub registered.\n");
}

pub fn handle_ioctl(request: u64, arg: u64) -> VfsResult<u64> {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "DRM: ioctl request 0x{:X}\n", request);

    match request {
        DRM_IOCTL_VERSION => {
            if !crate::mm::user::validate_user_range(arg, core::mem::size_of::<DrmVersion>()) {
                return Err(VfsError::PermissionDenied);
            }
            let ptr = arg as *mut DrmVersion;
            unsafe {
                (*ptr).version_major = 1;
                (*ptr).version_minor = 0;
                (*ptr).version_patchlevel = 0;
                // Leave strings alone for now
            }
            Ok(0)
        },
        DRM_IOCTL_MODE_CREATE_DUMB => {
            if !crate::mm::user::validate_user_range(arg, core::mem::size_of::<DrmModeCreateDumb>()) {
                return Err(VfsError::PermissionDenied);
            }
            let ptr = arg as *mut DrmModeCreateDumb;
            let mut dev = CARD0.lock();
            
            unsafe {
                // For a simple framebuffer, bpp is usually 32
                let width = (*ptr).width;
                let height = (*ptr).height;
                let bpp = (*ptr).bpp;
                let pitch = width * (bpp / 8);
                let size = pitch * height;
                
                // If we don't have a buffer allocated yet, allocate it using the physical memory allocator
                if dev.dumb_buffer_addr == 0 {
                    let num_pages = (size as usize + 4095) / 4096;
                    let mut pmm_lock = crate::mm::pmm::PMM.lock();
                    if let Some(ref mut pmm) = *pmm_lock {
                        if let Some(phys) = pmm.alloc_contiguous(num_pages) {
                            let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
                            let virt = phys + hhdm;
                            // Clear it
                            core::ptr::write_bytes(virt as *mut u8, 0, size as usize);
                            
                            dev.dumb_buffer_addr = virt;
                            dev.dumb_buffer_phys = phys;
                            dev.dumb_buffer_size = size as usize;
                        } else {
                            return Err(VfsError::NoSpace); // ENOMEM
                        }
                    } else {
                        return Err(VfsError::NoSpace); // PMM not ready
                    }
                }
                
                (*ptr).handle = dev.dumb_buffer_handle;
                (*ptr).pitch = pitch;
                (*ptr).size = size as u64;
            }
            let _ = write!(serial, "DRM: CREATE_DUMB succeeded.\n");
            Ok(0)
        },
        DRM_IOCTL_MODE_MAP_DUMB => {
            if !crate::mm::user::validate_user_range(arg, core::mem::size_of::<DrmModeMapDumb>()) {
                return Err(VfsError::PermissionDenied);
            }
            let ptr = arg as *mut DrmModeMapDumb;
            let dev = CARD0.lock();
            
            unsafe {
                if (*ptr).handle == dev.dumb_buffer_handle {
                    // The 'offset' returned here is actually a magic cookie used by mmap.
                    // We'll just return the physical/virtual address or a magic handle to map later.
                    // For now, let's use the actual address as the fake "offset" so mmap knows where it is.
                    // In real Linux DRM, this is a generated fake offset.
                    (*ptr).offset = 0x100000000; // Fake offset handle
                } else {
                    return Err(VfsError::NotFound); // Invalid handle
                }
            }
            let _ = write!(serial, "DRM: MAP_DUMB succeeded.\n");
            Ok(0)
        },
        DRM_IOCTL_MODE_DIRTYFB => {
            // Flush dumb buffer to actual simple framebuffer.
            // The framebuffer physical address is only mapped in the kernel page table,
            // not the user page table. We must temporarily switch to kernel CR3.
            let dev = CARD0.lock();
            if dev.dumb_buffer_addr != 0 {
                let src_addr = dev.dumb_buffer_addr;
                let src_len = dev.dumb_buffer_size / 4;
                drop(dev);

                // Save user CR3, switch to kernel CR3
                let user_cr3: u64;
                let kernel_cr3 = crate::mm::vmm::KERNEL_PML4
                    .load(core::sync::atomic::Ordering::Relaxed) as u64;
                unsafe {
                    core::arch::asm!("mov {}, cr3", out(reg) user_cr3);
                    if user_cr3 != kernel_cr3 {
                        core::arch::asm!("mov cr3, {}", in(reg) kernel_cr3);
                    }
                }

                // Now safe to access the physical framebuffer
                let src = unsafe { core::slice::from_raw_parts(src_addr as *const u32, src_len) };
                crate::drivers::video::copy_buffer(src);
                let _ = write!(serial, "DRM: DIRTYFB -> framebuffer updated.\n");

                // Restore user CR3
                unsafe {
                    if user_cr3 != kernel_cr3 {
                        core::arch::asm!("mov cr3, {}", in(reg) user_cr3);
                    }
                }
            } else {
                drop(dev);
            }
            Ok(0)
        },
        // DRM_IOCTL_GET_CAP: Return capability values so libdrm knows what the device supports.
        // Cap 0x1 = DRM_CAP_DUMB_BUFFER, 0x2 = DRM_CAP_VBLANK_HIGH_CRTC, etc.
        DRM_IOCTL_GET_CAP => {
            // struct drm_get_cap { __u64 capability; __u64 value; }
            if !crate::mm::user::validate_user_range(arg, 16) {
                return Err(VfsError::PermissionDenied);
            }
            let cap_ptr = arg as *mut u64;
            unsafe {
                let cap = *cap_ptr;  // which capability is requested
                let val_ptr = cap_ptr.add(1);
                match cap {
                    1 => *val_ptr = 1, // DRM_CAP_DUMB_BUFFER: supported
                    2 => *val_ptr = 0, // DRM_CAP_VBLANK_HIGH_CRTC: not supported
                    3 => *val_ptr = 4096, // DRM_CAP_DUMB_PREFERRED_DEPTH: 32bpp
                    4 => *val_ptr = 0, // DRM_CAP_DUMB_SHADOW_FB: not needed
                    5 => *val_ptr = 0, // DRM_CAP_PRIME: not supported
                    6 => *val_ptr = 0, // DRM_CAP_TIMESTAMP_MONOTONIC
                    7 => *val_ptr = 0, // DRM_CAP_ASYNC_PAGE_FLIP
                    8 => *val_ptr = 0, // DRM_CAP_CURSOR_WIDTH
                    9 => *val_ptr = 0, // DRM_CAP_CURSOR_HEIGHT
                    _ => *val_ptr = 0, // Unknown caps: return 0 (unsupported)
                }
            }
            let _ = write!(serial, "DRM: GET_CAP ok\n");
            Ok(0)
        },

        // DRM_IOCTL_MODE_GETRESOURCES: Tell libdrm we have 1 FB, 1 CRTC, 1 encoder, 1 connector.
        // Struct layout (simplified from drm_mode_card_res):
        //   u64 fb_id_ptr, u64 crtc_id_ptr, u64 encoder_id_ptr, u64 connector_id_ptr
        //   u32 count_fbs, u32 count_crtcs, u32 count_encoders, u32 count_connectors
        //   u32 min_width, u32 max_width, u32 min_height, u32 max_height
        DRM_IOCTL_MODE_GETRESOURCES => {
            if !crate::mm::user::validate_user_range(arg, 56) {
                return Err(VfsError::PermissionDenied);
            }
            let ptr = arg as *mut u32;
            unsafe {
                // Offsets in u64 units (ptr as *mut u64)
                let p64 = arg as *mut u64;
                let fb_id_ptr   = *p64.add(0); // user ptr to fb id array
                let crtc_id_ptr = *p64.add(1); // user ptr to crtc id array
                let enc_id_ptr  = *p64.add(2); // user ptr to encoder id array
                let con_id_ptr  = *p64.add(3); // user ptr to connector id array

                // Write IDs into user arrays if provided
                if fb_id_ptr != 0 && crate::mm::user::validate_user_range(fb_id_ptr, 4) {
                    *(fb_id_ptr as *mut u32) = 1; // fb id = 1
                }
                if crtc_id_ptr != 0 && crate::mm::user::validate_user_range(crtc_id_ptr, 4) {
                    *(crtc_id_ptr as *mut u32) = 1; // crtc id = 1
                }
                if enc_id_ptr != 0 && crate::mm::user::validate_user_range(enc_id_ptr, 4) {
                    *(enc_id_ptr as *mut u32) = 1; // encoder id = 1
                }
                if con_id_ptr != 0 && crate::mm::user::validate_user_range(con_id_ptr, 4) {
                    *(con_id_ptr as *mut u32) = 1; // connector id = 1
                }

                // Write counts at offset 32 (after four u64 pointers)
                let counts = arg.wrapping_add(32) as *mut u32;
                *counts.add(0) = 1; // count_fbs
                *counts.add(1) = 1; // count_crtcs
                *counts.add(2) = 1; // count_encoders
                *counts.add(3) = 1; // count_connectors

                // Min/max resolution
                let fb_w = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock() as u32;
                let fb_h = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock() as u32;
                *counts.add(4) = 64;  // min_width
                *counts.add(5) = fb_w.max(1920); // max_width
                *counts.add(6) = 64;  // min_height
                *counts.add(7) = fb_h.max(1080); // max_height
            }
            let _ = write!(serial, "DRM: GETRESOURCES ok\n");
            Ok(0)
        },

        // DRM_IOCTL_MODE_GETCRTC: Return CRTC state with the current display mode.
        // drm_mode_crtc { u64 set_connectors_ptr; u32 count_connectors;
        //   u32 crtc_id; u32 fb_id; u32 x; u32 y;
        //   u32 gamma_size; u32 mode_valid; drm_mode_modeinfo mode; }
        DRM_IOCTL_MODE_GETCRTC => {
            if !crate::mm::user::validate_user_range(arg, 120) {
                return Err(VfsError::PermissionDenied);
            }
            let fb_w = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock() as u32;
            let fb_h = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock() as u32;
            unsafe {
                let p32 = arg as *mut u32;
                // Skip set_connectors_ptr (u64) + count_connectors (u32) = 12 bytes = 3 u32s
                *p32.add(2) = 1;         // count_connectors
                *p32.add(3) = 1;         // crtc_id
                *p32.add(4) = 1;         // fb_id
                *p32.add(5) = 0;         // x
                *p32.add(6) = 0;         // y
                *p32.add(7) = 0;         // gamma_size
                *p32.add(8) = 1;         // mode_valid = 1 (CRTC has a valid mode)
                // drm_mode_modeinfo starts at offset 36 bytes = p32+9
                // { u32 clock; u16 hdisplay, hsync_start, hsync_end, htotal, hskew;
                //   u16 vdisplay, vsync_start, vsync_end, vtotal, vscan;
                //   u32 vrefresh; u32 flags; u32 type; char name[32]; }
                let mode = p32.add(9) as *mut u8;
                // clock (u32) at +0
                *(mode as *mut u32) = 40000; // 40 MHz pixel clock (typical 800x600)
                // hdisplay (u16) at +4
                *(mode.add(4) as *mut u16) = fb_w as u16;
                *(mode.add(6) as *mut u16)  = (fb_w + 40) as u16;  // hsync_start
                *(mode.add(8) as *mut u16)  = (fb_w + 128) as u16; // hsync_end
                *(mode.add(10) as *mut u16) = (fb_w + 256) as u16; // htotal
                *(mode.add(12) as *mut u16) = 0u16; // hskew
                // vdisplay (u16) at +14
                *(mode.add(14) as *mut u16) = fb_h as u16;
                *(mode.add(16) as *mut u16) = (fb_h + 1) as u16;  // vsync_start
                *(mode.add(18) as *mut u16) = (fb_h + 4) as u16;  // vsync_end
                *(mode.add(20) as *mut u16) = (fb_h + 28) as u16; // vtotal
                *(mode.add(22) as *mut u16) = 0u16; // vscan
                // vrefresh (u32) at +24
                *(mode.add(24) as *mut u32) = 60; // 60 Hz
                // flags (u32) at +28
                *(mode.add(28) as *mut u32) = 0;
                // type (u32) at +32
                *(mode.add(32) as *mut u32) = 0x48; // DRM_MODE_TYPE_PREFERRED | DRM_MODE_TYPE_DRIVER
                // name[32] at +36 — ASCII "800x600" or similar
                let name = mode.add(36);
                let label = b"ainux-fb\0";
                for (i, &b) in label.iter().enumerate() {
                    *name.add(i) = b;
                }
            }
            let _ = write!(serial, "DRM: GETCRTC ok ({}x{})\n", fb_w, fb_h);
            Ok(0)
        },

        DRM_IOCTL_MODE_SETCRTC => {
            // Accept mode set silently — we use the simple framebuffer, not KMS scanout.
            let _ = write!(serial, "DRM: SETCRTC accepted (simple-fb mode)\n");
            Ok(0)
        },

        // DRM_IOCTL_MODE_GETENCODER: Return encoder info (type NONE, CRTC 1).
        DRM_IOCTL_MODE_GETENCODER => {
            if !crate::mm::user::validate_user_range(arg, 20) {
                return Err(VfsError::PermissionDenied);
            }
            unsafe {
                let p32 = arg as *mut u32;
                *p32.add(0) = 1; // encoder_id
                *p32.add(1) = 2; // encoder_type: DRM_MODE_ENCODER_TMDS (HDMI/DVI-like)
                *p32.add(2) = 1; // crtc_id
                *p32.add(3) = 1; // possible_crtcs bitmask (CRTC 0)
                *p32.add(4) = 0; // possible_clones
            }
            let _ = write!(serial, "DRM: GETENCODER ok\n");
            Ok(0)
        },

        // DRM_IOCTL_MODE_GETCONNECTOR: Report a connected display with one valid mode.
        DRM_IOCTL_MODE_GETCONNECTOR => {
            if !crate::mm::user::validate_user_range(arg, 80) {
                return Err(VfsError::PermissionDenied);
            }
            let fb_w = *crate::drivers::video::FRAMEBUFFER_WIDTH.lock() as u32;
            let fb_h = *crate::drivers::video::FRAMEBUFFER_HEIGHT.lock() as u32;
            unsafe {
                let p32 = arg as *mut u32;
                // drm_mode_get_connector layout:
                //   u64 encoders_ptr, modes_ptr, props_ptr, prop_values_ptr  (32 bytes)
                //   u32 count_modes, count_props, count_encoders
                //   u32 encoder_id, connector_id
                //   u32 connector_type, connector_type_id
                //   u32 connection  (1=connected, 2=disconnected, 3=unknown)
                //   u32 mm_width, mm_height
                //   u32 subpixel
                let p64 = arg as *mut u64;
                let modes_ptr   = *p64.add(1); // user buffer for mode list
                let enc_ptr     = *p64.add(0); // user buffer for encoder id list

                // Fill in one encoder id
                if enc_ptr != 0 && crate::mm::user::validate_user_range(enc_ptr, 4) {
                    *(enc_ptr as *mut u32) = 1;
                }

                // Fill in one mode if the user gave us a modes buffer
                if modes_ptr != 0 && crate::mm::user::validate_user_range(modes_ptr, 68) {
                    let mode = modes_ptr as *mut u8;
                    *(mode as *mut u32)     = 40000;
                    *(mode.add(4) as *mut u16) = fb_w as u16;
                    *(mode.add(6) as *mut u16) = (fb_w + 40) as u16;
                    *(mode.add(8) as *mut u16) = (fb_w + 128) as u16;
                    *(mode.add(10) as *mut u16) = (fb_w + 256) as u16;
                    *(mode.add(12) as *mut u16) = 0u16;
                    *(mode.add(14) as *mut u16) = fb_h as u16;
                    *(mode.add(16) as *mut u16) = (fb_h + 1) as u16;
                    *(mode.add(18) as *mut u16) = (fb_h + 4) as u16;
                    *(mode.add(20) as *mut u16) = (fb_h + 28) as u16;
                    *(mode.add(22) as *mut u16) = 0u16;
                    *(mode.add(24) as *mut u32) = 60;
                    *(mode.add(28) as *mut u32) = 0;
                    *(mode.add(32) as *mut u32) = 0x48;
                    let name = mode.add(36);
                    let label = b"ainux-fb\0";
                    for (i, &b) in label.iter().enumerate() {
                        *name.add(i) = b;
                    }
                }

                // Write scalar fields at fixed offsets (after the 4 u64 pointers = 32 bytes)
                let fields = arg.wrapping_add(32) as *mut u32;
                *fields.add(0) = 1; // count_modes
                *fields.add(1) = 0; // count_props
                *fields.add(2) = 1; // count_encoders
                *fields.add(3) = 1; // encoder_id (current encoder)
                *fields.add(4) = 1; // connector_id
                *fields.add(5) = 11; // connector_type: DRM_MODE_CONNECTOR_VIRTUAL
                *fields.add(6) = 1;  // connector_type_id
                *fields.add(7) = 1;  // connection: DRM_MODE_CONNECTED
                *fields.add(8) = (fb_w * 25 / 96); // mm_width (~96 DPI)
                *fields.add(9) = (fb_h * 25 / 96); // mm_height
                *fields.add(10) = 0; // subpixel
            }
            let _ = write!(serial, "DRM: GETCONNECTOR ok\n");
            Ok(0)
        },

        DRM_IOCTL_MODE_ADDFB => {
            // Accept add-framebuffer and return handle 1.
            if !crate::mm::user::validate_user_range(arg, 32) {
                return Err(VfsError::PermissionDenied);
            }
            unsafe {
                // drm_mode_fb_cmd: u32 fb_id is at offset 28
                let fb_id_ptr = arg.wrapping_add(28) as *mut u32;
                *fb_id_ptr = 1; // Our one and only framebuffer
            }
            let _ = write!(serial, "DRM: ADDFB ok\n");
            Ok(0)
        },

        _ => {
            let _ = write!(serial, "DRM: Unhandled IOCTL 0x{:X}\n", request);
            Ok(0) // Best-effort: lie and say OK for unknown ioctls
        }
    }
}

pub fn handle_mmap(offset: u64, size: usize) -> VfsResult<Option<u64>> {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    use core::fmt::Write;
    let _ = write!(serial, "DRM: handle_mmap called with offset 0x{:X}, size {}\n", offset, size);

    if offset == 0x100000000 {
        let dev = CARD0.lock();
        if dev.dumb_buffer_addr != 0 {
            // Return the physical address so sys_mmap can map it into user space
            return Ok(Some(dev.dumb_buffer_phys));
        }
    }
    Err(VfsError::NotFound)
}
