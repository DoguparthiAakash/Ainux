// =============================================================================
// VirtIO Block Storage Driver (vda)
// PCI Vendor 0x1AF4 Device 0x1001 (Legacy) / 0x1042 (Modern)
// High-performance virtual disk driver for QEMU, KVM, Limbo, and Cloud instances.
// =============================================================================

use core::arch::asm;
use spin::Mutex;
use alloc::string::String;
use alloc::format;
use core::fmt::Write;

// VirtIO PCI Registers (I/O BAR)
const VIRTIO_PCI_HOST_FEATURES:  u16 = 0x00; // 32-bit R
const VIRTIO_PCI_GUEST_FEATURES: u16 = 0x04; // 32-bit W
const VIRTIO_PCI_QUEUE_PFN:      u16 = 0x08; // 32-bit RW
const VIRTIO_PCI_QUEUE_NUM:      u16 = 0x0C; // 16-bit R
const VIRTIO_PCI_QUEUE_SEL:      u16 = 0x0E; // 16-bit W
const VIRTIO_PCI_QUEUE_NOTIFY:   u16 = 0x10; // 16-bit W
const VIRTIO_PCI_STATUS:         u16 = 0x12; // 8-bit RW
const VIRTIO_PCI_ISR:            u16 = 0x13; // 8-bit R
const VIRTIO_PCI_CONFIG_OFF:     u16 = 0x14; // Start of device config

// VirtIO Status Bits
const STATUS_ACKNOWLEDGE: u8 = 1;
const STATUS_DRIVER:      u8 = 2;
const STATUS_DRIVER_OK:   u8 = 4;
const STATUS_FEATURES_OK: u8 = 8;
const STATUS_FAILED:      u8 = 128;

// Block Request Types
const VIRTIO_BLK_T_IN:  u32 = 0; // Read
const VIRTIO_BLK_T_OUT: u32 = 1; // Write

// Virtqueue Descriptor Flags
const VRING_DESC_F_NEXT:  u16 = 1;
const VRING_DESC_F_WRITE: u16 = 2;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct VRingDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct VRingAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; 16],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct VRingUsedElem {
    pub id: u32,
    pub len: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct VRingUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VRingUsedElem; 16],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct VirtioBlkReqHeader {
    pub req_type: u32,
    pub reserved: u32,
    pub sector: u64,
}

#[repr(C, align(4096))]
struct VirtioBlkMemory {
    desc: [VRingDesc; 16],
    avail: VRingAvail,
    used: VRingUsed,
    header: VirtioBlkReqHeader,
    status_byte: u8,
    bounce_buf: [u8; 512],
}

static mut VIRTIO_BLK_MEM: VirtioBlkMemory = VirtioBlkMemory {
    desc: [VRingDesc { addr: 0, len: 0, flags: 0, next: 0 }; 16],
    avail: VRingAvail { flags: 0, idx: 0, ring: [0; 16] },
    used: VRingUsed { flags: 0, idx: 0, ring: [VRingUsedElem { id: 0, len: 0 }; 16] },
    header: VirtioBlkReqHeader { req_type: 0, reserved: 0, sector: 0 },
    status_byte: 0,
    bounce_buf: [0; 512],
};

pub struct VirtioBlkDevice {
    pub io_base: u16,
    pub capacity_sectors: u64,
    pub capacity_mb: u64,
    pub queue_size: u16,
    pub last_used_idx: u16,
    pub initialized: bool,
}

pub static ACTIVE_VIRTIO_BLK: Mutex<Option<VirtioBlkDevice>> = Mutex::new(None);

#[inline]
unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let mut val: u8;
    asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

#[inline]
unsafe fn outw(port: u16, val: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") val, options(nomem, nostack, preserves_flags));
}

#[inline]
unsafe fn inw(port: u16) -> u16 {
    let mut val: u16;
    asm!("in ax, dx", out("ax") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

#[inline]
unsafe fn outl(port: u16, val: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") val, options(nomem, nostack, preserves_flags));
}

#[inline]
unsafe fn inl(port: u16) -> u32 {
    let mut val: u32;
    asm!("in eax, dx", out("eax") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

impl VirtioBlkDevice {
    pub fn new(io_base: u16) -> Self {
        Self {
            io_base,
            capacity_sectors: 0,
            capacity_mb: 0,
            queue_size: 0,
            last_used_idx: 0,
            initialized: false,
        }
    }

    pub fn init(&mut self) -> bool {
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        let _ = write!(serial, "VirtIO-Blk: Initializing at I/O Port {:#x}...\n", self.io_base);

        unsafe {
            // 1. Reset device
            outb(self.io_base + VIRTIO_PCI_STATUS, 0);

            // 2. Acknowledge device
            outb(self.io_base + VIRTIO_PCI_STATUS, STATUS_ACKNOWLEDGE);

            // 3. Set DRIVER status bit
            outb(self.io_base + VIRTIO_PCI_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER);

            // 4. Read Capacity from device-specific config (offset 0x14, 64 bits)
            let cap_low = inl(self.io_base + VIRTIO_PCI_CONFIG_OFF) as u64;
            let cap_high = inl(self.io_base + VIRTIO_PCI_CONFIG_OFF + 4) as u64;
            self.capacity_sectors = (cap_high << 32) | cap_low;
            self.capacity_mb = (self.capacity_sectors * 512) / (1024 * 1024);

            // 5. Select queue 0 (Request queue)
            outw(self.io_base + VIRTIO_PCI_QUEUE_SEL, 0);
            self.queue_size = inw(self.io_base + VIRTIO_PCI_QUEUE_NUM);

            if self.queue_size == 0 {
                let _ = write!(serial, "VirtIO-Blk: Queue 0 not available!\n");
                return false;
            }

            // 6. Program Queue PFN
            let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
            let mem_phys = (&VIRTIO_BLK_MEM as *const _ as u64).wrapping_sub(hhdm);
            let pfn = (mem_phys >> 12) as u32;

            outl(self.io_base + VIRTIO_PCI_QUEUE_PFN, pfn);

            // 7. Set DRIVER_OK
            outb(
                self.io_base + VIRTIO_PCI_STATUS,
                STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_DRIVER_OK
            );

            let _ = write!(
                serial,
                "VirtIO-Blk: Online | Disk Size: {} MB ({} Sectors) | Queue Size: {}\n",
                self.capacity_mb, self.capacity_sectors, self.queue_size
            );
        }

        self.initialized = true;
        true
    }

    pub fn read_sector(&mut self, sector: u64, buf: &mut [u8; 512]) -> bool {
        unsafe {
            let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
            let mem = &mut VIRTIO_BLK_MEM;

            // Setup Request Header
            mem.header.req_type = VIRTIO_BLK_T_IN;
            mem.header.reserved = 0;
            mem.header.sector = sector;
            mem.status_byte = 0xFF;

            let hdr_phys = (&mem.header as *const _ as u64).wrapping_sub(hhdm);
            let buf_phys = (mem.bounce_buf.as_ptr() as u64).wrapping_sub(hhdm);
            let status_phys = (&mem.status_byte as *const _ as u64).wrapping_sub(hhdm);

            // Descriptor 0: Request Header (Device readable)
            mem.desc[0].addr = hdr_phys;
            mem.desc[0].len = core::mem::size_of::<VirtioBlkReqHeader>() as u32;
            mem.desc[0].flags = VRING_DESC_F_NEXT;
            mem.desc[0].next = 1;

            // Descriptor 1: Data Buffer (Device writable)
            mem.desc[1].addr = buf_phys;
            mem.desc[1].len = 512;
            mem.desc[1].flags = VRING_DESC_F_NEXT | VRING_DESC_F_WRITE;
            mem.desc[1].next = 2;

            // Descriptor 2: Status byte (Device writable)
            mem.desc[2].addr = status_phys;
            mem.desc[2].len = 1;
            mem.desc[2].flags = VRING_DESC_F_WRITE;
            mem.desc[2].next = 0;

            // Put Head descriptor into Available Ring
            let avail_idx = mem.avail.idx;
            mem.avail.ring[(avail_idx as usize) % 16] = 0;
            mem.avail.idx = avail_idx.wrapping_add(1);

            // Notify device of Queue 0
            outw(self.io_base + VIRTIO_PCI_QUEUE_NOTIFY, 0);

            // Poll Used Ring for completion
            let expected_idx = self.last_used_idx.wrapping_add(1);
            for _ in 0..100_000 {
                if mem.used.idx == expected_idx {
                    self.last_used_idx = expected_idx;
                    if mem.status_byte == 0 {
                        buf.copy_from_slice(&mem.bounce_buf);
                        return true;
                    }
                    return false;
                }
                core::hint::spin_loop();
            }
        }
        false
    }
}

pub fn detect_and_init(bus: u8, slot: u8, func: u8, bar0_raw: u32) -> bool {
    let io_base = (bar0_raw & 0xFFFC) as u16;
    let mut dev = VirtioBlkDevice::new(io_base);
    if dev.init() {
        *ACTIVE_VIRTIO_BLK.lock() = Some(dev);
        true
    } else {
        false
    }
}

pub fn is_present() -> bool {
    ACTIVE_VIRTIO_BLK.lock().is_some()
}

pub fn get_status() -> String {
    let guard = ACTIVE_VIRTIO_BLK.lock();
    if let Some(dev) = guard.as_ref() {
        format!(
            "VirtIO Block Device (/dev/vda):\n  Interface   : VirtIO PCI Virtual Disk (I/O {:#x})\n  Capacity    : {} MB ({} Sectors)\n  Queue Size  : {} Descriptors\n  Status      : Online",
            dev.io_base, dev.capacity_mb, dev.capacity_sectors, dev.queue_size
        )
    } else {
        format!("VirtIO-Blk: No VirtIO virtual disk detected.")
    }
}

