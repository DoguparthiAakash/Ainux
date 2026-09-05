use crate::drivers::pci;
use crate::drivers::serial;
use core::fmt::Write;
use spin::Mutex;
use alloc::vec::Vec;

pub const INTEL_VEND: u16 = 0x8086;
pub const E1000_DEV: u16 = 0x100E;
pub const E1000_I217: u16 = 0x153A;
pub const E1000_82577LM: u16 = 0x10EA;

// Base MMIO Register Offsets
const REG_CTRL: usize = 0x0000;
const REG_STATUS: usize = 0x0008;
const REG_EEPROM: usize = 0x0014;
const REG_CTRL_EXT: usize = 0x0018;
const REG_IMASK: usize = 0x00D0;
const REG_IMC: usize = 0x00D8;
const REG_RCTRL: usize = 0x0100;
const REG_RXDESCLO: usize = 0x2800;
const REG_RXDESCHI: usize = 0x2804;
const REG_RXDESCLEN: usize = 0x2808;
const REG_RXDESCHEAD: usize = 0x2810;
const REG_RXDESCTAIL: usize = 0x2818;
const REG_TCTRL: usize = 0x0400;
const REG_TXDESCLO: usize = 0x3800;
const REG_TXDESCHI: usize = 0x3804;
const REG_TXDESCLEN: usize = 0x3808;
const REG_TXDESCHEAD: usize = 0x3810;
const REG_TXDESCTAIL: usize = 0x3818;

const NUM_RX_DESC: usize = 32;
const NUM_TX_DESC: usize = 8;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct E1000RxDesc {
    addr: u64,
    length: u16,
    checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct E1000TxDesc {
    addr: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u16,
}

static mut RX_DESCRIPTORS: [E1000RxDesc; NUM_RX_DESC] = [E1000RxDesc { addr: 0, length: 0, checksum: 0, status: 0, errors: 0, special: 0 }; NUM_RX_DESC];
static mut TX_DESCRIPTORS: [E1000TxDesc; NUM_TX_DESC] = [E1000TxDesc { addr: 0, length: 0, cso: 0, cmd: 0, status: 0, css: 0, special: 0 }; NUM_TX_DESC];

static mut RX_BUFFERS: [[u8; 2048]; NUM_RX_DESC] = [[0; 2048]; NUM_RX_DESC];
static mut TX_BUFFERS: [[u8; 2048]; NUM_TX_DESC] = [[0; 2048]; NUM_TX_DESC];

static mut RX_CUR: usize = 0;
static mut TX_CUR: usize = 0;

pub struct E1000Driver {
    pub mem_base: usize,
    pub mac_address: [u8; 6],
    pub initialized: bool,
}

impl E1000Driver {
    pub const fn new() -> Self {
        E1000Driver {
            mem_base: 0,
            mac_address: [0; 6],
            initialized: false,
        }
    }

    pub fn write_reg(&self, offset: usize, value: u32) {
        unsafe {
            core::ptr::write_volatile((self.mem_base + offset) as *mut u32, value);
        }
    }

    pub fn read_reg(&self, offset: usize) -> u32 {
        unsafe {
            core::ptr::read_volatile((self.mem_base + offset) as *const u32)
        }
    }

    pub fn init(&mut self, bus: u8, slot: u8, func: u8) -> bool {
        let mut serial = serial::SerialPort::new(0x3F8);
        
        let bar0 = pci::pci_config_read(bus, slot, func, 0x10);
        if bar0 & 1 == 1 {
            let _ = write!(serial, "E1000: BAR0 is I/O Space, we require Memory Mapped IO.\n");
            return false;
        }

        self.mem_base = (bar0 & 0xFFFFFFF0) as usize;
        let _ = write!(serial, "E1000: Initializing at MMIO base {:#x}\n", self.mem_base);

        // Enable Bus Mastering
        let cmd = pci::pci_config_read(bus, slot, func, 0x04);
        pci::pci_config_write(bus, slot, func, 0x04, cmd | (1 << 2));

        // Read MAC Address from EEPROM
        let mut mac = [0u8; 6];
        if self.read_eeprom(0, &mut mac[0..2]) &&
           self.read_eeprom(1, &mut mac[2..4]) &&
           self.read_eeprom(2, &mut mac[4..6]) {
            self.mac_address = mac;
            let _ = write!(serial, "E1000: MAC Address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}\n",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
        } else {
            let _ = write!(serial, "E1000: Failed to read MAC from EEPROM.\n");
            return false;
        }

        // Disable all interrupts
        self.write_reg(REG_IMC, 0xFFFFFFFF);

        // Initialize Link
        let mut ctrl = self.read_reg(REG_CTRL);
        ctrl |= 1 << 6; // SLU (Set Link Up)
        ctrl &= !(1 << 3); // Unset LRST (Link Reset)
        self.write_reg(REG_CTRL, ctrl);

        self.init_rx();
        self.init_tx();

        self.initialized = true;
        let _ = write!(serial, "E1000: Driver initialized with Rings.\n");
        true
    }

    fn init_rx(&mut self) {
        unsafe {
            let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
            let desc_phys = RX_DESCRIPTORS.as_ptr() as u64 - hhdm;
            
            for i in 0..NUM_RX_DESC {
                RX_DESCRIPTORS[i].addr = RX_BUFFERS[i].as_ptr() as u64 - hhdm;
                RX_DESCRIPTORS[i].status = 0;
            }

            self.write_reg(REG_RXDESCLO, (desc_phys & 0xFFFFFFFF) as u32);
            self.write_reg(REG_RXDESCHI, (desc_phys >> 32) as u32);
            self.write_reg(REG_RXDESCLEN, (NUM_RX_DESC * 16) as u32);
            self.write_reg(REG_RXDESCHEAD, 0);
            self.write_reg(REG_RXDESCTAIL, (NUM_RX_DESC - 1) as u32);

            // RCTRL: EN (1), BAM (15), BSIZE=2048 (0)
            self.write_reg(REG_RCTRL, (1 << 1) | (1 << 15));
        }
    }

    fn init_tx(&mut self) {
        unsafe {
            let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
            let desc_phys = TX_DESCRIPTORS.as_ptr() as u64 - hhdm;
            
            for i in 0..NUM_TX_DESC {
                TX_DESCRIPTORS[i].addr = TX_BUFFERS[i].as_ptr() as u64 - hhdm;
                TX_DESCRIPTORS[i].cmd = 0;
            }

            self.write_reg(REG_TXDESCLO, (desc_phys & 0xFFFFFFFF) as u32);
            self.write_reg(REG_TXDESCHI, (desc_phys >> 32) as u32);
            self.write_reg(REG_TXDESCLEN, (NUM_TX_DESC * 16) as u32);
            self.write_reg(REG_TXDESCHEAD, 0);
            self.write_reg(REG_TXDESCTAIL, 0);

            // TCTRL: EN (1), PSP (3)
            self.write_reg(REG_TCTRL, (1 << 1) | (1 << 3));
        }
    }

    fn read_eeprom(&self, offset: u8, out: &mut [u8]) -> bool {
        self.write_reg(REG_EEPROM, 1 | ((offset as u32) << 8));
        for _ in 0..10000 {
            let tmp = self.read_reg(REG_EEPROM);
            if tmp & (1 << 4) != 0 {
                let data = ((tmp >> 16) & 0xFFFF) as u16;
                out[0] = (data & 0xFF) as u8;
                out[1] = (data >> 8) as u8;
                return true;
            }
        }
        false
    }
}

pub static E1000_DEVICE: Mutex<E1000Driver> = Mutex::new(E1000Driver::new());

pub fn detect_and_init() -> bool {
    for bus in 0..=255 {
        for slot in 0..=31 {
            let vendor = pci::pci_config_read(bus, slot, 0, 0) & 0xFFFF;
            if vendor == INTEL_VEND as u32 {
                let device = (pci::pci_config_read(bus, slot, 0, 0) >> 16) & 0xFFFF;
                if device == E1000_DEV as u32 || device == E1000_I217 as u32 || device == E1000_82577LM as u32 {
                    let mut e1000 = E1000_DEVICE.lock();
                    return e1000.init(bus, slot, 0);
                }
            }
        }
    }
    false
}

pub fn send_packet(data: &[u8]) {
    let mut e1000 = E1000_DEVICE.lock();
    if !e1000.initialized { return; }

    unsafe {
        let cur = TX_CUR;
        let len = if data.len() > 2048 { 2048 } else { data.len() };
        core::ptr::copy_nonoverlapping(data.as_ptr(), TX_BUFFERS[cur].as_mut_ptr(), len);
        
        TX_DESCRIPTORS[cur].length = len as u16;
        // CMD: EOP (0), IFCS (1), RS (3) -> (1 | 2 | 8) = 0xB
        TX_DESCRIPTORS[cur].cmd = 0xB;
        TX_DESCRIPTORS[cur].status = 0;

        let next = (cur + 1) % NUM_TX_DESC;
        TX_CUR = next;
        e1000.write_reg(REG_TXDESCTAIL, next as u32);
    }
}

pub fn receive_packet<F>(mut handler: F) where F: FnMut(&[u8]) {
    let mut e1000 = E1000_DEVICE.lock();
    if !e1000.initialized { return; }

    unsafe {
        let mut cur = RX_CUR;
        if (RX_DESCRIPTORS[cur].status & 0x1) != 0 {
            let len = RX_DESCRIPTORS[cur].length as usize;
            let packet = core::slice::from_raw_parts(RX_BUFFERS[cur].as_ptr(), len);
            handler(packet);

            RX_DESCRIPTORS[cur].status = 0;
            let next = (cur + 1) % NUM_RX_DESC;
            e1000.write_reg(REG_RXDESCTAIL, cur as u32);
            RX_CUR = next;
        }
    }
}
