use alloc::sync::Arc;
use spin::Mutex;
use crate::fs::vfs::{VfsResult, VfsError};

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
    pub dumb_buffer_addr: u64, // Single dumb buffer for simpledrm
    pub dumb_buffer_size: usize,
    pub dumb_buffer_handle: u32,
}

lazy_static::lazy_static! {
    pub static ref CARD0: Arc<Mutex<DrmDevice>> = Arc::new(Mutex::new(DrmDevice {
        name: "ainux-dummy-drm",
        card_no: 0,
        dumb_buffer_addr: 0,
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
            if !crate::mm::user::validate_user_ptr(arg, core::mem::size_of::<DrmVersion>()) {
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
            if !crate::mm::user::validate_user_ptr(arg, core::mem::size_of::<DrmModeCreateDumb>()) {
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
                
                // If we don't have a buffer allocated yet, allocate it using the global heap
                if dev.dumb_buffer_addr == 0 {
                    // Quick and dirty allocation using the kernel allocator
                    use alloc::alloc::{alloc, Layout};
                    let layout = Layout::from_size_align(size as usize, 4096).unwrap();
                    let mem = alloc(layout);
                    // Clear it
                    core::ptr::write_bytes(mem, 0, size as usize);
                    
                    dev.dumb_buffer_addr = mem as u64;
                    dev.dumb_buffer_size = size as usize;
                }
                
                (*ptr).handle = dev.dumb_buffer_handle;
                (*ptr).pitch = pitch;
                (*ptr).size = size as u64;
            }
            let _ = write!(serial, "DRM: CREATE_DUMB succeeded.\n");
            Ok(0)
        },
        DRM_IOCTL_MODE_MAP_DUMB => {
            if !crate::mm::user::validate_user_ptr(arg, core::mem::size_of::<DrmModeMapDumb>()) {
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
        // Mocks for getters so Wayland doesn't crash immediately
        DRM_IOCTL_GET_CAP => { Ok(0) },
        DRM_IOCTL_MODE_GETRESOURCES => { Ok(0) },
        DRM_IOCTL_MODE_GETCRTC => { Ok(0) },
        DRM_IOCTL_MODE_SETCRTC => { Ok(0) },
        DRM_IOCTL_MODE_GETENCODER => { Ok(0) },
        DRM_IOCTL_MODE_GETCONNECTOR => { Ok(0) },
        DRM_IOCTL_MODE_ADDFB => { Ok(0) },
        _ => {
            let _ = write!(serial, "DRM: Unhandled IOCTL 0x{:X}\n", request);
            Ok(0) // Lie and say OK for unknown
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
            // Translate the kernel virtual address to a physical address so the VMM can map it into user space
            // In our higher-half kernel, the physical address is usually VA - 0xFFFFFFFF80000000
            // BUT alloc::alloc returns heap addresses which are in the range 0xFFFF...
            // Wait, we need to map the exact physical pages.
            // For now, return the Virtual Address and let sys_mmap handle mapping the underlying physical frames!
            return Ok(Some(dev.dumb_buffer_addr));
        }
    }
    Err(VfsError::NotFound)
}
