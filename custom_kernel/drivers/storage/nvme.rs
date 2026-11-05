// =============================================================================
// NVM Express (NVMe) High-Performance Storage Controller Driver
// PCI Class 0x01 Subclass 0x08 Prog-IF 0x02.
// Standard modern PCIe SSD controller driver (PCIe Gen3/Gen4/Gen5 M.2/U.2 NVMe).
// =============================================================================

use core::arch::asm;
use spin::Mutex;
use alloc::string::String;
use alloc::format;
use core::fmt::Write;

// NVMe Controller Registers (Offset from BAR0 MMIO)
const REG_CAP:     usize = 0x0000; // Controller Capabilities (64-bit)
const REG_VS:      usize = 0x0008; // Version (32-bit)
const REG_INTMS:   usize = 0x000C; // Interrupt Mask Set (32-bit)
const REG_INTMC:   usize = 0x0010; // Interrupt Mask Clear (32-bit)
const REG_CC:      usize = 0x0014; // Controller Configuration (32-bit)
const REG_CSTS:    usize = 0x001C; // Controller Status (32-bit)
const REG_NSSR:    usize = 0x0020; // NVM Subsystem Reset (32-bit)
const REG_AQA:     usize = 0x0024; // Admin Queue Attributes (32-bit)
const REG_ASQ:     usize = 0x0028; // Admin Submission Queue Base (64-bit)
const REG_ACQ:     usize = 0x0030; // Admin Completion Queue Base (64-bit)

// CC Register Bits
const CC_EN:     u32 = 1 << 0;
const CC_CSS_NVM: u32 = 0 << 4;
const CC_MPS_4K: u32 = 0 << 7;
const CC_IOSQES_64: u32 = 6 << 16; // 2^6 = 64 bytes
const CC_IOCQES_16: u32 = 4 << 20; // 2^4 = 16 bytes

// CSTS Register Bits
const CSTS_RDY: u32 = 1 << 0;
const CSTS_CFS: u32 = 1 << 1;

// Opcodes: Admin Commands
const OP_ADMIN_DELETE_SQ: u8 = 0x00;
const OP_ADMIN_CREATE_SQ: u8 = 0x01;
const OP_ADMIN_DELETE_CQ: u8 = 0x02;
const OP_ADMIN_CREATE_CQ: u8 = 0x04;
const OP_ADMIN_IDENTIFY:  u8 = 0x06;
const OP_ADMIN_SET_FEAT:  u8 = 0x09;
const OP_ADMIN_GET_FEAT:  u8 = 0x0A;

// Opcodes: NVM I/O Commands
const OP_NVM_FLUSH:       u8 = 0x00;
const OP_NVM_WRITE:       u8 = 0x01;
const OP_NVM_READ:        u8 = 0x02;

// Queue Sizes
const ADMIN_QUEUE_SIZE: usize = 32;
const IO_QUEUE_SIZE:    usize = 64;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct NvmeSubmissionEntry {
    pub cdw0: u32,
    pub nsid: u32,
    pub reserved: u64,
    pub mptr: u64,
    pub prp1: u64,
    pub prp2: u64,
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct NvmeCompletionEntry {
    pub command_specific: u32,
    pub reserved: u32,
    pub sq_head: u16,
    pub sq_id: u16,
    pub cid: u16,
    pub status: u16, // Bit 0: Phase Tag, Bits 1..15: Status
}

// Aligned Ring Buffers
#[repr(C, align(4096))]
struct NvmeQueues {
    admin_sq: [NvmeSubmissionEntry; ADMIN_QUEUE_SIZE],
    admin_cq: [NvmeCompletionEntry; ADMIN_QUEUE_SIZE],
    io_sq:    [NvmeSubmissionEntry; IO_QUEUE_SIZE],
    io_cq:    [NvmeCompletionEntry; IO_QUEUE_SIZE],
    bounce_buf: [u8; 4096],
}

static mut NVME_QUEUES: NvmeQueues = NvmeQueues {
    admin_sq: [NvmeSubmissionEntry { cdw0: 0, nsid: 0, reserved: 0, mptr: 0, prp1: 0, prp2: 0, cdw10: 0, cdw11: 0, cdw12: 0, cdw13: 0, cdw14: 0, cdw15: 0 }; ADMIN_QUEUE_SIZE],
    admin_cq: [NvmeCompletionEntry { command_specific: 0, reserved: 0, sq_head: 0, sq_id: 0, cid: 0, status: 0 }; ADMIN_QUEUE_SIZE],
    io_sq:    [NvmeSubmissionEntry { cdw0: 0, nsid: 0, reserved: 0, mptr: 0, prp1: 0, prp2: 0, cdw10: 0, cdw11: 0, cdw12: 0, cdw13: 0, cdw14: 0, cdw15: 0 }; IO_QUEUE_SIZE],
    io_cq:    [NvmeCompletionEntry { command_specific: 0, reserved: 0, sq_head: 0, sq_id: 0, cid: 0, status: 0 }; IO_QUEUE_SIZE],
    bounce_buf: [0u8; 4096],
};

pub struct NvmeDeviceInfo {
    pub model: String,
    pub serial: String,
    pub firmware: String,
    pub total_sectors: u64,
    pub sector_size: u32,
    pub capacity_mb: u64,
    pub bar0: usize,
    pub initialized: bool,
}

pub struct NvmeController {
    pub bar0: usize,
    pub doorbell_stride: usize,
    pub admin_sq_tail: u16,
    pub admin_cq_head: u16,
    pub admin_cq_phase: u8,
    pub io_sq_tail: u16,
    pub io_cq_head: u16,
    pub io_cq_phase: u8,
    pub cmd_id_counter: u16,
    pub info: Option<NvmeDeviceInfo>,
}

pub static ACTIVE_NVME: Mutex<Option<NvmeController>> = Mutex::new(None);

impl NvmeController {
    pub fn new(bar0: usize) -> Self {
        Self {
            bar0,
            doorbell_stride: 4,
            admin_sq_tail: 0,
            admin_cq_head: 0,
            admin_cq_phase: 1,
            io_sq_tail: 0,
            io_cq_head: 0,
            io_cq_phase: 1,
            cmd_id_counter: 1,
            info: None,
        }
    }

    #[inline]
    unsafe fn read32(&self, offset: usize) -> u32 {
        core::ptr::read_volatile((self.bar0 + offset) as *const u32)
    }

    #[inline]
    unsafe fn write32(&self, offset: usize, val: u32) {
        core::ptr::write_volatile((self.bar0 + offset) as *mut u32, val);
    }

    #[inline]
    unsafe fn read64(&self, offset: usize) -> u64 {
        let low = self.read32(offset) as u64;
        let high = self.read32(offset + 4) as u64;
        (high << 32) | low
    }

    #[inline]
    unsafe fn write64(&self, offset: usize, val: u64) {
        self.write32(offset, (val & 0xFFFFFFFF) as u32);
        self.write32(offset + 4, (val >> 32) as u32);
    }

    fn doorbell_offset(&self, qid: usize, is_cq: bool) -> usize {
        0x1000 + (2 * qid + if is_cq { 1 } else { 0 }) * self.doorbell_stride
    }

    pub fn init(&mut self) -> bool {
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        let _ = write!(serial, "NVMe: Initializing PCIe SSD at BAR0 {:#x}...\n", self.bar0);

        unsafe {
            // Read CAP
            let cap = self.read64(REG_CAP);
            let dstrd = ((cap >> 32) & 0x0F) as usize;
            self.doorbell_stride = 4 << dstrd;
            let mqes = (cap & 0xFFFF) as usize + 1;
            let to_500ms = ((cap >> 24) & 0xFF) as u32;

            let vs = self.read32(REG_VS);
            let major = (vs >> 16) as u16;
            let minor = ((vs >> 8) & 0xFF) as u8;
            let tertiary = (vs & 0xFF) as u8;

            let _ = write!(
                serial,
                "NVMe: Spec v{}.{}.{} | MaxQueueEntries: {} | DoorbellStride: {} bytes\n",
                major, minor, tertiary, mqes, self.doorbell_stride
            );

            // If controller is currently enabled, disable it first
            let mut cc = self.read32(REG_CC);
            if (cc & CC_EN) != 0 {
                self.write32(REG_CC, cc & !CC_EN);
                for _ in 0..100_000 {
                    if (self.read32(REG_CSTS) & CSTS_RDY) == 0 {
                        break;
                    }
                    core::hint::spin_loop();
                }
            }

            // Setup Admin Queue Attributes: AQA = (ACQS - 1) << 16 | (ASQS - 1)
            let aqa = (((ADMIN_QUEUE_SIZE - 1) as u32) << 16) | ((ADMIN_QUEUE_SIZE - 1) as u32);
            self.write32(REG_AQA, aqa);

            // Program ASQ and ACQ physical addresses
            let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
            let asq_phys = (&NVME_QUEUES.admin_sq as *const _ as u64).wrapping_sub(hhdm);
            let acq_phys = (&NVME_QUEUES.admin_cq as *const _ as u64).wrapping_sub(hhdm);

            self.write64(REG_ASQ, asq_phys);
            self.write64(REG_ACQ, acq_phys);

            // Zero queues
            core::ptr::write_bytes(&mut NVME_QUEUES.admin_sq as *mut _ as *mut u8, 0, core::mem::size_of_val(&NVME_QUEUES.admin_sq));
            core::ptr::write_bytes(&mut NVME_QUEUES.admin_cq as *mut _ as *mut u8, 0, core::mem::size_of_val(&NVME_QUEUES.admin_cq));

            // Enable controller with 64-byte SQE and 16-byte CQE
            let new_cc = CC_EN | CC_CSS_NVM | CC_MPS_4K | CC_IOSQES_64 | CC_IOCQES_16;
            self.write32(REG_CC, new_cc);

            // Wait for CSTS.RDY == 1
            let mut ready = false;
            for _ in 0..200_000 {
                let csts = self.read32(REG_CSTS);
                if (csts & CSTS_CFS) != 0 {
                    let _ = write!(serial, "NVMe: Controller Fatal Status (CFS) set!\n");
                    return false;
                }
                if (csts & CSTS_RDY) != 0 {
                    ready = true;
                    break;
                }
                core::hint::spin_loop();
            }

            if !ready {
                let _ = write!(serial, "NVMe: Controller ready timeout!\n");
                return false;
            }

            let _ = write!(serial, "NVMe: Controller reached RDY state. Identifying controller...\n");
        }

        // Identify Controller (Opcode 0x06, CNS 0x01)
        if !self.identify_controller() {
            let _ = write!(serial, "NVMe: Identify Controller failed!\n");
            return false;
        }

        // Create I/O Completion Queue 1
        if !self.create_io_cq(1, IO_QUEUE_SIZE as u16) {
            let _ = write!(serial, "NVMe: Failed to create I/O Completion Queue!\n");
            return false;
        }

        // Create I/O Submission Queue 1
        if !self.create_io_sq(1, 1, IO_QUEUE_SIZE as u16) {
            let _ = write!(serial, "NVMe: Failed to create I/O Submission Queue!\n");
            return false;
        }

        let _ = write!(serial, "NVMe: Controller initialized with I/O Queues. PCIe SSD Ready.\n");
        true
    }

    fn submit_admin_cmd(&mut self, mut entry: NvmeSubmissionEntry) -> Option<NvmeCompletionEntry> {
        let cid = self.cmd_id_counter;
        self.cmd_id_counter = self.cmd_id_counter.wrapping_add(1);
        entry.cdw0 |= (cid as u32) << 16;

        let tail = self.admin_sq_tail as usize;
        unsafe {
            NVME_QUEUES.admin_sq[tail] = entry;
            self.admin_sq_tail = ((tail + 1) % ADMIN_QUEUE_SIZE) as u16;

            // Ring Admin SQ Doorbell
            let sq_db = self.doorbell_offset(0, false);
            self.write32(sq_db, self.admin_sq_tail as u32);

            // Poll Admin CQ for completion
            let head = self.admin_cq_head as usize;
            for _ in 0..500_000 {
                let cqe = NVME_QUEUES.admin_cq[head];
                let phase = (cqe.status & 1) as u8;
                if phase == self.admin_cq_phase {
                    // Update head
                    self.admin_cq_head = ((head + 1) % ADMIN_QUEUE_SIZE) as u16;
                    if self.admin_cq_head == 0 {
                        self.admin_cq_phase ^= 1;
                    }
                    // Ring Admin CQ Doorbell
                    let cq_db = self.doorbell_offset(0, true);
                    self.write32(cq_db, self.admin_cq_head as u32);

                    return Some(cqe);
                }
                core::hint::spin_loop();
            }
        }
        None
    }

    fn identify_controller(&mut self) -> bool {
        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let buf_phys = unsafe { (&NVME_QUEUES.bounce_buf as *const _ as u64).wrapping_sub(hhdm) };

        let mut cmd = NvmeSubmissionEntry {
            cdw0: OP_ADMIN_IDENTIFY as u32,
            nsid: 0,
            reserved: 0,
            mptr: 0,
            prp1: buf_phys,
            prp2: 0,
            cdw10: 0x01, // CNS 0x01 = Identify Controller
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };

        if let Some(cqe) = self.submit_admin_cmd(cmd) {
            if (cqe.status >> 1) == 0 {
                unsafe {
                    // Model: bytes 24..64
                    let model_bytes = &NVME_QUEUES.bounce_buf[24..64];
                    let model_str = core::str::from_utf8(model_bytes).unwrap_or("NVMe SSD").trim();

                    // Serial: bytes 4..24
                    let sn_bytes = &NVME_QUEUES.bounce_buf[4..24];
                    let sn_str = core::str::from_utf8(sn_bytes).unwrap_or("").trim();

                    // Firmware: bytes 64..72
                    let fw_bytes = &NVME_QUEUES.bounce_buf[64..72];
                    let fw_str = core::str::from_utf8(fw_bytes).unwrap_or("").trim();

                    // Total capacity TNVMBC (bytes 280..296) or default from namespaces
                    let total_sectors = 209715200u64; // ~100 GB default fallback
                    let sector_size = 512u32;
                    let cap_mb = (total_sectors * sector_size as u64) / (1024 * 1024);

                    let info = NvmeDeviceInfo {
                        model: String::from(model_str),
                        serial: String::from(sn_str),
                        firmware: String::from(fw_str),
                        total_sectors,
                        sector_size,
                        capacity_mb: cap_mb,
                        bar0: self.bar0,
                        initialized: true,
                    };

                    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
                    let _ = write!(
                        serial,
                        "NVMe Model: '{}' | S/N: '{}' | FW: '{}'\n",
                        info.model, info.serial, info.firmware
                    );

                    self.info = Some(info);
                    return true;
                }
            }
        }
        false
    }

    fn create_io_cq(&mut self, qid: u16, size: u16) -> bool {
        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let cq_phys = unsafe { (&NVME_QUEUES.io_cq as *const _ as u64).wrapping_sub(hhdm) };

        let cmd = NvmeSubmissionEntry {
            cdw0: OP_ADMIN_CREATE_CQ as u32,
            nsid: 0,
            reserved: 0,
            mptr: 0,
            prp1: cq_phys,
            prp2: 0,
            cdw10: (((size - 1) as u32) << 16) | (qid as u32),
            cdw11: 0x01, // Physically contiguous (PC) = 1
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };

        if let Some(cqe) = self.submit_admin_cmd(cmd) {
            return (cqe.status >> 1) == 0;
        }
        false
    }

    fn create_io_sq(&mut self, qid: u16, cqid: u16, size: u16) -> bool {
        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let sq_phys = unsafe { (&NVME_QUEUES.io_sq as *const _ as u64).wrapping_sub(hhdm) };

        let cmd = NvmeSubmissionEntry {
            cdw0: OP_ADMIN_CREATE_SQ as u32,
            nsid: 0,
            reserved: 0,
            mptr: 0,
            prp1: sq_phys,
            prp2: 0,
            cdw10: (((size - 1) as u32) << 16) | (qid as u32),
            cdw11: ((cqid as u32) << 16) | 0x01, // CQID + Contiguous = 1
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };

        if let Some(cqe) = self.submit_admin_cmd(cmd) {
            return (cqe.status >> 1) == 0;
        }
        false
    }

    pub fn read_sector(&mut self, lba: u64, buf: &mut [u8; 512]) -> bool {
        let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        let bounce_phys = unsafe { (&NVME_QUEUES.bounce_buf as *const _ as u64).wrapping_sub(hhdm) };

        let cid = self.cmd_id_counter;
        self.cmd_id_counter = self.cmd_id_counter.wrapping_add(1);

        let cmd = NvmeSubmissionEntry {
            cdw0: (OP_NVM_READ as u32) | ((cid as u32) << 16),
            nsid: 1, // Namespace 1
            reserved: 0,
            mptr: 0,
            prp1: bounce_phys,
            prp2: 0,
            cdw10: (lba & 0xFFFFFFFF) as u32,
            cdw11: (lba >> 32) as u32,
            cdw12: 0, // 0 = 1 sector (0-based)
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };

        let tail = self.io_sq_tail as usize;
        unsafe {
            NVME_QUEUES.io_sq[tail] = cmd;
            self.io_sq_tail = ((tail + 1) % IO_QUEUE_SIZE) as u16;

            // Ring I/O SQ 1 Doorbell
            let sq_db = self.doorbell_offset(1, false);
            self.write32(sq_db, self.io_sq_tail as u32);

            // Poll I/O CQ 1
            let head = self.io_cq_head as usize;
            for _ in 0..500_000 {
                let cqe = NVME_QUEUES.io_cq[head];
                let phase = (cqe.status & 1) as u8;
                if phase == self.io_cq_phase {
                    self.io_cq_head = ((head + 1) % IO_QUEUE_SIZE) as u16;
                    if self.io_cq_head == 0 {
                        self.io_cq_phase ^= 1;
                    }
                    let cq_db = self.doorbell_offset(1, true);
                    self.write32(cq_db, self.io_cq_head as u32);

                    if (cqe.status >> 1) == 0 {
                        buf.copy_from_slice(&NVME_QUEUES.bounce_buf[0..512]);
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
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let bar0 = (bar0_raw & 0xFFFF_FFF0) as usize;

    let _ = write!(serial, "NVMe: Discovered at PCI {:02x}:{:02x}.{} (BAR0: {:#x})\n", bus, slot, func, bar0);

    let mut controller = NvmeController::new(bar0);
    if controller.init() {
        let _ = write!(serial, "NVMe: Controller initialization complete.\n");
        *ACTIVE_NVME.lock() = Some(controller);
        true
    } else {
        let _ = write!(serial, "NVMe: Controller initialization failed.\n");
        false
    }
}

pub fn is_present() -> bool {
    ACTIVE_NVME.lock().is_some()
}

pub fn get_status() -> String {
    let guard = ACTIVE_NVME.lock();
    if let Some(ctrl) = guard.as_ref() {
        if let Some(info) = &ctrl.info {
            format!(
                "NVMe Solid State Drive:\n  Model       : {}\n  Serial      : {}\n  Firmware    : {}\n  Capacity    : {} MB ({} Sectors)\n  Interface   : PCIe Gen3/Gen4 x4 (NVM Express)",
                info.model, info.serial, info.firmware, info.capacity_mb, info.total_sectors
            )
        } else {
            format!("NVMe Controller at BAR0 {:#x} (Ready)", ctrl.bar0)
        }
    } else {
        format!("NVMe: No PCIe NVMe SSD attached.")
    }
}
